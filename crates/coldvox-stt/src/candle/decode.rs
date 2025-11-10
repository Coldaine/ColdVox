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
///
/// # Token ID Source
///
/// These token IDs are defined in the Whisper tokenizer vocabulary (GPT-2 BPE based):
/// - **50257** (<|endoftext|>): Marks the end of a transcription
/// - **50258** (<|startoftranscript|>): Marks the beginning of a transcription
/// - **50358** (<|translate|>): Indicates translation task (audio to English)
/// - **50359** (<|transcribe|>): Indicates transcription task (audio to same language)
/// - **50363** (<|notimestamps|>): Disables timestamp generation
///
/// These IDs are hard-coded in the Whisper model training and tokenizer configuration.
/// They cannot be changed without retraining the model.
///
/// Reference: openai/whisper repository, assets/multilingual.tiktoken
const SOT_TOKEN: u32 = 50258; // Start of transcript <|startoftranscript|>
const EOT_TOKEN: u32 = 50257; // End of transcript <|endoftext|>
const NO_TIMESTAMPS_TOKEN: u32 = 50363; // <|notimestamps|>
const TRANSLATE_TOKEN: u32 = 50358; // <|translate|>
const TRANSCRIBE_TOKEN: u32 = 50359; // <|transcribe|>

/// Maximum number of tokens to generate
///
/// # Why 448 tokens?
///
/// This limit comes from the Whisper model architecture:
/// 1. Whisper's decoder has a maximum context length of 448 tokens
/// 2. This includes the prompt (SOT, language, task, timestamp tokens)
/// 3. For 30 seconds of audio with timestamps:
///    - Prompt: ~5 tokens
///    - Text: ~200-300 tokens (typical)
///    - Timestamps: ~100-150 tokens
///    - Total: ~400-450 tokens
/// 4. Exceeding this causes attention mask overflow
///
/// If you hit this limit, the model stops generating. In practice, this rarely
/// happens because the model usually generates EOT before 448 tokens.
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
    /// Create a new decoder state with empty cache
    ///
    /// # KV Cache Initialization
    ///
    /// The `m::Cache::new(true)` parameter controls cross-attention caching:
    /// - **true**: Enable caching of cross-attention keys/values
    /// - **false**: Disable caching (slower but uses less memory)
    ///
    /// We use `true` because:
    /// 1. The encoder output is fixed for the entire decoding process
    /// 2. Caching cross-attention K/V saves ~30-40% inference time
    /// 3. Memory cost is acceptable (encoder_dim × sequence_length)
    /// 4. All production Whisper implementations use cross-attention caching
    pub fn new() -> Self {
        Self {
            kv_cache: m::Cache::new(true), // Enable cross-attention caching
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
            //
            // Why manual softmax instead of using a library?
            // 1. Candle's softmax operates on Tensors, we have Vec<f32> here
            // 2. Converting back and forth (Vec -> Tensor -> softmax -> Vec) is slower
            // 3. This manual implementation is simple, correct, and efficient
            // 4. The numerically stable softmax (subtracting max) prevents overflow
            //
            // TODO: Consider using Candle's built-in softmax if performance becomes critical
            let logits_adjusted: Vec<f32> = logits_vec
                .iter()
                .map(|&l| l / temperature)
                .collect();

            // Numerically stable softmax: exp(x - max(x)) / sum(exp(x - max(x)))
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
        // Why dim(1)? - 1?
        // - logits shape: [batch=1, sequence_length, vocab_size]
        // - After squeeze(0): [sequence_length, vocab_size]
        // - dim(1) gives sequence_length
        // - We want the logits for the last generated token: sequence_length - 1
        // - This is confusing because it looks like we're getting dim(1) from a 2D tensor,
        //   but dim(1) here refers to the second dimension (sequence_length)
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
                    // TODO: Calculate avg_logprob from state.logprobs
                    // Should average the log probabilities of tokens in this segment
                    // Useful for filtering low-confidence segments
                    avg_logprob: 0.0,
                    // TODO: Calculate no_speech_prob from initial decoder output
                    // Whisper models have a special <|nospeech|> token (ID varies by model)
                    // The probability of this token at the first decoder step indicates
                    // whether the audio segment contains speech
                    // High no_speech_prob (>0.6) suggests silence or non-speech audio
                    no_speech_prob: 0.0,
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
    fn test_decoder_state_initialization() {
        let state = DecoderState::new();
        assert!(!state.is_done());
        assert_eq!(state.tokens().len(), 0);
        assert_eq!(state.logprobs.len(), 0);
    }

    #[test]
    fn test_decoder_state_max_tokens() {
        let mut state = DecoderState::new();

        // Add tokens up to the limit
        for _ in 0..MAX_TOKENS {
            state.tokens.push(100);
        }

        assert!(state.is_done(), "Should be done when reaching MAX_TOKENS");
    }

    #[test]
    fn test_decoder_state_eot() {
        let mut state = DecoderState::new();
        state.tokens.push(100);
        assert!(!state.is_done());

        state.is_done = true;
        assert!(state.is_done());
    }

    #[test]
    fn test_special_token_constants() {
        // Verify token IDs match Whisper specification
        assert_eq!(SOT_TOKEN, 50258);
        assert_eq!(EOT_TOKEN, 50257);
        assert_eq!(NO_TIMESTAMPS_TOKEN, 50363);
        assert_eq!(TRANSLATE_TOKEN, 50358);
        assert_eq!(TRANSCRIBE_TOKEN, 50359);
    }

    #[test]
    fn test_max_tokens_constant() {
        // Verify MAX_TOKENS matches Whisper architecture
        assert_eq!(MAX_TOKENS, 448);
    }

    // The following tests require actual model and tokenizer instances,
    // which cannot be created without model files. They are documented
    // here for manual/integration testing:
    //
    // #[test]
    // #[ignore] // Requires model files
    // fn test_init_prompt_transcribe() {
    //     // Test prompt initialization for transcription task
    //     let tokenizer = load_test_tokenizer();
    //     let opts = TranscribeOptions {
    //         language: Some("en".to_string()),
    //         task: WhisperTask::Transcribe,
    //         temperature: 0.0,
    //         enable_timestamps: true,
    //     };
    //     let prompt = init_prompt(&tokenizer, &opts);
    //     assert_eq!(prompt[0], SOT_TOKEN);
    //     assert!(prompt.contains(&TRANSCRIBE_TOKEN));
    //     assert!(prompt.contains(&TIMESTAMP_BEGIN));
    // }
    //
    // #[test]
    // #[ignore] // Requires model files
    // fn test_init_prompt_translate() {
    //     // Test prompt initialization for translation task
    //     let tokenizer = load_test_tokenizer();
    //     let opts = TranscribeOptions {
    //         language: None,
    //         task: WhisperTask::Translate,
    //         temperature: 0.0,
    //         enable_timestamps: false,
    //     };
    //     let prompt = init_prompt(&tokenizer, &opts);
    //     assert_eq!(prompt[0], SOT_TOKEN);
    //     assert!(prompt.contains(&TRANSLATE_TOKEN));
    //     assert!(prompt.contains(&NO_TIMESTAMPS_TOKEN));
    // }
    //
    // #[test]
    // #[ignore] // Requires model files
    // fn test_sample_token_greedy() {
    //     // Test greedy sampling (temperature = 0)
    //     let device = Device::Cpu;
    //     let logits = Tensor::from_vec(
    //         vec![0.1, 0.9, 0.3, 0.5], // Token 1 has highest logit
    //         4,
    //         &device,
    //     ).unwrap();
    //
    //     let token = sample_token(&logits, 0.0);
    //     assert_eq!(token, 1, "Greedy sampling should pick token with highest logit");
    // }
    //
    // #[test]
    // #[ignore] // Requires model files
    // fn test_sample_token_temperature() {
    //     // Test temperature sampling with reproducible seed
    //     let device = Device::Cpu;
    //     let logits = Tensor::from_vec(
    //         vec![1.0, 2.0, 3.0, 4.0],
    //         4,
    //         &device,
    //     ).unwrap();
    //
    //     // Temperature sampling should be stochastic but reproducible with same seed
    //     let token = sample_token_with_seed(&logits, 0.8, 42);
    //     assert!(token < 4, "Should sample a valid token ID");
    // }
    //
    // #[test]
    // #[ignore] // Requires model and audio files
    // fn test_full_decode_pipeline() {
    //     // End-to-end test with real model and audio
    //     let model = load_test_model();
    //     let tokenizer = load_test_tokenizer();
    //     let config = load_test_config();
    //     let device = Device::Cpu;
    //     let audio = load_test_audio();
    //
    //     let opts = TranscribeOptions::default();
    //     let mut decoder = Decoder::new(&model, &tokenizer, &config, &device, 42);
    //
    //     let mel = log_mel_spectrogram(&audio, &device).unwrap();
    //     let encoder_output = decoder.encode(&mel).unwrap();
    //     let transcript = decoder.decode(&encoder_output, &opts).unwrap();
    //
    //     assert!(!transcript.segments.is_empty());
    //     assert!(!transcript.segments[0].text.is_empty());
    // }
}
