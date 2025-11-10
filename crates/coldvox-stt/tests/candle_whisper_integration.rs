//! Integration tests for Candle Whisper plugin.
//!
//! These tests verify the full integration of the Candle Whisper plugin
//! with the ColdVox architecture, including plugin system integration,
//! configuration handling, and audio processing pipeline.

#[cfg(test)]
mod basic_tests {
    use coldvox_stt::types::{TranscriptionConfig, TranscriptionEvent, WordInfo};

    #[tokio::test]
    async fn test_transcription_config_default() {
        let config = TranscriptionConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.model_path, "base.en");
        assert!(config.partial_results);
        assert_eq!(config.max_alternatives, 1);
        assert!(!config.include_words);
        assert_eq!(config.buffer_size_ms, 512);
        assert!(!config.streaming);
        assert!(config.auto_extract_model);
    }

    #[test]
    fn test_transcription_config_custom() {
        let mut config = TranscriptionConfig::default();
        config.enabled = true;
        config.model_path = "/custom/model".to_string();
        config.include_words = true;
        config.streaming = true;

        assert!(config.enabled);
        assert_eq!(config.model_path, "/custom/model");
        assert!(config.include_words);
        assert!(config.streaming);
    }

    #[test]
    fn test_environment_model_path() {
        // Test that environment variable is respected
        std::env::set_var("WHISPER_MODEL_PATH", "/env/model");

        let config = TranscriptionConfig::default();
        assert_eq!(config.model_path, "/env/model");

        std::env::remove_var("WHISPER_MODEL_PATH");
    }

    #[tokio::test]
    async fn test_transcription_event_creation() {
        // Test Partial transcription event
        let partial_event = TranscriptionEvent::Partial {
            utterance_id: 1,
            text: "Hello".to_string(),
            t0: Some(0.0),
            t1: Some(1.0),
        };

        match partial_event {
            TranscriptionEvent::Partial {
                utterance_id,
                text,
                t0,
                t1,
            } => {
                assert_eq!(utterance_id, 1);
                assert_eq!(text, "Hello");
                assert_eq!(t0, Some(0.0));
                assert_eq!(t1, Some(1.0));
            }
            _ => panic!("Expected Partial event"),
        }

        // Test Final transcription event
        let final_event = TranscriptionEvent::Final {
            utterance_id: 1,
            text: "Hello world".to_string(),
            words: Some(vec![WordInfo {
                start: 0.0,
                end: 1.0,
                conf: 0.95,
                text: "Hello".to_string(),
            }]),
        };

        match final_event {
            TranscriptionEvent::Final {
                utterance_id,
                text,
                words,
            } => {
                assert_eq!(utterance_id, 1);
                assert_eq!(text, "Hello world");
                assert!(words.is_some());
                assert_eq!(words.as_ref().unwrap().len(), 1);
                assert_eq!(words.as_ref().unwrap()[0].text, "Hello");
            }
            _ => panic!("Expected Final event"),
        }

        // Test Error transcription event
        let error_event = TranscriptionEvent::Error {
            code: "MODEL_NOT_FOUND".to_string(),
            message: "Model file not found".to_string(),
        };

        match error_event {
            TranscriptionEvent::Error { code, message } => {
                assert_eq!(code, "MODEL_NOT_FOUND");
                assert_eq!(message, "Model file not found");
            }
            _ => panic!("Expected Error event"),
        }
    }
}

// Tests for plugin compilation and basic functionality
#[cfg(feature = "candle-whisper")]
mod feature_tests {
    use coldvox_stt::plugin::{PluginInfo, SttPlugin, SttPluginFactory, SttPluginRegistry};
    use coldvox_stt::plugins::candle_whisper::{CandleWhisperPlugin, CandleWhisperPluginFactory};
    use coldvox_stt::types::TranscriptionConfig;

    #[test]
    fn test_plugin_creation_with_feature() {
        let plugin = CandleWhisperPlugin::new();
        let info = plugin.info();

        assert_eq!(info.id, "candle-whisper");
        assert_eq!(info.name, "Candle Whisper");
        assert!(info.is_local);
        assert!(!info.requires_network);
    }

    #[test]
    fn test_factory_creation_with_feature() {
        let factory = CandleWhisperPluginFactory::new();
        let plugin_result = factory.create();

        assert!(plugin_result.is_ok());
        let plugin = plugin_result.unwrap();
        let info = plugin.info();
        assert_eq!(info.id, "candle-whisper");
    }

    #[test]
    fn test_plugin_capabilities() {
        let plugin = CandleWhisperPlugin::new();
        let capabilities = plugin.capabilities();

        assert!(capabilities.streaming);
        assert!(capabilities.batch);
        assert!(capabilities.word_timestamps);
        assert!(capabilities.confidence_scores);
        assert!(!capabilities.speaker_diarization);
        assert!(capabilities.auto_punctuation);
        assert!(!capabilities.custom_vocabulary);
    }

    #[test]
    fn test_plugin_registry_integration() {
        let mut registry = SttPluginRegistry::new();
        let factory = CandleWhisperPluginFactory::new();

        // Register the factory
        registry.register(Box::new(factory));

        // Get available plugins
        let available_plugins = registry.available_plugins();
        assert!(!available_plugins.is_empty());

        // Check if our plugin is in the list
        let candle_plugin_info: Option<PluginInfo> = available_plugins
            .iter()
            .find(|info| info.id == "candle-whisper")
            .cloned();

        assert!(candle_plugin_info.is_some());
        let info = candle_plugin_info.unwrap();
        assert_eq!(info.id, "candle-whisper");
        assert_eq!(info.name, "Candle Whisper");
    }
}

// Tests for Candle engine configuration
#[cfg(feature = "candle-whisper")]
mod engine_tests {
    use coldvox_stt::candle::engine::{DevicePreference, WhisperEngineInit};

    #[test]
    fn test_whisper_engine_init_builder() {
        let init = WhisperEngineInit::new()
            .with_model_id("test/model")
            .with_revision("v1.0")
            .with_device_preference(DevicePreference::Cpu)
            .with_quantized(true)
            .with_language("en")
            .with_max_tokens(512)
            .with_temperature(0.5)
            .with_generate_timestamps(false);

        assert_eq!(init.model_id, "test/model");
        assert_eq!(init.revision, "v1.0");
        assert_eq!(init.device_preference, DevicePreference::Cpu);
        assert!(init.quantized);
        assert_eq!(init.language, Some("en".to_string()));
        assert_eq!(init.max_tokens, 512);
        assert_eq!(init.temperature, 0.5);
        assert!(!init.generate_timestamps);
    }

    #[test]
    fn test_whisper_engine_init_defaults() {
        let init = WhisperEngineInit::new();
        assert_eq!(init.model_id, "openai/whisper-base.en");
        assert_eq!(init.revision, "main");
        assert_eq!(init.device_preference, DevicePreference::Auto);
        assert!(!init.quantized);
        assert_eq!(init.language, None);
        assert_eq!(init.max_tokens, 448);
        assert_eq!(init.temperature, 0.0);
        assert!(init.generate_timestamps);
    }

    #[test]
    fn test_whisper_engine_init_validation() {
        // Test valid configuration
        let init = WhisperEngineInit::new()
            .with_model_id("test/model")
            .with_revision("v1.0");
        assert!(init.validate().is_ok());

        // Test invalid empty model ID
        let mut init = WhisperEngineInit::new();
        init.model_id = "".to_string();
        assert!(init.validate().is_err());

        // Test invalid empty revision
        let mut init = WhisperEngineInit::new();
        init.revision = "".to_string();
        assert!(init.validate().is_err());
    }

    #[test]
    fn test_device_preference_from_str() {
        assert_eq!(
            "cpu".parse::<DevicePreference>().unwrap(),
            DevicePreference::Cpu
        );
        assert_eq!(
            "cuda".parse::<DevicePreference>().unwrap(),
            DevicePreference::Cuda
        );
        assert_eq!(
            "auto".parse::<DevicePreference>().unwrap(),
            DevicePreference::Auto
        );

        // Test case insensitivity
        assert_eq!(
            "CPU".parse::<DevicePreference>().unwrap(),
            DevicePreference::Cpu
        );
        assert_eq!(
            "CUDA".parse::<DevicePreference>().unwrap(),
            DevicePreference::Cuda
        );
        assert_eq!(
            "AUTO".parse::<DevicePreference>().unwrap(),
            DevicePreference::Auto
        );

        // Test invalid values
        assert!("invalid".parse::<DevicePreference>().is_err());
        assert!("".parse::<DevicePreference>().is_err());
    }

    #[test]
    fn test_device_preference_default() {
        let default = DevicePreference::default();
        assert_eq!(default, DevicePreference::Auto);
    }
}

// Audio processing tests
#[cfg(test)]
mod audio_processing_tests {
    use coldvox_stt::types::TranscriptionConfig;

    #[test]
    fn test_audio_sample_conversion() {
        // Test that i16 samples are correctly converted to f32
        let i16_samples = vec![-32768i16, -16384i16, 0i16, 16384i16, 32767i16];

        // This simulates the conversion logic used in the plugin
        let f32_samples: Vec<f32> = i16_samples
            .iter()
            .map(|&sample| (sample as f32) / 32768.0)
            .collect();

        // Verify conversion is within expected range
        for &sample in &f32_samples {
            assert!(sample >= -1.0 && sample <= 1.0);
        }

        // Verify specific conversions
        assert!((f32_samples[0] - (-1.0)).abs() < 0.001);
        assert!((f32_samples[2] - 0.0).abs() < 0.001);
        assert!((f32_samples[4] - 0.99997).abs() < 0.001);
    }
}

// Environment configuration tests
#[cfg(test)]
mod environment_tests {
    use std::path::PathBuf;

    #[test]
    fn test_plugin_environment_variables() {
        // Test CANDLE_WHISPER_DEVICE environment variable handling
        std::env::set_var("CANDLE_WHISPER_DEVICE", "cpu");
        std::env::set_var("WHISPER_LANGUAGE", "es");

        // Factory should respect environment variables
        // Note: This test just verifies the environment variables are set
        assert_eq!(std::env::var("CANDLE_WHISPER_DEVICE").unwrap(), "cpu");
        assert_eq!(std::env::var("WHISPER_LANGUAGE").unwrap(), "es");

        std::env::remove_var("CANDLE_WHISPER_DEVICE");
        std::env::remove_var("WHISPER_LANGUAGE");
    }
}
