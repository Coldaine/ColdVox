/// STT test utilities and end-to-end tests
/// 
/// This module provides utilities for testing speech-to-text functionality,
/// including WER calculation, timeout handling, and integration tests.

pub mod wer_utils;
pub mod timeout_utils;

#[cfg(feature = "vosk")]
pub mod end_to_end_wav;

#[cfg(test)]
mod vosk_tests {
    use crate::stt::*;

    #[test]
    fn test_transcription_config_default() {
        let config = TranscriptionConfig::default();
        assert!(!config.enabled);
        // Model path should match default or env var if set
        let expected_path = std::env::var("VOSK_MODEL_PATH")
            .unwrap_or_else(|_| "models/vosk-model-small-en-us-0.15".to_string());
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

    #[cfg(feature = "vosk")]
    mod vosk_integration_tests {
        use coldvox_stt::EventBasedTranscriber;

        use crate::stt::vosk::VoskTranscriber;
        use crate::stt::TranscriptionConfig;

        #[test]
        fn test_vosk_transcriber_missing_model() {
            // Ensure the environment variable is not set for this test
            std::env::remove_var("VOSK_MODEL_PATH");

            // Create a path that is guaranteed not to exist
            let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
            let non_existent_path = temp_dir.path().join("non_existent_model");

            let config = TranscriptionConfig {
                enabled: true,
                model_path: non_existent_path.to_str().unwrap().to_string(),
                partial_results: true,
                max_alternatives: 1,
                include_words: false,
                buffer_size_ms: 512,
                streaming: false,
            };

            let result = VoskTranscriber::new(config, 16000.0);
            assert!(
                result.is_err(),
                "Expected an error for a missing model path, but got Ok"
            );
            if let Err(e) = result {
                assert!(
                    e.contains("does not exist or is not a directory"),
                    "Error message was: {}",
                    e
                );
            }
        }

        #[test]
        fn test_vosk_transcriber_empty_model_path() {
            // Empty model_path should fall back to default_model_path()
            let config = TranscriptionConfig {
                enabled: true,
                model_path: "".to_string(),
                partial_results: true,
                max_alternatives: 1,
                include_words: false,
                buffer_size_ms: 512,
                streaming: false,
            };

            let result = VoskTranscriber::new(config, 16000.0);

            // With an empty model path, `locate_model` should try to find the default.
            // Depending on the test environment, the default model may or may not exist.
            match result {
                Ok(_) => {
                    // This is fine, means a default model was found
                }
                Err(e) => {
                    // This is also fine, means no default model was found
                    assert!(e.contains("Vosk model not found"));
                }
            }
        }

        // Integration test with real model (if available)
        #[test]
        #[ignore] // Run with: cargo test -- --ignored
        fn test_vosk_transcriber_with_model() {
            let model_path = "models/vosk-model-small-en-us-0.15";
            if !std::path::Path::new(model_path).exists() {
                eprintln!("Skipping test: Model not found at {}", model_path);
                return;
            }

            let config = TranscriptionConfig {
                enabled: true,
                model_path: model_path.to_string(),
                partial_results: true,
                max_alternatives: 1,
                include_words: false,
                buffer_size_ms: 512,
                streaming: false,
            };

            let result = VoskTranscriber::new(config.clone(), 16000.0);
            assert!(result.is_ok());

            let mut transcriber = result.unwrap();

            // Test with silence (should not produce transcription)
            let silence = vec![0i16; 512];
            let event = transcriber.accept_frame(&silence);
            assert!(event.is_ok());

            // Test finalization
            let final_result = transcriber.finalize_utterance();
            assert!(final_result.is_ok());
        }
    }
}

#[cfg(all(test, feature = "vosk"))]
mod processor_tests {
    use crate::stt::processor::*;
    use std::time::Instant;
    use tokio::sync::{broadcast, mpsc};

    #[test]
    fn test_utterance_state_transitions() {
        let idle = UtteranceState::Idle;
        matches!(idle, UtteranceState::Idle);

        let active = UtteranceState::SpeechActive {
            started_at: Instant::now(),
            audio_buffer: Vec::new(),
            frames_buffered: 0,
        };

        match active {
            UtteranceState::SpeechActive {
                frames_buffered, ..
            } => {
                assert_eq!(frames_buffered, 0);
            }
            _ => panic!("Expected SpeechActive state"),
        }
    }

    #[test]
    fn test_stt_metrics_default() {
        let metrics = SttMetrics::default();
        assert_eq!(metrics.frames_in, 0);
        assert_eq!(metrics.frames_out, 0);
        assert_eq!(metrics.frames_dropped, 0);
        assert_eq!(metrics.partial_count, 0);
        assert_eq!(metrics.final_count, 0);
        assert_eq!(metrics.error_count, 0);
        assert_eq!(metrics.queue_depth, 0);
        assert!(metrics.last_event_time.is_none());
    }

}
