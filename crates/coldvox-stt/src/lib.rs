//! Speech-to-text abstraction layer for ColdVox
//!
//! This crate provides the core abstractions for speech-to-text functionality,
//! including transcription events, configuration, and the base Transcriber trait.

use async_trait::async_trait;
use std::sync::atomic::{AtomicU64, Ordering};

pub mod plugin;
pub mod plugin_types;
pub mod plugins;
pub mod processor;
pub mod types;

pub use types::{TranscriptionConfig, TranscriptionEvent, WordInfo};
pub use plugin::{SttPlugin, SttPluginError};
// Re-export streaming interfaces so application layer can depend only on crate root
pub use crate::{StreamingStt as StreamingSttTrait};

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

/// Streaming STT interface for real-time transcription
#[async_trait]
pub trait StreamingStt: Send + Sync {
    /// Process streaming audio and return transcription events
    async fn process_stream(
        &mut self,
        audio: &[i16],
    ) -> Result<Vec<TranscriptionEvent>, Box<dyn std::error::Error + Send + Sync>>;

    /// Finalize the current stream and get remaining transcriptions
    async fn finalize_stream(&mut self) -> Result<Vec<TranscriptionEvent>, Box<dyn std::error::Error + Send + Sync>>;

    /// Reset the streaming state
    async fn reset_stream(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

/// Adapter to bridge SttPlugin to StreamingStt
pub struct PluginAdapter<P: SttPlugin> {
    plugin: P,
}

impl<P: SttPlugin> PluginAdapter<P> {
    pub fn new(plugin: P, _config: TranscriptionConfig) -> Self {
        Self { plugin }
    }

    /// Access underlying plugin (read-only)
    pub fn inner(&self) -> &P { &self.plugin }
}

#[async_trait]
impl<P: SttPlugin> StreamingStt for PluginAdapter<P> {
    async fn process_stream(
        &mut self,
        audio: &[i16],
    ) -> Result<Vec<TranscriptionEvent>, Box<dyn std::error::Error + Send + Sync>> {
        match self.plugin.process_audio(audio).await {
            Ok(Some(event)) => Ok(vec![event]),
            Ok(None) => Ok(vec![]),
            Err(e) => Err(Box::new(e)),
        }
    }

    async fn finalize_stream(&mut self) -> Result<Vec<TranscriptionEvent>, Box<dyn std::error::Error + Send + Sync>> {
        match self.plugin.finalize().await {
            Ok(Some(event)) => Ok(vec![event]),
            Ok(None) => Ok(vec![]),
            Err(e) => Err(Box::new(e)),
        }
    }

    async fn reset_stream(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.plugin.reset().await.map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)) as Box<dyn std::error::Error + Send + Sync>)
    }
}
