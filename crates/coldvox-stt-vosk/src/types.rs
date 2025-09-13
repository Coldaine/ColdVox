//! Core types for speech-to-text functionality

use std::sync::atomic::{AtomicU64, Ordering};

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

/// Transcription event types
#[derive(Debug, Clone)]
pub enum TranscriptionEvent {
    /// Partial transcription result (ongoing speech)
    Partial {
        utterance_id: u64,
        text: String,
        /// Optional start time offset in seconds
        t0: Option<f32>,
        /// Optional end time offset in seconds
        t1: Option<f32>,
    },
    /// Final transcription result (speech segment complete)
    Final {
        utterance_id: u64,
        text: String,
        /// Optional word-level timing information
        words: Option<Vec<WordInfo>>,
    },
    /// Transcription error
    Error { code: String, message: String },
}

/// Word-level timing and confidence information
#[derive(Debug, Clone)]
pub struct WordInfo {
    /// Start time in seconds
    pub start: f32,
    /// End time in seconds
    pub end: f32,
    /// Confidence score (0.0-1.0)
    pub conf: f32,
    /// Word text
    pub text: String,
}

/// Transcription configuration
#[derive(Debug, Clone)]
pub struct TranscriptionConfig {
    /// Enable/disable transcription
    pub enabled: bool,
    /// Path to model directory or file
    pub model_path: String,
    /// Emit partial recognition results
    pub partial_results: bool,
    /// Maximum alternatives in results
    pub max_alternatives: u32,
    /// Include word-level timing in results
    pub include_words: bool,
    /// Buffer size in milliseconds
    pub buffer_size_ms: u32,
    /// Enable streaming mode (process audio incrementally vs batch)
    pub streaming: bool,
}

impl Default for TranscriptionConfig {
    fn default() -> Self {
        // Try to get model path from environment, falling back to default
        let model_path = std::env::var("VOSK_MODEL_PATH")
            .unwrap_or_else(|_| "models/vosk-model-small-en-us-0.15".to_string());

        Self {
            enabled: false,
            model_path,
            partial_results: true,
            max_alternatives: 1,
            include_words: false,
            buffer_size_ms: 512,
            streaming: false, // Default to batch mode for backward compatibility
        }
    }
}