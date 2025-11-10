//! Speech-to-text abstraction layer for ColdVox
//!
//! This crate provides the core abstractions for speech-to-text functionality,
//! including transcription events, configuration, and the base Transcriber trait.

use async_trait::async_trait;
use std::sync::atomic::{AtomicU64, Ordering};

pub mod constants;
// pub mod helpers; // TODO: Requires coldvox_telemetry dependency - move to app crate
pub mod plugin;
pub mod plugin_adapter; // new adapter implementing StreamingStt
pub mod plugin_types;
pub mod plugins;
pub mod processor; // legacy (EventBasedTranscriber-based) processor
pub mod types;

pub use coldvox_foundation::error::ColdVoxError;
pub use plugin::SttPlugin;
pub use plugin_adapter::PluginAdapter; // adapter for plugin → StreamingStt
pub use types::{TranscriptionConfig, TranscriptionEvent, WordInfo};
pub mod validation;

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
pub trait EventBasedTranscriber: Send + Sync {
    /// Accept PCM16 audio and return transcription events
    fn accept_frame(&mut self, pcm: &[i16]) -> Result<Option<TranscriptionEvent>, String>;

    /// Finalize current utterance and return final result
    fn finalize_utterance(&mut self) -> Result<Option<TranscriptionEvent>, String>;

    /// Reset transcriber state for new utterance
    fn reset(&mut self) -> Result<(), String>;

    /// Get current configuration
    fn config(&self) -> &TranscriptionConfig;
}

/// Streaming STT interface used by the new async processor.
/// This mirrors the agent branch simpler interface: per-frame processing,
/// finalize at speech end, and reset. Additional richer streaming methods
/// can be layered later if needed.
#[async_trait]
pub trait StreamingStt: Send + Sync {
    async fn on_speech_frame(&mut self, samples: &[i16]) -> Option<TranscriptionEvent>;
    async fn on_speech_end(&mut self) -> Option<TranscriptionEvent>;
    async fn reset(&mut self);
}
