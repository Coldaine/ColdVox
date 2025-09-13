//! Vosk speech recognition implementation for ColdVox STT
//!
//! This crate provides Vosk-specific implementations of the ColdVox STT traits.
//! The implementation is feature-gated behind the "vosk" feature.

#[cfg(feature = "vosk")]
pub mod vosk_transcriber;

#[cfg(feature = "vosk")]
pub use vosk_transcriber::VoskTranscriber;

pub mod types;

pub use types::{TranscriptionConfig, TranscriptionEvent, WordInfo};

#[cfg(feature = "vosk")]
pub mod model;

/// Get default model path from environment or fallback (string form).
/// Delegates to `model::default_model_path` when the feature is enabled.
pub fn default_model_path() -> String {
    #[cfg(feature = "vosk")]
    {
        model::default_model_path().to_string_lossy().to_string()
    }
    #[cfg(not(feature = "vosk"))]
    {
        std::env::var("VOSK_MODEL_PATH")
            .unwrap_or_else(|_| "models/vosk-model-small-en-us-0.15".to_string())
    }
}

#[cfg(not(feature = "vosk"))]
pub fn create_default_transcriber(_config: TranscriptionConfig) -> Result<(), String> {
    Err("Vosk feature is not enabled. Enable with --features vosk".to_string())
}
