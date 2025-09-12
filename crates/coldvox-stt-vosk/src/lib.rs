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

#[cfg(feature = "vosk")]
pub mod model;

#[cfg(feature = "vosk")]
pub use model::locate_model;

#[cfg(not(feature = "vosk"))]
pub fn create_default_transcriber(_config: TranscriptionConfig) -> Result<(), String> {
    Err("Vosk feature is not enabled. Enable with --features vosk".to_string())
}
