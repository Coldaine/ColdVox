//! Vosk speech recognition implementation for ColdVox STT
//!
//! This crate provides Vosk-specific implementations of the ColdVox STT traits.
//! The implementation is feature-gated behind the "vosk" feature.

use coldvox_stt::{EventBasedTranscriber, TranscriptionConfig};

#[cfg(feature = "vosk")]
pub mod vosk_transcriber;

#[cfg(feature = "vosk")]
pub use vosk_transcriber::VoskTranscriber;

#[cfg(feature = "vosk")]
pub mod model;

#[cfg(feature = "vosk")]
pub mod plugin;

#[cfg(feature = "vosk")]
pub use plugin::VoskPluginFactory;

/// Create a new Vosk transcriber as a trait object
#[cfg(feature = "vosk")]
pub fn create_transcriber(
    config: TranscriptionConfig,
    sample_rate: f32,
) -> Result<Box<dyn EventBasedTranscriber>, String> {
    VoskTranscriber::new(config, sample_rate).map(|t| Box::new(t) as Box<dyn EventBasedTranscriber>)
}

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
pub fn create_transcriber(
    _config: TranscriptionConfig,
    _sample_rate: f32,
) -> Result<Box<dyn EventBasedTranscriber>, String> {
    Err("Vosk feature is not enabled. Enable with --features vosk".to_string())
}
