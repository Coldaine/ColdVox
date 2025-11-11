//! Core decoding logic for Whisper transcription
//!
//! This module implements the token-by-token decoding loop for Whisper models,
//! adapted from the Candle Whisper examples.
//!
//! Attribution: This code is derived from the Candle project's Whisper examples
//! (https://github.com/huggingface/candle/tree/main/candle-examples/examples/whisper)

use anyhow::{Context, Result};
use candle_core::{Device, Tensor};
use candle_transformers::models::whisper::{self as m, Config};
use rand::Rng;

use crate::candle::timestamps::{extract_segments, is_timestamp_token};
use crate::candle::types::{Segment, TranscribeOptions, Transcript, WhisperTask};

/// Special tokens used by Whisper
pub struct SpecialTokens {
    pub sot: u32,              // Start of transcript
    pub eot: u32,              // End of transcript
    pub no_timestamps: u32,    // No timestamp token
    pub timestamp_begin: u32,  // First timestamp token
    pub translate: u32,        // Translate task token
    pub transcribe: u32,       // Transcribe task token
    pub no_speech: u32,        // No speech token
}

impl SpecialTokens {
    /// Extract special tokens from tokenizer
    pub fn from_tokenizer(tokenizer: &tokenizers::Tokenizer) -> Result<Self> {
        let get_token = |text: &str| -> Result<u32> {
            tokenizer
                .token_to_id(text)
                .ok_or_else(|| anyhow::anyhow!("Token '{}' not found in tokenizer", text))
        };

        Ok(Self {
            sot: get_token("<|startoftranscript|>")?,
            eot: get_token("<|endoftext|>")?,
            no_timestamps: get_token("<|notimestamps|>")?,
            timestamp_begin: 50364, // Hardcoded as per Whisper spec
            translate: get_token("<|translate|>")?,
            transcribe: get_token("<|transcribe|>")?,
            no_speech: get_token("<|nospeech|>")?,
        })
    }
}

/// Decoder state for iterative decoding
pub struct DecoderState {
    tokens: Vec<u32>,
    logprobs: Vec<f64>,
    no_speech_probs: Vec<f64>,
}

impl DecoderState {
    pub fn new() -> Self {
        Self {
            tokens: Vec::new(),
            logprobs: Vec::new(),
            no_speech_probs: Vec::new(),
        }
    }

    pub fn push_token(&mut self, token: u32, logprob: f64) {
        self.tokens.push(token);
        self.logprobs.push(logprob);
    }

    pub fn push_no_speech_prob(&mut self, prob: f64) {
        self.no_speech_probs.push(prob);
    }
}

/// Main decoder for Whisper model
pub struct WhisperDecoder<'a> {
    model: &'a m::model::Whisper,
    config: &'a Config,
    tokenizer: &'a tokenizers::Tokenizer,
    special_tokens: SpecialTokens,
    device: &'a Device,
}

impl<'a> WhisperDecoder<'a> {
    /// Create a new decoder
    pub fn new(
        model: &'a m::model::Whisper,
        config: &'a Config,
        tokenizer: &'a tokenizers::Tokenizer,
        device: &'a Device,
    ) -> Result<Self> {
        let special_tokens = SpecialTokens::from_tokenizer(tokenizer)?;

        Ok(Self {
            model,
            config,
            tokenizer,
            special_tokens,
            device,
        })
    }

    /// Decode audio mel spectrogram to transcript
    pub fn decode(
        &self,
        mel: &Tensor,
        opts: &TranscribeOptions,
    ) -> Result<Transcript> {
        // Run encoder
        let audio_features = self
            .model
            .encoder()
            .forward(mel, true)
            .context("Encoder forward pass failed")?;

        // Initialize decoder tokens
        let mut tokens = vec![self.special_tokens.sot];

        // Add language token if specified
        if let Some(ref lang) = opts.language {
            let lang_token = self.get_language_token(lang)?;
            tokens.push(lang_token);
        }

        // Add task token
        let task_token = match opts.task {
            WhisperTask::Transcribe => self.special_tokens.transcribe,
            WhisperTask::Translate => self.special_tokens.translate,
        };
        tokens.push(task_token);

        // Add timestamp control token
        if !opts.enable_timestamps {
            tokens.push(self.special_tokens.no_timestamps);
        }

        // Decode loop
        let mut state = DecoderState::new();
        let max_tokens = 448; // Maximum tokens per segment as per Whisper

        let mut kv_cache = None;

        for _ in 0..max_tokens {
            // Prepare decoder input
            let tokens_tensor = Tensor::new(&tokens[..], self.device)
                .context("Failed to create tokens tensor")?
                .unsqueeze(0)
                .context("Failed to unsqueeze tokens")?;

            // Run decoder
            let (logits, new_kv_cache) = self
                .model
                .decoder()
                .forward(&tokens_tensor, &audio_features, kv_cache.as_ref())
                .context("Decoder forward pass failed")?;

            kv_cache = Some(new_kv_cache);

            // Get last token logits
            let logits = logits
                .squeeze(0)
                .context("Failed to squeeze logits")?
                .get(tokens.len() - 1)
                .context("Failed to get last token logits")?;

            // Sample next token
            let (next_token, logprob) = if opts.temperature > 0.0 {
                self.sample_token(&logits, opts.temperature)?
            } else {
                self.greedy_sample(&logits)?
            };

            // Check for no-speech
            if next_token == self.special_tokens.no_speech {
                state.push_no_speech_prob(1.0);
                break;
            }

            // Check for end of transcript
            if next_token == self.special_tokens.eot {
                break;
            }

            // Add token to sequence
            tokens.push(next_token);
            state.push_token(next_token, logprob);

            // Stop if we hit maximum length
            if tokens.len() >= max_tokens {
                break;
            }
        }

        // Extract segments from decoded tokens
        let segments = if opts.enable_timestamps {
            extract_segments(
                &state.tokens,
                self.tokenizer,
                &state.logprobs,
                &state.no_speech_probs,
            )
        } else {
            // No timestamps - create a single segment
            self.tokens_to_segment(&state.tokens, &state.logprobs, &state.no_speech_probs)?
        };

        // Detect language if it wasn't specified
        let detected_language = if opts.language.is_none() {
            self.detect_language_from_tokens(&tokens)
        } else {
            opts.language.clone()
        };

        Ok(Transcript::new(segments, detected_language))
    }

    /// Greedy sampling - select the token with highest probability
    fn greedy_sample(&self, logits: &Tensor) -> Result<(u32, f64)> {
        let logits_vec = logits.to_vec1::<f32>()
            .context("Failed to convert logits to vec")?;

        let max_idx = logits_vec
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(idx, _)| idx)
            .context("Failed to find max logit")?;

        // Apply softmax to get normalized probabilities
        let max_logit = logits_vec.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

        let exp_logits: Vec<f32> = logits_vec
            .iter()
            .map(|&x| (x - max_logit).exp())
            .collect();

        let sum_exp: f32 = exp_logits.iter().sum();
        let prob = exp_logits[max_idx] / sum_exp;
        let logprob = prob.ln() as f64;

        Ok((max_idx as u32, logprob))
    }

    /// Temperature-based sampling
    fn sample_token(&self, logits: &Tensor, temperature: f32) -> Result<(u32, f64)> {
        let logits_vec = logits.to_vec1::<f32>()
            .context("Failed to convert logits to vec")?;

        // Apply temperature
        let scaled_logits: Vec<f32> = logits_vec
            .iter()
            .map(|&x| x / temperature)
            .collect();

        // Compute softmax
        let max_logit = scaled_logits
            .iter()
            .cloned()
            .fold(f32::NEG_INFINITY, f32::max);

        let exp_logits: Vec<f32> = scaled_logits
            .iter()
            .map(|&x| (x - max_logit).exp())
            .collect();

        let sum_exp: f32 = exp_logits.iter().sum();
        let probs: Vec<f32> = exp_logits.iter().map(|&x| x / sum_exp).collect();

        // Sample from distribution
        let mut rng = rand::thread_rng();
        let sample: f32 = rng.gen();

        let mut cumsum = 0.0;
        let mut sampled_idx = 0;

        for (idx, &prob) in probs.iter().enumerate() {
            cumsum += prob;
            if sample < cumsum {
                sampled_idx = idx;
                break;
            }
        }

        let logprob = probs[sampled_idx].ln() as f64;

        Ok((sampled_idx as u32, logprob))
    }

    /// Get language token ID from language code
    fn get_language_token(&self, lang: &str) -> Result<u32> {
        let lang_token = format!("<|{}|>", lang);
        self.tokenizer
            .token_to_id(&lang_token)
            .ok_or_else(|| anyhow::anyhow!("Language '{}' not supported", lang))
    }

    /// Detect language from decoded tokens
    fn detect_language_from_tokens(&self, tokens: &[u32]) -> Option<String> {
        // The language token is typically the second token (after SOT)
        if tokens.len() > 1 {
            let token_id = tokens[1];
            if let Some(token_str) = self.tokenizer.id_to_token(token_id) {
                // Language tokens are in format <|en|>, <|es|>, etc.
                if token_str.starts_with("<|") && token_str.ends_with("|>") {
                    let lang = &token_str[2..token_str.len() - 2];
                    return Some(lang.to_string());
                }
            }
        }
        None
    }

    /// Convert tokens to a single segment (for non-timestamp mode)
    fn tokens_to_segment(
        &self,
        tokens: &[u32],
        logprobs: &[f64],
        no_speech_probs: &[f64],
    ) -> Result<Vec<Segment>> {
        let text = self
            .tokenizer
            .decode(tokens, true)
            .context("Failed to decode tokens")?
            .trim()
            .to_string();

        if text.is_empty() {
            return Ok(Vec::new());
        }

        let avg_logprob = if logprobs.is_empty() {
            0.0
        } else {
            logprobs.iter().sum::<f64>() / logprobs.len() as f64
        };

        let no_speech_prob = no_speech_probs.first().copied().unwrap_or(0.0);

        Ok(vec![Segment {
            start_seconds: 0.0,
            end_seconds: 30.0, // Default to full 30s window
            text,
            avg_logprob,
            no_speech_prob,
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decoder_state() {
        let mut state = DecoderState::new();
        state.push_token(100, -0.5);
        state.push_token(101, -0.3);

        assert_eq!(state.tokens.len(), 2);
        assert_eq!(state.logprobs.len(), 2);
    }
}
