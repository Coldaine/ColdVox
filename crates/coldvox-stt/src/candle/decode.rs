//! Core decoding logic for Whisper transcription.
//!
//! This module implements the token-by-token decoding loop with key-value caching.
//! The implementation is based on the Candle Whisper examples.

#[cfg(feature = "whisper")]
use anyhow::{Context, Result};
#[cfg(feature = "whisper")]
use candle_core::{Device, Tensor};
#[cfg(feature = "whisper")]
use candle_transformers::models::whisper::{self as m, Config};
#[cfg(feature = "whisper")]
use rand::{distributions::Distribution, SeedableRng};
#[cfg(feature = "whisper")]
use tokenizers::Tokenizer;

use super::timestamps::{token_to_seconds, SegmentBuilder, TIMESTAMP_BEGIN};
use super::types::{Segment, TranscribeOptions, Transcript, WhisperTask};

/// Special token IDs
const SOT_TOKEN: u32 = 50258; // Start of transcript
const EOT_TOKEN: u32 = 50257; // End of transcript
const NO_TIMESTAMPS_TOKEN: u32 = 50363;
const TRANSLATE_TOKEN: u32 = 50358;
const TRANSCRIBE_TOKEN: u32 = 50359;

/// Maximum number of tokens to generate
const MAX_TOKENS: usize = 448;

/// Decoder state with key-value cache
#[cfg(feature = "whisper")]
pub struct DecoderState {
    /// Key-value cache for the decoder
    kv_cache: m::Cache,
    /// Generated tokens so far
    tokens: Vec<u32>,
    /// Log probabilities for each token
    logprobs: Vec<f64>,
    /// Whether decoding is complete
    is_done: bool,
}

#[cfg(feature = "whisper")]
impl DecoderState {
    pub fn new() -> Self {
        Self {
            kv_cache: m::Cache::new(true),
            tokens: Vec::new(),
            logprobs: Vec::new(),
            is_done: false,
        }
    }

    pub fn is_done(&self) -> bool {
        self.is_done || self.tokens.len() >= MAX_TOKENS
    }

    pub fn tokens(&self) -> &[u32] {
        &self.tokens
    }
}

/// Decodes audio features into text tokens
#[cfg(feature = "whisper")]
pub struct Decoder<'a> {
    model: &'a m::Whisper,
    tokenizer: &'a Tokenizer,
    config: &'a Config,
    device: &'a Device,
    rng: rand::rngs::StdRng,
}

#[cfg(feature = "whisper")]
impl<'a> Decoder<'a> {
    pub fn new(
        model: &'a m::Whisper,
        tokenizer: &'a Tokenizer,
        config: &'a Config,
        device: &'a Device,
        seed: u64,
    ) -> Self {
        Self {
            model,
            tokenizer,
            config,
            device,
            rng: rand::rngs::StdRng::seed_from_u64(seed),
        }
    }

    /// Run the encoder on mel spectrogram features
    pub fn encode(&self, mel: &Tensor) -> Result<Tensor> {
        self.model.encoder().forward(mel, true)
    }

    /// Initialize decoder with prompt tokens
    fn init_prompt(&self, opts: &TranscribeOptions) -> Vec<u32> {
        let mut prompt = vec![SOT_TOKEN];

        // Add language token if specified
        if let Some(ref lang) = opts.language {
            if let Ok(Some(token_id)) = self.tokenizer.token_to_id(&format!("<|{}|>", lang)) {
                prompt.push(token_id);
            }
        }

        // Add task token
        match opts.task {
            WhisperTask::Transcribe => prompt.push(TRANSCRIBE_TOKEN),
            WhisperTask::Translate => prompt.push(TRANSLATE_TOKEN),
        }

        // Add timestamp or no-timestamp token
        if opts.enable_timestamps {
            prompt.push(TIMESTAMP_BEGIN); // First timestamp token (0.0s)
        } else {
            prompt.push(NO_TIMESTAMPS_TOKEN);
        }

        prompt
    }

    /// Sample next token from logits
    fn sample_token(&mut self, logits: &Tensor, temperature: f32) -> Result<u32> {
        let logits_vec: Vec<f32> = logits.to_vec1()?;

        if temperature <= 0.0 {
            // Greedy sampling
            let mut best_token = 0;
            let mut best_logit = f32::NEG_INFINITY;

            for (idx, &logit) in logits_vec.iter().enumerate() {
                if logit > best_logit {
                    best_logit = logit;
                    best_token = idx;
                }
            }

            Ok(best_token as u32)
        } else {
            // Temperature sampling
            let logits_adjusted: Vec<f32> = logits_vec
                .iter()
                .map(|&l| l / temperature)
                .collect();

            // Softmax
            let max_logit = logits_adjusted
                .iter()
                .copied()
                .fold(f32::NEG_INFINITY, f32::max);

            let exp_logits: Vec<f32> = logits_adjusted
                .iter()
                .map(|&l| (l - max_logit).exp())
                .collect();

            let sum: f32 = exp_logits.iter().sum();
            let probs: Vec<f32> = exp_logits
                .iter()
                .map(|&e| e / sum)
                .collect();

            // Sample from distribution
            let dist = rand::distributions::WeightedIndex::new(&probs)
                .context("Failed to create weighted distribution")?;

            Ok(dist.sample(&mut self.rng) as u32)
        }
    }

    /// Decode one step
    fn decode_step(
        &mut self,
        state: &mut DecoderState,
        encoder_output: &Tensor,
        temperature: f32,
    ) -> Result<()> {
        // Prepare input tokens (last token or initial prompt)
        let input_tokens = if state.tokens.is_empty() {
            // This shouldn't happen, but handle it gracefully
            vec![SOT_TOKEN]
        } else {
            vec![*state.tokens.last().unwrap()]
        };

        // Create token tensor
        let tokens_tensor = Tensor::new(input_tokens.as_slice(), self.device)?
            .unsqueeze(0)?; // Add batch dimension

        // Run decoder forward pass
        let logits = self.model.decoder().forward(
            &tokens_tensor,
            encoder_output,
            &mut state.kv_cache,
        )?;

        // Get logits for the last position
        let logits = logits.squeeze(0)?.get(logits.dim(1)? - 1)?;

        // Sample next token
        let next_token = self.sample_token(&logits, temperature)?;

        // Add to state
        state.tokens.push(next_token);

        // Check if done
        if next_token == EOT_TOKEN {
            state.is_done = true;
        }

        Ok(())
    }

    /// Decode audio encoder output into text
    pub fn decode(
        &mut self,
        encoder_output: &Tensor,
        opts: &TranscribeOptions,
    ) -> Result<Transcript> {
        // Initialize decoder state
        let mut state = DecoderState::new();

        // Initialize with prompt tokens
        let prompt = self.init_prompt(opts);
        state.tokens = prompt;

        // Decode tokens
        while !state.is_done() {
            self.decode_step(&mut state, encoder_output, opts.temperature)?;
        }

        // Convert tokens to text and segments
        self.tokens_to_transcript(&state, opts)
    }

    /// Convert tokens to transcript with segments
    fn tokens_to_transcript(
        &self,
        state: &DecoderState,
        opts: &TranscribeOptions,
    ) -> Result<Transcript> {
        let tokens = state.tokens();

        if opts.enable_timestamps {
            // Extract segments with timestamps
            let mut segment_builder = SegmentBuilder::new();

            for &token in tokens {
                if let Some(timestamp) = token_to_seconds(token) {
                    segment_builder.add_timestamp(timestamp);
                    // Finalize previous segment when we hit a new timestamp
                    if segment_builder.current_start.is_some() && segment_builder.current_end.is_some() {
                        segment_builder.finalize_segment();
                    }
                } else if token != SOT_TOKEN && token != EOT_TOKEN
                    && token != TRANSCRIBE_TOKEN && token != TRANSLATE_TOKEN
                    && token != NO_TIMESTAMPS_TOKEN
                    && token < TIMESTAMP_BEGIN {
                    // This is a text token
                    if let Some(text) = self.tokenizer.id_to_token(token) {
                        segment_builder.add_text(&text);
                    }
                }
            }

            let raw_segments = segment_builder.segments();

            let segments: Vec<Segment> = raw_segments
                .into_iter()
                .map(|(start, end, text)| Segment {
                    start_seconds: start,
                    end_seconds: end,
                    text,
                    avg_logprob: 0.0, // TODO: Calculate from logprobs
                    no_speech_prob: 0.0, // TODO: Calculate no_speech probability
                })
                .collect();

            Ok(Transcript {
                segments,
                language: opts.language.clone(),
            })
        } else {
            // No timestamps - return full text as one segment
            let text_tokens: Vec<u32> = tokens
                .iter()
                .filter(|&&t| {
                    t != SOT_TOKEN
                        && t != EOT_TOKEN
                        && t != TRANSCRIBE_TOKEN
                        && t != TRANSLATE_TOKEN
                        && t != NO_TIMESTAMPS_TOKEN
                        && t < TIMESTAMP_BEGIN
                })
                .copied()
                .collect();

            let full_text = self.tokenizer
                .decode(&text_tokens, true)
                .map_err(|e| anyhow::anyhow!("Failed to decode tokens: {}", e))?;

            Ok(Transcript {
                segments: vec![Segment::new(0.0, 30.0, full_text)],
                language: opts.language.clone(),
            })
        }
    }
}

#[cfg(test)]
#[cfg(feature = "whisper")]
mod tests {
    use super::*;

    #[test]
    fn test_init_prompt() {
        // This test would require a real tokenizer and model
        // Placeholder for future implementation
    }

    #[test]
    fn test_decoder_state() {
        let mut state = DecoderState::new();
        assert!(!state.is_done());
        assert_eq!(state.tokens().len(), 0);

        state.tokens.push(100);
        assert_eq!(state.tokens().len(), 1);
        assert!(!state.is_done());

        state.is_done = true;
        assert!(state.is_done());
    }
}
