//! Type definitions for Candle-based Whisper engine
//!
//! This module defines the public API types for the WhisperEngine facade.

use std::path::PathBuf;

/// Device configuration for Whisper inference
#[derive(Debug, Clone)]
pub enum WhisperDevice {
    /// CPU inference
    Cpu,
    /// CUDA GPU inference with device ID
    Cuda(usize),
}

/// Task type for Whisper inference
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhisperTask {
    /// Transcribe audio in the same language
    Transcribe,
    /// Translate audio to English
    Translate,
}

/// Configuration for initializing the WhisperEngine
#[derive(Debug, Clone)]
pub struct WhisperEngineInit {
    /// Path to the model file (safetensors or GGUF)
    pub model_path: PathBuf,
    /// Path to the tokenizer configuration
    pub tokenizer_path: PathBuf,
    /// Path to the model configuration JSON
    pub config_path: PathBuf,
    /// Whether the model is quantized (GGUF format)
    pub quantized: bool,
    /// Device to run inference on
    pub device: WhisperDevice,
}

/// Options for transcription
#[derive(Debug, Clone)]
pub struct TranscribeOptions {
    /// Language code (e.g., "en", "es"). None for auto-detection.
    pub language: Option<String>,
    /// Task type (transcribe or translate)
    pub task: WhisperTask,
    /// Temperature for sampling. 0.0 for greedy decoding, > 0.0 for temperature sampling.
    pub temperature: f32,
    /// Whether to generate timestamps
    pub enable_timestamps: bool,
}

impl Default for TranscribeOptions {
    fn default() -> Self {
        Self {
            language: None,
            task: WhisperTask::Transcribe,
            temperature: 0.0, // Greedy by default
            enable_timestamps: true,
        }
    }
}

/// A segment of transcribed text with timing information
#[derive(Debug, Clone)]
pub struct Segment {
    /// Start time in seconds
    pub start_seconds: f64,
    /// End time in seconds
    pub end_seconds: f64,
    /// Transcribed text for this segment
    pub text: String,
    /// Average log probability for this segment
    pub avg_logprob: f64,
    /// No-speech probability for this segment
    pub no_speech_prob: f64,
}

/// Complete transcription result
#[derive(Debug, Clone)]
pub struct Transcript {
    /// Transcribed segments
    pub segments: Vec<Segment>,
    /// Detected or specified language code
    pub language: Option<String>,
    /// Full concatenated text
    pub text: String,
}

impl Transcript {
    /// Create a new transcript from segments
    pub fn new(segments: Vec<Segment>, language: Option<String>) -> Self {
        let text = segments
            .iter()
            .map(|s| s.text.trim())
            .collect::<Vec<_>>()
            .join(" ");

        Self {
            segments,
            language,
            text,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_whisper_task_equality() {
        assert_eq!(WhisperTask::Transcribe, WhisperTask::Transcribe);
        assert_eq!(WhisperTask::Translate, WhisperTask::Translate);
        assert_ne!(WhisperTask::Transcribe, WhisperTask::Translate);
    }

    #[test]
    fn test_transcribe_options_default() {
        let opts = TranscribeOptions::default();
        assert_eq!(opts.language, None);
        assert_eq!(opts.task, WhisperTask::Transcribe);
        assert_eq!(opts.temperature, 0.0);
        assert!(opts.enable_timestamps);
    }

    #[test]
    fn test_transcribe_options_custom() {
        let opts = TranscribeOptions {
            language: Some("es".to_string()),
            task: WhisperTask::Translate,
            temperature: 0.8,
            enable_timestamps: false,
        };

        assert_eq!(opts.language.as_deref(), Some("es"));
        assert_eq!(opts.task, WhisperTask::Translate);
        assert_eq!(opts.temperature, 0.8);
        assert!(!opts.enable_timestamps);
    }

    #[test]
    fn test_segment_creation() {
        let segment = Segment {
            start_seconds: 0.0,
            end_seconds: 2.5,
            text: "Hello world".to_string(),
            avg_logprob: -0.5,
            no_speech_prob: 0.01,
        };

        assert_eq!(segment.start_seconds, 0.0);
        assert_eq!(segment.end_seconds, 2.5);
        assert_eq!(segment.text, "Hello world");
        assert_eq!(segment.avg_logprob, -0.5);
        assert_eq!(segment.no_speech_prob, 0.01);
    }

    #[test]
    fn test_transcript_single_segment() {
        let segments = vec![Segment {
            start_seconds: 0.0,
            end_seconds: 2.5,
            text: "Hello world".to_string(),
            avg_logprob: -0.5,
            no_speech_prob: 0.01,
        }];

        let transcript = Transcript::new(segments, Some("en".to_string()));

        assert_eq!(transcript.text, "Hello world");
        assert_eq!(transcript.language.as_deref(), Some("en"));
        assert_eq!(transcript.segments.len(), 1);
    }

    #[test]
    fn test_transcript_multiple_segments() {
        let segments = vec![
            Segment {
                start_seconds: 0.0,
                end_seconds: 2.5,
                text: "Hello world".to_string(),
                avg_logprob: -0.5,
                no_speech_prob: 0.01,
            },
            Segment {
                start_seconds: 2.5,
                end_seconds: 5.0,
                text: "How are you".to_string(),
                avg_logprob: -0.3,
                no_speech_prob: 0.02,
            },
        ];

        let transcript = Transcript::new(segments, Some("en".to_string()));

        assert_eq!(transcript.text, "Hello world How are you");
        assert_eq!(transcript.segments.len(), 2);
    }

    #[test]
    fn test_transcript_trimming() {
        let segments = vec![
            Segment {
                start_seconds: 0.0,
                end_seconds: 2.5,
                text: "  Hello  ".to_string(),
                avg_logprob: -0.5,
                no_speech_prob: 0.01,
            },
            Segment {
                start_seconds: 2.5,
                end_seconds: 5.0,
                text: "  world  ".to_string(),
                avg_logprob: -0.3,
                no_speech_prob: 0.02,
            },
        ];

        let transcript = Transcript::new(segments, None);

        assert_eq!(transcript.text, "Hello world");
    }

    #[test]
    fn test_transcript_empty_segments() {
        let segments = vec![];
        let transcript = Transcript::new(segments, None);

        assert_eq!(transcript.text, "");
        assert_eq!(transcript.language, None);
        assert_eq!(transcript.segments.len(), 0);
    }

    #[test]
    fn test_whisper_device_clone() {
        let cpu = WhisperDevice::Cpu;
        let cpu2 = cpu.clone();

        let cuda = WhisperDevice::Cuda(0);
        let cuda2 = cuda.clone();

        // Just verify cloning works
        drop(cpu2);
        drop(cuda2);
    }

    #[test]
    fn test_whisper_engine_init_clone() {
        let init = WhisperEngineInit {
            model_path: PathBuf::from("/path/to/model"),
            tokenizer_path: PathBuf::from("/path/to/tokenizer"),
            config_path: PathBuf::from("/path/to/config"),
            quantized: false,
            device: WhisperDevice::Cpu,
        };

        let init2 = init.clone();
        assert_eq!(init.model_path, init2.model_path);
        assert_eq!(init.quantized, init2.quantized);
    }
}
