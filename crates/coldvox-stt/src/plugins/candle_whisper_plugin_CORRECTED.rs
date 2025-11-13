//! Candle Whisper plugin implementation - CORRECTED VERSION
//!
//! This plugin wraps the Candle-based WhisperEngine and adapts it to the SttPlugin interface.

use async_trait::async_trait;
use parking_lot::Mutex;
use std::path::Path;
use std::sync::Arc;

use crate::candle::{TranscribeOptions, WhisperDevice, WhisperEngine, WhisperEngineInit, WhisperTask};
use crate::plugin::{PluginCapabilities, PluginInfo, SttPlugin};
use crate::types::{TranscriptionConfig, TranscriptionEvent, WordInfo};
use coldvox_foundation::error::{ColdVoxError, SttError};

/// Configuration for the Candle Whisper plugin
#[derive(Debug, Clone)]
pub struct CandleWhisperConfig {
    /// Path to model or HuggingFace model ID
    pub model_id: String,
    /// Whether to use quantized model
    pub quantized: bool,
    /// Device to use (CPU or CUDA)
    pub device: WhisperDevice,
    /// Language code (e.g., "en", "es"). None for auto-detection.
    pub language: Option<String>,
    /// Temperature for sampling (0.0 = greedy)
    pub temperature: f32,
    /// Enable word-level timestamps
    pub enable_timestamps: bool,
}

impl Default for CandleWhisperConfig {
    fn default() -> Self {
        Self {
            model_id: "openai/whisper-base".to_string(),
            quantized: false,
            device: WhisperDevice::Cpu,
            language: Some("en".to_string()),
            temperature: 0.0,
            enable_timestamps: true,
        }
    }
}

/// Candle Whisper plugin - corrected to implement actual SttPlugin trait
#[derive(Debug)]
pub struct CandleWhisperPlugin {
    engine: Option<Arc<WhisperEngine>>,
    config: CandleWhisperConfig,
    audio_buffer: Mutex<Vec<i16>>,
    initialized: bool,
    /// Track next utterance ID for this session
    current_utterance_id: Mutex<u64>,
}

impl CandleWhisperPlugin {
    /// Create a new Candle Whisper plugin (lazy initialization)
    pub fn new(config: CandleWhisperConfig) -> Self {
        Self {
            engine: None,
            config,
            audio_buffer: Mutex::new(Vec::new()),
            initialized: false,
            current_utterance_id: Mutex::new(1),
        }
    }

    /// Get transcription options from config and TranscriptionConfig
    fn get_transcribe_options(&self, transcription_config: &TranscriptionConfig) -> TranscribeOptions {
        TranscribeOptions {
            language: transcription_config.language.clone()
                .or_else(|| self.config.language.clone()),
            task: WhisperTask::Transcribe,
            temperature: self.config.temperature,
            enable_timestamps: self.config.enable_timestamps,
        }
    }

    /// Initialize the engine (called during initialize())
    fn init_engine(&mut self) -> Result<(), ColdVoxError> {
        if self.engine.is_some() {
            return Ok(());
        }

        tracing::info!("Initializing Candle Whisper engine with model: {}", self.config.model_id);

        let engine = WhisperEngine::from_model_id(
            &self.config.model_id,
            self.config.quantized,
            self.config.device.clone(),
        )
        .map_err(|e| ColdVoxError::from(SttError::InitializationFailed {
            plugin: "candle-whisper".to_string(),
            reason: format!("Failed to initialize Whisper engine: {}", e),
        }))?;

        self.engine = Some(Arc::new(engine));
        Ok(())
    }

    fn next_utterance_id(&self) -> u64 {
        let mut id = self.current_utterance_id.lock();
        let current = *id;
        *id += 1;
        current
    }
}

#[async_trait]
impl SttPlugin for CandleWhisperPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: "candle-whisper".to_string(),
            name: "Candle Whisper STT".to_string(),
            description: "Pure Rust Whisper implementation using Candle ML framework".to_string(),
            requires_network: false, // Can work offline once model is downloaded
            is_local: true,
            is_available: true, // TODO: Check if model exists
            supported_languages: vec![
                "en", "es", "fr", "de", "it", "pt", "nl", "pl", "ru", "zh", "ja", "ko"
            ].iter().map(|s| s.to_string()).collect(),
            memory_usage_mb: Some(if self.config.quantized { 150 } else { 500 }), // Approximate
        }
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            streaming: false, // Whisper is batch-based
            batch: true,
            word_timestamps: self.config.enable_timestamps,
            confidence_scores: true,
            speaker_diarization: false,
            auto_punctuation: true, // Whisper includes punctuation
            custom_vocabulary: false,
        }
    }

    async fn is_available(&self) -> Result<bool, ColdVoxError> {
        // TODO: Check if model exists or can be downloaded
        Ok(true)
    }

    async fn initialize(&mut self, config: TranscriptionConfig) -> Result<(), ColdVoxError> {
        if self.initialized {
            return Ok(());
        }

        // Initialize engine
        self.init_engine()?;

        self.initialized = true;
        tracing::info!("Candle Whisper plugin initialized successfully");
        Ok(())
    }

    async fn process_audio(
        &mut self,
        samples: &[i16],
    ) -> Result<Option<TranscriptionEvent>, ColdVoxError> {
        // Buffer audio for later transcription
        let mut buffer = self.audio_buffer.lock();
        buffer.extend_from_slice(samples);

        // Return None - we process on finalize (batch mode)
        Ok(None)
    }

    async fn finalize(&mut self) -> Result<Option<TranscriptionEvent>, ColdVoxError> {
        let mut buffer = self.audio_buffer.lock();

        if buffer.is_empty() {
            return Ok(None);
        }

        tracing::debug!("Finalizing transcription with {} samples", buffer.len());

        let engine = self.engine.as_ref()
            .ok_or_else(|| ColdVoxError::from(SttError::NotInitialized {
                plugin: "candle-whisper".to_string(),
            }))?;

        // Get transcription options (using default TranscriptionConfig for now)
        let transcription_config = TranscriptionConfig::default();
        let opts = self.get_transcribe_options(&transcription_config);

        // Transcribe buffered audio
        let transcript = engine
            .transcribe_pcm16(&buffer, &opts)
            .map_err(|e| ColdVoxError::from(SttError::TranscriptionFailed {
                plugin: "candle-whisper".to_string(),
                reason: format!("Transcription failed: {}", e),
            }))?;

        // Clear buffer
        buffer.clear();
        drop(buffer);

        if transcript.text.is_empty() {
            return Ok(None);
        }

        // Convert to TranscriptionEvent
        let words = if self.config.enable_timestamps {
            Some(
                transcript
                    .segments
                    .iter()
                    .map(|seg| WordInfo {
                        word: seg.text.clone(),
                        start: seg.start_seconds,
                        end: seg.end_seconds,
                        probability: seg.avg_logprob.exp(), // Convert log prob to probability
                    })
                    .collect(),
            )
        } else {
            None
        };

        let utterance_id = self.next_utterance_id();

        Ok(Some(TranscriptionEvent::Final {
            text: transcript.text,
            utterance_id,
            words,
        }))
    }

    async fn reset(&mut self) -> Result<(), ColdVoxError> {
        let mut buffer = self.audio_buffer.lock();
        buffer.clear();
        Ok(())
    }

    async fn load_model(&mut self, _model_path: Option<&Path>) -> Result<(), ColdVoxError> {
        // Model loading happens during initialize()
        self.init_engine()
    }

    async fn unload(&mut self) -> Result<(), ColdVoxError> {
        self.engine = None;
        self.initialized = false;
        tracing::info!("Candle Whisper plugin unloaded");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CandleWhisperConfig::default();
        assert_eq!(config.model_id, "openai/whisper-base");
        assert!(!config.quantized);
        assert_eq!(config.temperature, 0.0);
    }

    #[test]
    fn test_plugin_info() {
        let plugin = CandleWhisperPlugin::new(CandleWhisperConfig::default());
        let info = plugin.info();
        assert_eq!(info.id, "candle-whisper");
        assert!(!info.requires_network);
        assert!(info.is_local);
    }

    #[test]
    fn test_plugin_capabilities() {
        let plugin = CandleWhisperPlugin::new(CandleWhisperConfig::default());
        let caps = plugin.capabilities();
        assert!(!caps.streaming); // Whisper is batch-based
        assert!(caps.batch);
        assert!(caps.word_timestamps);
    }
}
