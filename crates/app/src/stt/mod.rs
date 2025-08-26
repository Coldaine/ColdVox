// STT abstraction and optional engine implementations (feature-gated)

use std::sync::atomic::{AtomicU64, Ordering};

/// Generates unique utterance IDs
static UTTERANCE_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

pub fn next_utterance_id() -> u64 {
    UTTERANCE_ID_COUNTER.fetch_add(1, Ordering::SeqCst)
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
    Error {
        code: String,
        message: String,
    },
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
    /// Path to Vosk model directory
    pub model_path: String,
    /// Emit partial recognition results
    pub partial_results: bool,
    /// Maximum alternatives in results
    pub max_alternatives: u32,
    /// Include word-level timing in results
    pub include_words: bool,
    /// Buffer size in milliseconds
    pub buffer_size_ms: u32,
}

impl Default for TranscriptionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            model_path: String::new(),
            partial_results: true,
            max_alternatives: 1,
            include_words: false,
            buffer_size_ms: 512,
        }
    }
}

/// Minimal streaming transcription interface (deprecated - kept for backward compatibility)
/// New code should use VoskTranscriber directly with TranscriptionEvent
pub trait Transcriber {
    /// Feed 16 kHz, mono, S16LE PCM samples.
    /// Returns Some(final_text_or_json) when an utterance completes, else None.
    fn accept_pcm16(&mut self, pcm: &[i16]) -> Result<Option<String>, String>;

    /// Signal end of input for the current utterance and get final result if any.
    fn finalize(&mut self) -> Result<Option<String>, String>;
}

#[cfg(feature = "vosk")]
pub mod vosk;

#[cfg(feature = "vosk")]
pub use vosk::VoskTranscriber;

#[cfg(feature = "vosk")]
pub mod processor;
