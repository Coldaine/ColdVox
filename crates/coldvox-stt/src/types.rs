//! Core types for speech-to-text functionality

use std::time::Instant;

/// STT processor metrics
#[derive(Debug, Clone, Default)]
pub struct SttMetrics {
    /// Total frames received
    pub frames_in: u64,
    /// Total frames processed
    pub frames_out: u64,
    /// Total frames dropped due to overflow
    pub frames_dropped: u64,
    /// Number of partial transcriptions
    pub partial_count: u64,
    /// Number of final transcriptions
    pub final_count: u64,
    /// Number of errors
    pub error_count: u64,
    /// Current queue depth
    pub queue_depth: usize,
    /// Time since last STT event
    pub last_event_time: Option<Instant>,
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
    /// Automatically extract model from a zip archive if not found
    pub auto_extract_model: bool,
}

impl Default for TranscriptionConfig {
    fn default() -> Self {
        // Try to get model path from environment, falling back to default
        let model_path = std::env::var("WHISPER_MODEL_PATH")
            .unwrap_or_else(|_| "base.en".to_string());

        Self {
            enabled: false,
            model_path,
            partial_results: true,
            max_alternatives: 1,
            include_words: false,
            buffer_size_ms: 512,
            streaming: false, // Default to batch mode for backward compatibility
            auto_extract_model: true,
        }
    }
}
