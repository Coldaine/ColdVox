//! Vosk speech recognition implementation for ColdVox STT
//!
//! This crate provides Vosk-specific implementations of the ColdVox STT traits.
//! The implementation is feature-gated behind the "vosk" feature.

#[cfg(feature = "vosk")]
pub mod vosk_transcriber;

#[cfg(feature = "vosk")]
pub use vosk_transcriber::VoskTranscriber;

// Re-export common types
pub use coldvox_stt::{
    next_utterance_id, EventBasedTranscriber, Transcriber, TranscriptionConfig, TranscriptionEvent,
    WordInfo,
};

/// Get default model path from environment or fallback
pub fn default_model_path() -> String {
    std::env::var("VOSK_MODEL_PATH").unwrap_or_else(|_| "vosk-model-small-en-us-0.15".to_string())
}

#[cfg(not(feature = "vosk"))]
pub fn create_default_transcriber(_config: TranscriptionConfig) -> Result<(), String> {
    Err("Vosk feature is not enabled. Enable with --features vosk".to_string())
}
