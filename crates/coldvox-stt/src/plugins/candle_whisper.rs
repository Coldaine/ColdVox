//! Candle (Rust Whisper) speech-to-text plugin implementation.
//!
//! This plugin provides a local transcription backend powered by the
//! pure Rust Candle implementation of OpenAI's Whisper model. It provides
//! full local processing without Python dependencies, GPU acceleration
//! support, and comprehensive audio processing capabilities.

use crate::plugin::*;
use crate::types::{TranscriptionConfig, TranscriptionEvent};
use crate::candle::engine::{WhisperEngine, WhisperEngineInit, DevicePreference};
use crate::candle::types::Transcript;
use async_trait::async_trait;
use coldvox_foundation::error::{ColdVoxError, SttError};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tracing::{debug, info, warn};

/// Candle Whisper-based STT plugin.
#[derive(Debug)]
pub struct CandleWhisperPlugin {
    engine: Option<WhisperEngine>,
    model_path: Option<PathBuf>,
    device_preference: DevicePreference,
    language: Option<String>,
    initialized: bool,
    audio_buffer: Arc<Mutex<Vec<f32>>>,
    active_config: Option<TranscriptionConfig>,
}

impl CandleWhisperPlugin {
    pub fn new() -> Self {
        Self {
            engine: None,
            model_path: None,
            device_preference: DevicePreference::Auto,
            language: None,
            initialized: false,
            audio_buffer: Arc::new(Mutex::new(Vec::new())),
            active_config: None,
        }
    }

    pub fn with_model_path(mut self, path: PathBuf) -> Self {
        self.model_path = Some(path);
        self
    }

    pub fn with_device_preference(mut self, preference: DevicePreference) -> Self {
        self.device_preference = preference;
        self
    }

    pub fn with_language(mut self, language: String) -> Self {
        self.language = Some(language);
        self
    }

    /// Convert TranscriptionConfig to WhisperEngineInit
    fn build_engine_init(&self, config: &TranscriptionConfig) -> WhisperEngineInit {
        let model_id = self.resolve_model_identifier(config);
        
        let mut init = WhisperEngineInit::new()
            .with_model_id(model_id)
            .with_device_preference(self.device_preference)
            .with_language(self.language.clone().unwrap_or_else(|| "en".to_string()))
            .with_max_tokens(448) // Whisper default
            .with_temperature(0.0) // Deterministic output
            .with_generate_timestamps(config.include_words);

        if let Some(ref path) = self.model_path {
            init = init.with_local_path(path);
        }

        // Enable streaming for better real-time experience
        if config.streaming {
            init = init.with_generate_timestamps(true);
        }

        init
    }

    /// Resolve model identifier from configuration
    fn resolve_model_identifier(&self, config: &TranscriptionConfig) -> String {
        if !config.model_path.is_empty() {
            return config.model_path.clone();
        }

        if let Some(ref path) = self.model_path {
            return path.to_string_lossy().to_string();
        }

        // Default model if no path specified
        "openai/whisper-base.en".to_string()
    }

    /// Convert internal transcript to TranscriptionEvent
    fn convert_transcript(&self, transcript: Transcript, utterance_id: u64, include_words: bool) -> TranscriptionEvent {
        if transcript.segments.is_empty() {
            return TranscriptionEvent::Final {
                utterance_id,
                text: String::new(),
                words: None,
            };
        }

        // For now, concatenate all segments into a single text
        let text = transcript.segments
            .iter()
            .map(|segment| segment.text.clone())
            .collect::<Vec<_>>()
            .join(" ");

        // Convert word timings if available and requested
        let words = if include_words {
            let mut word_infos = Vec::new();
            for segment in &transcript.segments {
                if let Some(ref segment_words) = segment.words {
                    for word in segment_words {
                        word_infos.push(crate::types::WordInfo {
                            start: word.start,
                            end: word.end,
                            conf: word.confidence,
                            text: word.text.clone(),
                        });
                    }
                }
            }
            if !word_infos.is_empty() {
                Some(word_infos)
            } else {
                None
            }
        } else {
            None
        };

        TranscriptionEvent::Final {
            utterance_id,
            text,
            words,
        }
    }
}

impl Default for CandleWhisperPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SttPlugin for CandleWhisperPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: "candle-whisper".to_string(),
            name: "Candle Whisper".to_string(),
            description: "Local transcription via pure Rust Candle Whisper implementation".to_string(),
            requires_network: false,
            is_local: true,
            is_available: true, // Always available when feature is enabled
            supported_languages: vec![
                "auto".to_string(),
                "en".to_string(),
                "es".to_string(),
                "fr".to_string(),
                "de".to_string(),
                "it".to_string(),
                "pt".to_string(),
                "ru".to_string(),
                "ja".to_string(),
                "ko".to_string(),
                "zh".to_string(),
            ],
            memory_usage_mb: Some(500), // Approximate for base model
        }
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            streaming: true,
            batch: true,
            word_timestamps: true,
            confidence_scores: true,
            speaker_diarization: false,
            auto_punctuation: true,
            custom_vocabulary: false,
        }
    }

    async fn is_available(&self) -> Result<bool, ColdVoxError> {
        Ok(true) // Always available when feature is enabled
    }

    async fn initialize(&mut self, config: TranscriptionConfig) -> Result<(), ColdVoxError> {
        debug!("Initializing Candle Whisper plugin");
        
        // Build engine configuration
        let init = self.build_engine_init(&config);
        
        debug!(
            "Initializing WhisperEngine with model_id={}, device_preference={:?}",
            init.model_id,
            init.device_preference
        );

        // Initialize the engine
        let engine = WhisperEngine::new(init)
            .map_err(|e| ColdVoxError::Stt(SttError::LoadFailed(e.to_string())))?;

        self.engine = Some(engine);
        self.audio_buffer.lock().unwrap().clear();
        self.active_config = Some(config);
        self.initialized = true;

        info!("Candle Whisper plugin initialized successfully");
        Ok(())
    }

    async fn process_audio(
        &mut self,
        samples: &[i16],
    ) -> Result<Option<TranscriptionEvent>, ColdVoxError> {
        if !self.initialized {
            return Err(ColdVoxError::Stt(SttError::NotAvailable {
                plugin: "candle-whisper".to_string(),
                reason: "Plugin not initialized".to_string(),
            }));
        }

        if samples.is_empty() {
            return Ok(None);
        }

        // Convert i16 samples to f32 and add to buffer
        let float_samples: Vec<f32> = samples
            .iter()
            .map(|&sample| (sample as f32) / 32768.0) // Normalize to [-1.0, 1.0]
            .collect();

        self.audio_buffer.lock().unwrap().extend_from_slice(&float_samples);

        // For streaming mode, we could provide partial results here
        // For now, return None to accumulate audio until finalize
        Ok(None)
    }

    async fn finalize(&mut self) -> Result<Option<TranscriptionEvent>, ColdVoxError> {
        if !self.initialized {
            return Ok(None);
        }

        let audio_buffer = {
            let buffer = self.audio_buffer.lock().unwrap();
            buffer.clone()
        };

        if audio_buffer.is_empty() {
            return Ok(None);
        }

        // Get the engine reference
        let engine = self.engine.as_mut()
            .ok_or_else(|| ColdVoxError::Stt(SttError::TranscriptionFailed(
                "Engine not available".to_string()
            )))?;

        // Transcribe the audio
        let transcript = engine.transcribe(&audio_buffer)
            .map_err(|e| ColdVoxError::Stt(SttError::TranscriptionFailed(e.to_string())))?;

        // Clear the buffer for next session
        self.audio_buffer.lock().unwrap().clear();

        // Get word inclusion setting
        let include_words = self
            .active_config
            .as_ref()
            .map(|cfg| cfg.include_words)
            .unwrap_or(false);

        // Convert to TranscriptionEvent
        Ok(Some(self.convert_transcript(transcript, 0, include_words)))
    }

    async fn reset(&mut self) -> Result<(), ColdVoxError> {
        self.audio_buffer.lock().unwrap().clear();
        Ok(())
    }

    async fn load_model(&mut self, model_path: Option<&Path>) -> Result<(), ColdVoxError> {
        if let Some(path) = model_path {
            self.model_path = Some(path.to_path_buf());
        }
        Ok(())
    }

    async fn unload(&mut self) -> Result<(), ColdVoxError> {
        self.engine = None;
        self.audio_buffer.lock().unwrap().clear();
        self.initialized = false;
        Ok(())
    }
}

/// Factory for creating CandleWhisperPlugin instances.
pub struct CandleWhisperPluginFactory {
    model_path: Option<PathBuf>,
    device_preference: DevicePreference,
    language: Option<String>,
}

impl CandleWhisperPluginFactory {
    pub fn new() -> Self {
        let device_preference = std::env::var("CANDLE_WHISPER_DEVICE")
            .unwrap_or_else(|_| "auto".to_string())
            .parse::<DevicePreference>()
            .unwrap_or_else(|_| {
                warn!("Invalid CANDLE_WHISPER_DEVICE value, using 'auto'");
                DevicePreference::Auto
            });

        Self {
            model_path: std::env::var("WHISPER_MODEL_PATH").ok().map(PathBuf::from),
            device_preference,
            language: std::env::var("WHISPER_LANGUAGE").ok(),
        }
    }

    pub fn with_model_path(mut self, path: PathBuf) -> Self {
        self.model_path = Some(path);
        self
    }

    pub fn with_device_preference(mut self, preference: DevicePreference) -> Self {
        self.device_preference = preference;
        self
    }

    pub fn with_language(mut self, language: String) -> Self {
        self.language = Some(language);
        self
    }
}

impl Default for CandleWhisperPluginFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl SttPluginFactory for CandleWhisperPluginFactory {
    fn create(&self) -> Result<Box<dyn SttPlugin>, ColdVoxError> {
        let mut plugin = CandleWhisperPlugin::new()
            .with_device_preference(self.device_preference);

        if let Some(ref path) = self.model_path {
            plugin = plugin.with_model_path(path.clone());
        }

        if let Some(ref lang) = self.language {
            plugin = plugin.with_language(lang.clone());
        }

        Ok(Box::new(plugin))
    }

    fn plugin_info(&self) -> PluginInfo {
        CandleWhisperPlugin::new()
            .with_device_preference(self.device_preference)
            .info()
    }

    fn check_requirements(&self) -> Result<(), ColdVoxError> {
        // Check if model path exists if specified
        if let Some(ref path) = self.model_path {
            if !path.exists() {
                return Err(ColdVoxError::Stt(SttError::ModelNotFound {
                    path: path.clone(),
                }));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_plugin_creation() {
        let plugin = CandleWhisperPlugin::new();
        assert!(!plugin.initialized);
        assert!(plugin.engine.is_none());
    }

    #[test]
    fn test_factory_creation() {
        let factory = CandleWhisperPluginFactory::new();
        assert_eq!(factory.device_preference, DevicePreference::Auto);
        assert!(factory.model_path.is_none());
    }

    #[test]
    fn test_model_path_handling() {
        let dir = tempdir().unwrap();
        let path = dir.path().to_path_buf();

        let factory = CandleWhisperPluginFactory::new()
            .with_model_path(path.clone());

        assert!(factory.check_requirements().is_ok());
    }

    #[test]
    fn test_non_existent_model_path() {
        let path = std::path::PathBuf::from("/non/existent/path");

        let factory = CandleWhisperPluginFactory::new()
            .with_model_path(path);

        assert!(factory.check_requirements().is_err());
    }

    #[test]
    fn test_plugin_info() {
        let plugin = CandleWhisperPlugin::new();
        let info = plugin.info();
        
        assert_eq!(info.id, "candle-whisper");
        assert_eq!(info.name, "Candle Whisper");
        assert!(info.is_local);
        assert!(!info.requires_network);
        assert!(info.supported_languages.contains(&"en".to_string()));
    }

    #[test]
    fn test_capabilities() {
        let plugin = CandleWhisperPlugin::new();
        let capabilities = plugin.capabilities();
        
        assert!(capabilities.streaming);
        assert!(capabilities.batch);
        assert!(capabilities.word_timestamps);
        assert!(capabilities.confidence_scores);
    }
}