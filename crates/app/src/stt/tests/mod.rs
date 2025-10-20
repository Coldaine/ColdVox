pub mod timeout_utils;
/// STT test utilities and end-to-end tests
///
/// This module provides utilities for testing speech-to-text functionality,
/// including WER calculation, timeout handling, and integration tests.
pub mod wer_utils;

#[cfg(feature = "whisper")]
pub mod end_to_end_wav;

#[cfg(test)]
mod stt_core_tests {
    use crate::stt::*;

    #[test]
    fn test_transcription_config_default() {
        std::env::remove_var("WHISPER_MODEL_PATH");
        let config = TranscriptionConfig::default();
        assert!(!config.enabled);
        let expected_path = std::env::var("WHISPER_MODEL_PATH").unwrap_or_else(|_| "base.en".to_string());
        assert_eq!(config.model_path, expected_path);
        assert!(config.partial_results);
        assert_eq!(config.max_alternatives, 1);
        assert!(!config.include_words);
        assert_eq!(config.buffer_size_ms, 512);
        assert!(!config.streaming);
    }

    #[test]
    fn test_utterance_id_generation() {
        let id1 = next_utterance_id();
        let id2 = next_utterance_id();
        assert_ne!(id1, id2);
        assert_eq!(id2, id1 + 1);
    }

    #[test]
    fn test_word_info_creation() {
        let word = WordInfo {
            text: "hello".to_string(),
            start: 0.5,
            end: 1.0,
            conf: 0.95,
        };
        assert_eq!(word.text, "hello");
        assert_eq!(word.start, 0.5);
        assert_eq!(word.end, 1.0);
        assert_eq!(word.conf, 0.95);
    }

    #[test]
    fn test_transcription_event_variants() {
        let partial = TranscriptionEvent::Partial {
            utterance_id: 1,
            text: "partial text".to_string(),
            t0: Some(0.0),
            t1: Some(1.0),
        };

        match partial {
            TranscriptionEvent::Partial { text, .. } => {
                assert_eq!(text, "partial text");
            }
            _ => panic!("Expected Partial variant"),
        }

        let error = TranscriptionEvent::Error {
            code: "TEST_ERROR".to_string(),
            message: "Test error message".to_string(),
        };

        match error {
            TranscriptionEvent::Error { code, message } => {
                assert_eq!(code, "TEST_ERROR");
                assert_eq!(message, "Test error message");
            }
            _ => panic!("Expected Error variant"),
        }
    }
}

#[cfg(all(test, feature = "whisper"))]
mod processor_tests {
    use crate::stt::processor::*;

    #[test]
    fn test_utterance_state_transitions() {
        let idle = UtteranceState::Idle;
        matches!(idle, UtteranceState::Idle);

        let active = UtteranceState::SpeechActive;
        matches!(active, UtteranceState::SpeechActive);

        let finalizing = UtteranceState::Finalizing;
        matches!(finalizing, UtteranceState::Finalizing);
    }

    #[test]
    fn test_stt_metrics_default() {
        let metrics = SttMetrics::default();
        assert_eq!(metrics.frames_in, 0);
        assert_eq!(metrics.partial_count, 0);
        assert_eq!(metrics.final_count, 0);
        assert_eq!(metrics.error_count, 0);
        assert!(metrics.last_event_time.is_none());
    }
}
