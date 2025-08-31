#[cfg(test)]
pub mod end_to_end_wav;

#[cfg(test)]
mod vosk_tests {
    use super::super::*;

    #[test]
    fn test_transcription_config_default() {
        let config = TranscriptionConfig::default();
        assert_eq!(config.enabled, false);
        assert_eq!(config.model_path, "");
        assert_eq!(config.partial_results, true);
        assert_eq!(config.max_alternatives, 1);
        assert_eq!(config.include_words, false);
        assert_eq!(config.buffer_size_ms, 512);
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
        use super::super::super::vosk::VoskTranscriber;
        use super::super::super::TranscriptionConfig;
        
        #[test]
        fn test_vosk_transcriber_missing_model() {
            let config = TranscriptionConfig {
                enabled: true,
                model_path: "/nonexistent/model/path".to_string(),
                partial_results: true,
                max_alternatives: 1,
                include_words: false,
                buffer_size_ms: 512,
            };
            
            let result = VoskTranscriber::new(config, 16000.0);
            assert!(result.is_err());
            if let Err(e) = result {
                assert!(e.contains("not found"));
            }
        }

        #[test]
        fn test_vosk_transcriber_empty_model_path() {
            let config = TranscriptionConfig {
                enabled: true,
                model_path: "".to_string(),
                partial_results: true,
                max_alternatives: 1,
                include_words: false,
                buffer_size_ms: 512,
            };
            
            let result = VoskTranscriber::new(config, 16000.0);
            assert!(result.is_err());
            if let Err(e) = result {
                assert!(e.contains("Model path is required"));
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
            UtteranceState::SpeechActive { frames_buffered, .. } => {
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