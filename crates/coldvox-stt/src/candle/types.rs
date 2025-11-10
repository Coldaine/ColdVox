//! Type definitions for the Candle Whisper engine.
//!
//! This module defines the core data structures used by the Whisper engine,
//! including configuration, transcription results, and device selection.

use std::path::PathBuf;

/// Configuration for initializing the Whisper engine.
#[derive(Debug, Clone)]
pub struct WhisperEngineInit {
    /// Path to the model weights file (safetensors or gguf format)
    pub model_path: PathBuf,
    /// Path to the tokenizer configuration directory or file
    pub tokenizer_path: PathBuf,
    /// Path to the model configuration JSON file
    pub config_path: PathBuf,
    /// Whether the model is quantized (GGUF format)
    pub quantized: bool,
    /// Device to run inference on
    pub device: WhisperDevice,
}

/// Device selection for Whisper inference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WhisperDevice {
    /// Use CPU for inference
    Cpu,
    /// Use CUDA GPU with the specified device index
    Cuda(usize),
    /// Use Metal (Apple Silicon)
    Metal,
}

impl WhisperDevice {
    /// Convert device to Candle device
    #[cfg(feature = "whisper")]
    pub(crate) fn to_candle_device(&self) -> anyhow::Result<candle_core::Device> {
        match self {
            WhisperDevice::Cpu => Ok(candle_core::Device::Cpu),
            WhisperDevice::Cuda(idx) => Ok(candle_core::Device::new_cuda(*idx)?),
            WhisperDevice::Metal => Ok(candle_core::Device::new_metal(0)?),
        }
    }
}

/// Task type for Whisper inference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhisperTask {
    /// Transcribe audio in its original language
    Transcribe,
    /// Translate audio to English
    Translate,
}

/// Options for transcription.
#[derive(Debug, Clone)]
pub struct TranscribeOptions {
    /// Language code (e.g., "en", "es"). None for auto-detection.
    pub language: Option<String>,
    /// Task to perform (transcribe or translate)
    pub task: WhisperTask,
    /// Sampling temperature (0.0 for greedy, > 0.0 for sampling)
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

/// Complete transcription result.
#[derive(Debug, Clone)]
pub struct Transcript {
    /// Transcribed text segments
    pub segments: Vec<Segment>,
    /// Detected language (if auto-detected)
    pub language: Option<String>,
}

/// A single transcription segment with timing and metadata.
#[derive(Debug, Clone)]
pub struct Segment {
    /// Start time in seconds
    pub start_seconds: f64,
    /// End time in seconds
    pub end_seconds: f64,
    /// Transcribed text
    pub text: String,
    /// Average log probability of the segment
    pub avg_logprob: f64,
    /// Probability that this segment contains no speech
    pub no_speech_prob: f64,
}

impl Segment {
    /// Create a new segment with default probabilities
    pub fn new(start: f64, end: f64, text: String) -> Self {
        Self {
            start_seconds: start,
            end_seconds: end,
            text,
            avg_logprob: 0.0,
            no_speech_prob: 0.0,
        }
    }
}
