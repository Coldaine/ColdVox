//! Candle-based Whisper engine for ColdVox
//!
//! This module provides a pure Rust implementation of Whisper STT using the Candle ML framework.
//! It replaces the previous Python-based faster-whisper backend with a fully native solution.
//!
//! # Architecture
//!
//! The implementation is organized into several submodules:
//! - `types`: Public API types and configuration
//! - `audio`: Audio preprocessing (mel spectrogram computation)
//! - `loader`: Model and tokenizer loading
//! - `decode`: Core transcription logic
//! - `timestamps`: Timestamp generation and processing
//!
//! # Usage
//!
//! ```rust,no_run
//! use coldvox_stt::candle::{WhisperEngine, WhisperEngineInit, TranscribeOptions, WhisperDevice};
//! use std::path::PathBuf;
//!
//! # fn main() -> anyhow::Result<()> {
//! // Initialize engine
//! let init = WhisperEngineInit {
//!     model_path: PathBuf::from("models/whisper-base"),
//!     tokenizer_path: PathBuf::from("models/whisper-base/tokenizer.json"),
//!     config_path: PathBuf::from("models/whisper-base/config.json"),
//!     quantized: false,
//!     device: WhisperDevice::Cpu,
//! };
//!
//! let engine = WhisperEngine::new(init)?;
//!
//! // Transcribe audio
//! let pcm_audio: Vec<f32> = vec![]; // Your 16kHz mono audio
//! let options = TranscribeOptions::default();
//! let transcript = engine.transcribe(&pcm_audio, &options)?;
//!
//! println!("Transcript: {}", transcript.text);
//! # Ok(())
//! # }
//! ```

mod audio;
mod decode;
mod loader;
mod timestamps;
pub mod types;

pub use types::{
    Segment, Transcript, TranscribeOptions, WhisperDevice, WhisperEngineInit, WhisperTask,
};

use anyhow::{Context, Result};
use candle_core::Device;
use candle_transformers::models::whisper::Config;

use audio::{log_mel_spectrogram, pcm16_to_f32};
use decode::WhisperDecoder;
use loader::{load_model, resolve_model_paths, LoadedModel};

/// Main Whisper engine facade
///
/// This struct provides a clean, high-level interface to the Candle-based Whisper
/// implementation. It handles model loading, audio preprocessing, and transcription.
pub struct WhisperEngine {
    model: candle_transformers::models::whisper::model::Whisper,
    config: Config,
    tokenizer: tokenizers::Tokenizer,
    device: Device,
}

impl WhisperEngine {
    /// Create a new WhisperEngine instance
    ///
    /// # Arguments
    /// * `init` - Initialization configuration
    ///
    /// # Returns
    /// A new WhisperEngine instance ready for transcription
    ///
    /// # Errors
    /// Returns an error if:
    /// - Model files cannot be loaded
    /// - Device initialization fails
    /// - Model architecture is incompatible
    pub fn new(init: WhisperEngineInit) -> Result<Self> {
        tracing::info!(
            "Initializing Candle Whisper engine with model: {:?}",
            init.model_path
        );

        // Convert device
        let device = match init.device {
            WhisperDevice::Cpu => Device::Cpu,
            WhisperDevice::Cuda(id) => Device::new_cuda(id)
                .context("Failed to initialize CUDA device")?,
        };

        // Load model
        let LoadedModel {
            model,
            config,
            tokenizer,
        } = load_model(
            &init.model_path,
            &init.config_path,
            &init.tokenizer_path,
            init.quantized,
            &device,
        )
        .context("Failed to load Whisper model")?;

        tracing::info!("Candle Whisper engine initialized successfully");

        Ok(Self {
            model,
            config,
            tokenizer,
            device,
        })
    }

    /// Create a new WhisperEngine from a model identifier
    ///
    /// This is a convenience method that handles model path resolution,
    /// including downloading from HuggingFace Hub if necessary.
    ///
    /// # Arguments
    /// * `model_id` - Model identifier (local path or HuggingFace model ID)
    /// * `quantized` - Whether to use quantized model
    /// * `device` - Device to run inference on
    ///
    /// # Example
    /// ```rust,no_run
    /// # use coldvox_stt::candle::{WhisperEngine, WhisperDevice};
    /// # fn main() -> anyhow::Result<()> {
    /// let engine = WhisperEngine::from_model_id(
    ///     "openai/whisper-base",
    ///     false,
    ///     WhisperDevice::Cpu,
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_model_id(
        model_id: &str,
        quantized: bool,
        device: WhisperDevice,
    ) -> Result<Self> {
        let (model_path, config_path, tokenizer_path) =
            resolve_model_paths(model_id, quantized)
                .context("Failed to resolve model paths")?;

        let init = WhisperEngineInit {
            model_path,
            config_path,
            tokenizer_path,
            quantized,
            device,
        };

        Self::new(init)
    }

    /// Transcribe audio to text
    ///
    /// # Arguments
    /// * `pcm_audio` - 16kHz mono PCM audio as f32 samples (normalized to [-1.0, 1.0])
    /// * `opts` - Transcription options (language, task, temperature, timestamps)
    ///
    /// # Returns
    /// A Transcript containing segments with text and timing information
    ///
    /// # Errors
    /// Returns an error if:
    /// - Audio preprocessing fails
    /// - Model inference fails
    /// - Decoding produces invalid output
    pub fn transcribe(&self, pcm_audio: &[f32], opts: &TranscribeOptions) -> Result<Transcript> {
        tracing::debug!("Starting transcription of {} samples", pcm_audio.len());

        // Compute mel spectrogram
        let mel = log_mel_spectrogram(pcm_audio, &self.device)
            .context("Failed to compute mel spectrogram")?;

        tracing::debug!("Mel spectrogram computed, shape: {:?}", mel.shape());

        // Create decoder and run transcription
        let decoder = WhisperDecoder::new(&self.model, &self.config, &self.tokenizer, &self.device)
            .context("Failed to create decoder")?;

        let transcript = decoder
            .decode(&mel, opts)
            .context("Failed to decode audio")?;

        tracing::debug!(
            "Transcription complete: {} segments, {} chars",
            transcript.segments.len(),
            transcript.text.len()
        );

        Ok(transcript)
    }

    /// Transcribe i16 PCM audio
    ///
    /// Convenience method that accepts i16 PCM samples and converts them to f32.
    ///
    /// # Arguments
    /// * `pcm16` - 16kHz mono PCM audio as i16 samples
    /// * `opts` - Transcription options
    pub fn transcribe_pcm16(&self, pcm16: &[i16], opts: &TranscribeOptions) -> Result<Transcript> {
        let pcm_f32 = pcm16_to_f32(pcm16);
        self.transcribe(&pcm_f32, opts)
    }

    /// Get the device being used for inference
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// Get the model configuration
    pub fn config(&self) -> &Config {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_api_types() {
        // Verify that the API types compile and have expected defaults
        let opts = TranscribeOptions::default();
        assert_eq!(opts.task, WhisperTask::Transcribe);
        assert_eq!(opts.temperature, 0.0);
        assert!(opts.enable_timestamps);
    }
}
