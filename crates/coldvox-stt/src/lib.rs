//! Speech-to-text abstraction layer for ColdVox
//!
//! This crate provides the core abstractions for speech-to-text functionality,
//! including transcription events, configuration, and the base Transcriber trait.

use std::sync::atomic::{AtomicU64, Ordering};

pub mod plugin;
pub mod plugins;
pub mod processor;
pub mod types;

pub use types::{TranscriptionConfig, TranscriptionEvent, WordInfo};

/// Generates unique utterance IDs
static UTTERANCE_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Generate a unique utterance ID
pub fn next_utterance_id() -> u64 {
    UTTERANCE_ID_COUNTER.fetch_add(1, Ordering::SeqCst)
}

/// Core transcription interface
///
/// This trait defines the minimal interface for streaming transcription.
/// It's kept for backward compatibility - new implementations should use
/// the event-based interface with TranscriptionEvent.
pub trait Transcriber {
    /// Feed 16 kHz, mono, S16LE PCM samples.
    /// Returns Some(final_text_or_json) when an utterance completes, else None.
    fn accept_pcm16(&mut self, pcm: &[i16]) -> Result<Option<String>, String>;

    /// Signal end of input for the current utterance and get final result if any.
    fn finalize(&mut self) -> Result<Option<String>, String>;
}

/// Modern event-based transcription interface
///
/// Implementations should prefer this interface over the legacy Transcriber trait.
pub trait EventBasedTranscriber {
    /// Accept PCM16 audio and return transcription events
    fn accept_frame(&mut self, pcm: &[i16]) -> Result<Option<TranscriptionEvent>, String>;

    /// Finalize current utterance and return final result
    fn finalize_utterance(&mut self) -> Result<Option<TranscriptionEvent>, String>;

    /// Reset transcriber state for new utterance
    fn reset(&mut self) -> Result<(), String>;

    /// Get current configuration
    fn config(&self) -> &TranscriptionConfig;
}

/// Async wrapper trait for non-blocking transcription operations
///
/// This trait provides async versions of transcriber operations that won't block
/// the tokio runtime, improving UI responsiveness and enabling concurrent processing.
#[async_trait::async_trait]
pub trait AsyncEventBasedTranscriber {
    /// Accept PCM16 audio and return transcription events (async)
    async fn accept_frame_async(&mut self, pcm: Vec<i16>) -> Result<Option<TranscriptionEvent>, String>;

    /// Finalize current utterance and return final result (async)
    async fn finalize_utterance_async(&mut self) -> Result<Option<TranscriptionEvent>, String>;

    /// Reset transcriber state for new utterance (async)
    async fn reset_async(&mut self) -> Result<(), String>;

    /// Get current configuration (synchronous - no blocking I/O)
    fn config(&self) -> &TranscriptionConfig;
}
