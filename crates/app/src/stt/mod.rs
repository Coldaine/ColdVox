// STT abstraction and optional engine implementations (feature-gated)

// Re-export core STT types from the new crate
pub use coldvox_stt::{
    next_utterance_id, AsyncEventBasedTranscriber, EventBasedTranscriber, Transcriber, TranscriptionConfig, TranscriptionEvent,
    WordInfo,
};

#[cfg(feature = "vosk")]
pub mod vosk;

#[cfg(feature = "vosk")]
pub use vosk::{VoskTranscriber, AsyncVoskTranscriber};

#[cfg(feature = "vosk")]
pub mod processor;

#[cfg(feature = "vosk")]
pub use processor::{SttProcessor, AsyncSttProcessor};

#[cfg(feature = "vosk")]
pub mod persistence;

#[cfg(test)]
mod tests;
