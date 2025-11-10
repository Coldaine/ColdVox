//! Type definitions for Candle-based Whisper engine
//!
//! This module defines the public API types for the WhisperEngine facade.

use std::path::PathBuf;

/// Device configuration for Whisper inference
#[derive(Debug, Clone)]
pub enum WhisperDevice {
    /// CPU inference
    Cpu,
    /// CUDA GPU inference with device ID
    Cuda(usize),
}

/// Task type for Whisper inference
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhisperTask {
    /// Transcribe audio in the same language
    Transcribe,
    /// Translate audio to English
    Translate,
}

/// Configuration for initializing the WhisperEngine
#[derive(Debug, Clone)]
pub struct WhisperEngineInit {
    /// Path to the model file (safetensors or GGUF)
    pub model_path: PathBuf,
    /// Path to the tokenizer configuration
    pub tokenizer_path: PathBuf,
    /// Path to the model configuration JSON
    pub config_path: PathBuf,
    /// Whether the model is quantized (GGUF format)
    pub quantized: bool,
    /// Device to run inference on
    pub device: WhisperDevice,
}

/// Options for transcription
#[derive(Debug, Clone)]
pub struct TranscribeOptions {
    /// Language code (e.g., "en", "es"). None for auto-detection.
    pub language: Option<String>,
    /// Task type (transcribe or translate)
    pub task: WhisperTask,
    /// Temperature for sampling. 0.0 for greedy decoding, > 0.0 for temperature sampling.
    pub temperature: f32,
    /// Whether to generate timestamps
    pub enable_timestamps: bool,
}

impl Default for TranscribeOptions {
    fn default() -> Self {
        Self {
            language: None,
            task: WhisperTask::Transcribe,
            temperature: 0.0, // Greedy by default
            enable_timestamps: true,
        }
    }
}

/// A segment of transcribed text with timing information
#[derive(Debug, Clone)]
pub struct Segment {
    /// Start time in seconds
    pub start_seconds: f64,
    /// End time in seconds
    pub end_seconds: f64,
    /// Transcribed text for this segment
    pub text: String,
    /// Average log probability for this segment
    pub avg_logprob: f64,
    /// No-speech probability for this segment
    pub no_speech_prob: f64,
}

/// Complete transcription result
#[derive(Debug, Clone)]
pub struct Transcript {
    /// Transcribed segments
    pub segments: Vec<Segment>,
    /// Detected or specified language code
    pub language: Option<String>,
    /// Full concatenated text
    pub text: String,
}

impl Transcript {
    /// Create a new transcript from segments
    pub fn new(segments: Vec<Segment>, language: Option<String>) -> Self {
        let text = segments
            .iter()
            .map(|s| s.text.trim())
            .collect::<Vec<_>>()
            .join(" ");

        Self {
            segments,
            language,
            text,
        }
    }
}
