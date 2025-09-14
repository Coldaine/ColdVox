//! Tests for TTS functionality

#[cfg(test)]
mod tests {
    use crate::EspeakEngine;
    use coldvox_tts::{TtsConfig, SynthesisOptions, TtsEngine};

    #[tokio::test]
    async fn test_espeak_engine_creation() {
        let engine = EspeakEngine::new();
        assert_eq!(engine.name(), "eSpeak");
        assert!(!engine.version().is_empty());
    }

    #[tokio::test]
    async fn test_espeak_availability() {
        let engine = EspeakEngine::new();
        // This test will pass regardless of whether eSpeak is actually installed
        // since we can't guarantee the test environment has eSpeak
        let _is_available = engine.is_available().await;
        // Just ensure the method doesn't panic
    }

    #[test]
    fn test_tts_config_default() {
        let config = TtsConfig::default();
        assert!(config.enabled);
        assert_eq!(config.speech_rate, Some(180));
        assert_eq!(config.pitch, Some(1.0));
        assert_eq!(config.volume, Some(0.8));
    }

    #[test]
    fn test_synthesis_options_default() {
        let options = SynthesisOptions::default();
        assert!(options.voice.is_none());
        assert!(options.speech_rate.is_none());
        assert!(!options.high_priority);
    }

    #[tokio::test]
    async fn test_espeak_engine_lifecycle() {
        let mut engine = EspeakEngine::new();
        
        // Test shutdown without initialization (should not panic)
        let result = engine.shutdown().await;
        assert!(result.is_ok());
        
        // Test configuration access
        let _config = engine.config();
    }

    #[test]
    fn test_voice_info_creation() {
        use std::collections::HashMap;
        use coldvox_tts::VoiceGender;
        
        let voice = coldvox_tts::VoiceInfo {
            id: "en-us".to_string(),
            name: "English (US)".to_string(),
            language: "en-US".to_string(),
            gender: Some(VoiceGender::Female),
            age: None,
            properties: HashMap::new(),
        };
        
        assert_eq!(voice.id, "en-us");
        assert_eq!(voice.language, "en-US");
        assert!(matches!(voice.gender, Some(VoiceGender::Female)));
    }
}