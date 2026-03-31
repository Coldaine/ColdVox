// STT abstraction and optional engine implementations (feature-gated)

// Re-export core STT types from the new crate
pub use coldvox_stt::{
    next_utterance_id, EventBasedTranscriber, Transcriber, TranscriptionConfig, TranscriptionEvent,
    WordInfo,
};

#[cfg(any(feature = "moonshine", feature = "parakeet", feature = "http-remote"))]
pub mod processor;

pub mod session;

#[cfg(any(feature = "moonshine", feature = "parakeet", feature = "http-remote"))]
pub mod persistence;

pub mod plugin_manager;

#[cfg(test)]
mod tests;
