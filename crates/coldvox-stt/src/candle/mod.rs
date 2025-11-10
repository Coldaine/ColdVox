//! Candle-based Whisper STT engine.
//!
//! This module provides a pure-Rust Whisper implementation using the Candle ML framework.
//! It replaces the previous Python-based faster-whisper backend.
//!
//! # Architecture
//!
//! The implementation is organized into several modules:
//! - `audio`: Audio preprocessing (PCM to mel spectrogram)
//! - `loader`: Model loading (SafeTensors and GGUF formats)
//! - `decode`: Token-by-token decoding with KV cache
//! - `timestamps`: Timestamp extraction and segment generation
//! - `types`: Public API types and configuration
//!
//! # Usage
//!
//! ```ignore
//! use coldvox_stt::candle::{WhisperEngine, WhisperEngineInit, TranscribeOptions, WhisperDevice};
//!
//! let init = WhisperEngineInit {
//!     model_path: "model.safetensors".into(),
//!     tokenizer_path: "tokenizer.json".into(),
//!     config_path: "config.json".into(),
//!     quantized: false,
//!     device: WhisperDevice::Cpu,
//! };
//!
//! let engine = WhisperEngine::new(init)?;
//! let audio: Vec<f32> = /* ... */;
//! let transcript = engine.transcribe(&audio, &TranscribeOptions::default())?;
//! ```

pub mod audio;
pub mod decode;
pub mod loader;
pub mod timestamps;
pub mod types;

pub use types::{
    Segment, TranscribeOptions, Transcript, WhisperDevice, WhisperEngineInit, WhisperTask,
};

#[cfg(feature = "whisper")]
use anyhow::{Context, Result};
#[cfg(feature = "whisper")]
use candle_core::Device;
#[cfg(feature = "whisper")]
use candle_transformers::models::whisper::{Config, Whisper};
#[cfg(feature = "whisper")]
use tokenizers::Tokenizer;

/// Main Whisper engine facade.
///
/// This is the primary interface for speech-to-text transcription using Candle Whisper.
/// It manages the model, tokenizer, and device, and provides a clean API for transcription.
#[cfg(feature = "whisper")]
pub struct WhisperEngine {
    model: Whisper,
    tokenizer: Tokenizer,
    config: Config,
    device: Device,
}

#[cfg(feature = "whisper")]
impl WhisperEngine {
    /// Create a new Whisper engine instance.
    ///
    /// This loads the model, tokenizer, and configuration from the specified paths.
    ///
    /// # Arguments
    /// * `init` - Initialization configuration
    ///
    /// # Returns
    /// A new WhisperEngine instance, or an error if loading fails
    pub fn new(init: WhisperEngineInit) -> Result<Self> {
        // Convert device to Candle device
        let device = init.device.to_candle_device()
            .context("Failed to initialize device")?;

        // Load model
        let model = loader::load_model(&init.model_path, &init.config_path, &device)
            .context("Failed to load Whisper model")?;

        // Load tokenizer
        let tokenizer = loader::load_tokenizer(&init.tokenizer_path)
            .context("Failed to load tokenizer")?;

        // Load config
        let config_str = std::fs::read_to_string(&init.config_path)
            .context("Failed to read model config")?;
        let config: Config = serde_json::from_str(&config_str)
            .context("Failed to parse model config")?;

        Ok(Self {
            model,
            tokenizer,
            config,
            device,
        })
    }

    /// Transcribe audio to text.
    ///
    /// # Arguments
    /// * `pcm_audio` - 16kHz mono f32 PCM audio samples (normalized to [-1, 1])
    /// * `opts` - Transcription options (language, task, temperature, timestamps)
    ///
    /// # Returns
    /// A Transcript containing segments with text and timing information
    pub fn transcribe(&self, pcm_audio: &[f32], opts: &TranscribeOptions) -> Result<Transcript> {
        // Preprocess audio to mel spectrogram
        let mel = audio::log_mel_spectrogram_from_f32(pcm_audio, &self.device)
            .context("Failed to compute mel spectrogram")?;

        // Create decoder
        let mut decoder = decode::Decoder::new(
            &self.model,
            &self.tokenizer,
            &self.config,
            &self.device,
            42, // Random seed for sampling
        );

        // Encode audio
        let encoder_output = decoder.encode(&mel)
            .context("Failed to encode audio")?;

        // Decode to transcript
        decoder.decode(&encoder_output, opts)
            .context("Failed to decode audio")
    }

    /// Transcribe i16 PCM audio (convenience method).
    ///
    /// This is a convenience wrapper that converts i16 PCM to f32 before transcription.
    ///
    /// # Arguments
    /// * `pcm_audio` - 16kHz mono i16 PCM audio samples
    /// * `opts` - Transcription options
    ///
    /// # Returns
    /// A Transcript containing segments with text and timing information
    pub fn transcribe_pcm16(&self, pcm_audio: &[i16], opts: &TranscribeOptions) -> Result<Transcript> {
        let pcm_f32 = audio::pcm_to_f32(pcm_audio);
        self.transcribe(&pcm_f32, opts)
    }

    /// Get the device this engine is running on
    pub fn device(&self) -> &Device {
        &self.device
    }
}

// Provide stub types when the whisper feature is not enabled
#[cfg(not(feature = "whisper"))]
pub struct WhisperEngine;

#[cfg(not(feature = "whisper"))]
impl WhisperEngine {
    pub fn new(_init: WhisperEngineInit) -> Result<Self, String> {
        Err("Whisper feature not enabled".to_string())
    }

    pub fn transcribe(&self, _pcm_audio: &[f32], _opts: &TranscribeOptions) -> Result<Transcript, String> {
        Err("Whisper feature not enabled".to_string())
    }

    pub fn transcribe_pcm16(&self, _pcm_audio: &[i16], _opts: &TranscribeOptions) -> Result<Transcript, String> {
        Err("Whisper feature not enabled".to_string())
    }
}

#[cfg(test)]
#[cfg(feature = "whisper")]
mod tests {
    use super::*;

    // Integration tests would require actual model files
    // These are placeholder tests to verify API structure

    #[test]
    fn test_transcribe_options_default() {
        let opts = TranscribeOptions::default();
        assert_eq!(opts.temperature, 0.0); // Greedy by default
        assert_eq!(opts.task, WhisperTask::Transcribe);
        assert!(opts.enable_timestamps);
        assert_eq!(opts.language, None);
    }

    #[test]
    fn test_segment_creation() {
        let seg = Segment::new(0.0, 1.5, "Hello world".to_string());
        assert_eq!(seg.start_seconds, 0.0);
        assert_eq!(seg.end_seconds, 1.5);
        assert_eq!(seg.text, "Hello world");
    }

    #[test]
    fn test_whisper_device() {
        let cpu_device = WhisperDevice::Cpu;
        let cuda_device = WhisperDevice::Cuda(0);

        assert_eq!(cpu_device, WhisperDevice::Cpu);
        assert_eq!(cuda_device, WhisperDevice::Cuda(0));
        assert_ne!(cpu_device, cuda_device);
    }
}
