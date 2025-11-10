//! Candle-based Whisper speech-to-text plugin implementation.
//!
//! This plugin provides a local transcription backend powered by the Candle ML framework.
//! It replaces the previous Python-based faster-whisper implementation with a pure-Rust solution.
//!
//! # Memory Management
//!
//! The plugin buffers incoming audio in `audio_buffer` until `finalize()` is called.
//! To prevent unbounded memory growth, the buffer is limited to MAX_AUDIO_BUFFER_SAMPLES.
//! At 16kHz, this represents approximately 10 minutes of audio (9.6 million samples ~= 18MB).
//! If the buffer limit is exceeded, older samples are discarded (ring buffer behavior).

use crate::plugin::*;
use crate::types::{TranscriptionConfig, TranscriptionEvent};
use async_trait::async_trait;
use coldvox_foundation::error::{ColdVoxError, SttError};
use std::path::{Path, PathBuf};
#[allow(unused_imports)]
use tracing::{debug, info, warn};

/// Maximum audio buffer size in samples (16-bit i16 samples at 16kHz)
/// ~10 minutes of audio = 9,600,000 samples = ~18MB of memory
const MAX_AUDIO_BUFFER_SAMPLES: usize = 9_600_000;

#[cfg(feature = "whisper")]
use crate::candle::{
    TranscribeOptions, WhisperDevice, WhisperEngine, WhisperEngineInit, WhisperTask,
};

#[cfg(feature = "whisper")]
use crate::WordInfo;

/// Candle-based Whisper STT plugin
#[derive(Debug)]
pub struct WhisperCandlePlugin {
    model_path: Option<PathBuf>,
    model_size: WhisperModelSize,
    language: Option<String>,
    device: String,
    initialized: bool,
    #[cfg(feature = "whisper")]
    engine: Option<WhisperEngine>,
    #[cfg(feature = "whisper")]
    audio_buffer: Vec<i16>,
    #[cfg(feature = "whisper")]
    active_config: Option<TranscriptionConfig>,
}

impl WhisperCandlePlugin {
    pub fn new() -> Self {
        Self {
            model_path: None,
            model_size: WhisperModelSize::default(),
            language: None,
            device: "cpu".to_string(),
            initialized: false,
            #[cfg(feature = "whisper")]
            engine: None,
            #[cfg(feature = "whisper")]
            audio_buffer: Vec::new(),
            #[cfg(feature = "whisper")]
            active_config: None,
        }
    }

    pub fn with_model_size(mut self, size: WhisperModelSize) -> Self {
        self.model_size = size;
        self
    }

    pub fn with_language(mut self, language: String) -> Self {
        self.language = Some(language);
        self
    }

    pub fn with_model_path(mut self, path: PathBuf) -> Self {
        self.model_path = Some(path);
        self
    }

    pub fn with_device<S: Into<String>>(mut self, device: S) -> Self {
        self.device = device.into();
        self
    }

    #[cfg(feature = "whisper")]
    fn parse_device(&self) -> WhisperDevice {
        if self.device.starts_with("cuda") {
            // Parse "cuda" or "cuda:0" format
            if let Some(idx_str) = self.device.strip_prefix("cuda:") {
                if let Ok(idx) = idx_str.parse::<usize>() {
                    return WhisperDevice::Cuda(idx);
                }
            }
            WhisperDevice::Cuda(0)
        } else if self.device == "metal" {
            WhisperDevice::Metal
        } else {
            WhisperDevice::Cpu
        }
    }

    #[cfg(feature = "whisper")]
    fn resolve_model_paths(&self) -> Result<(PathBuf, PathBuf, PathBuf), ColdVoxError> {
        // For now, use the model_path if provided
        // TODO: Implement model downloading from HuggingFace Hub
        let model_dir = self.model_path.clone().ok_or_else(|| {
            SttError::LoadFailed(
                "Model path not specified. Set WHISPER_MODEL_PATH or use with_model_path()".to_string()
            )
        })?;

        let model_file = if model_dir.is_dir() {
            model_dir.join("model.safetensors")
        } else {
            model_dir.clone()
        };

        let tokenizer_path = if model_dir.is_dir() {
            model_dir.join("tokenizer.json")
        } else {
            model_dir
                .parent()
                .ok_or_else(|| SttError::LoadFailed("Invalid model path".to_string()))?
                .join("tokenizer.json")
        };

        let config_path = if model_dir.is_dir() {
            model_dir.join("config.json")
        } else {
            model_dir
                .parent()
                .ok_or_else(|| SttError::LoadFailed("Invalid model path".to_string()))?
                .join("config.json")
        };

        Ok((model_file, tokenizer_path, config_path))
    }
}

impl Default for WhisperCandlePlugin {
    fn default() -> Self {
        Self::new()
    }
}

/// Available Whisper model sizes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WhisperModelSize {
    Tiny,
    #[default]
    Base,
    Small,
    Medium,
    Large,
    LargeV2,
    LargeV3,
}

impl WhisperModelSize {
    fn memory_usage_mb(&self) -> u32 {
        match self {
            Self::Tiny => 100,
            Self::Base => 200,
            Self::Small => 500,
            Self::Medium => 1500,
            Self::Large | Self::LargeV2 | Self::LargeV3 => 3000,
        }
    }
}

#[async_trait]
impl SttPlugin for WhisperCandlePlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: "whisper-candle".to_string(),
            name: "Whisper (Candle)".to_string(),
            description: "Local Rust-based transcription via Candle Whisper".to_string(),
            requires_network: false,
            is_local: true,
            is_available: cfg!(feature = "whisper"),
            supported_languages: vec!["auto".to_string(), "en".to_string()],
            memory_usage_mb: Some(self.model_size.memory_usage_mb()),
        }
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            streaming: false,
            batch: true,
            word_timestamps: false, // TODO: Implement word-level timestamps
            confidence_scores: true,
            speaker_diarization: false,
            auto_punctuation: true,
            custom_vocabulary: false,
        }
    }

    async fn is_available(&self) -> Result<bool, ColdVoxError> {
        Ok(cfg!(feature = "whisper"))
    }

    async fn initialize(&mut self, config: TranscriptionConfig) -> Result<(), ColdVoxError> {
        #[cfg(feature = "whisper")]
        {
            let (model_path, tokenizer_path, config_path) = self.resolve_model_paths()?;

            debug!(
                target: "coldvox::stt::whisper_candle",
                model = %model_path.display(),
                device = %self.device,
                "Initializing Candle Whisper engine"
            );

            let init = WhisperEngineInit {
                model_path,
                tokenizer_path,
                config_path,
                quantized: false, // TODO: Support quantized models
                device: self.parse_device(),
            };

            let engine = WhisperEngine::new(init)
                .map_err(|e| SttError::LoadFailed(format!("Failed to initialize Whisper engine: {}", e)))?;

            self.engine = Some(engine);
            self.audio_buffer.clear();
            self.active_config = Some(config);
            self.initialized = true;

            info!(
                target: "coldvox::stt::whisper_candle",
                "Candle Whisper plugin initialized successfully"
            );

            Ok(())
        }

        #[cfg(not(feature = "whisper"))]
        {
            let _ = config;
            Err(SttError::NotAvailable {
                plugin: "whisper-candle".to_string(),
                reason: "Whisper feature not compiled".to_string(),
            }
            .into())
        }
    }

    async fn process_audio(
        &mut self,
        samples: &[i16],
    ) -> Result<Option<TranscriptionEvent>, ColdVoxError> {
        #[cfg(feature = "whisper")]
        {
            if !self.initialized {
                return Err(SttError::NotAvailable {
                    plugin: "whisper-candle".to_string(),
                    reason: "Plugin not initialized".to_string(),
                }
                .into());
            }

            // Buffer audio for batch processing with size limit
            // If adding these samples would exceed the limit, keep only the most recent samples
            let new_total = self.audio_buffer.len() + samples.len();
            if new_total > MAX_AUDIO_BUFFER_SAMPLES {
                let overflow = new_total - MAX_AUDIO_BUFFER_SAMPLES;
                // Remove oldest samples to make room
                if overflow >= self.audio_buffer.len() {
                    // If the new samples alone exceed the limit, take only the last MAX_AUDIO_BUFFER_SAMPLES
                    self.audio_buffer.clear();
                    let start = samples.len().saturating_sub(MAX_AUDIO_BUFFER_SAMPLES);
                    self.audio_buffer.extend_from_slice(&samples[start..]);
                } else {
                    // Remove overflow amount from the beginning
                    self.audio_buffer.drain(..overflow);
                    self.audio_buffer.extend_from_slice(samples);
                }
                warn!(
                    target: "coldvox::stt::whisper_candle",
                    discarded_samples = overflow,
                    "Audio buffer size limit reached, discarding oldest samples"
                );
            } else {
                self.audio_buffer.extend_from_slice(samples);
            }
            Ok(None)
        }

        #[cfg(not(feature = "whisper"))]
        {
            let _ = samples;
            Err(SttError::NotAvailable {
                plugin: "whisper-candle".to_string(),
                reason: "Whisper feature not compiled".to_string(),
            }
            .into())
        }
    }

    async fn finalize(&mut self) -> Result<Option<TranscriptionEvent>, ColdVoxError> {
        #[cfg(feature = "whisper")]
        {
            if !self.initialized || self.audio_buffer.is_empty() {
                return Ok(None);
            }

            let engine = self
                .engine
                .as_ref()
                .ok_or_else(|| SttError::TranscriptionFailed("Engine not initialized".to_string()))?;

            debug!(
                target: "coldvox::stt::whisper_candle",
                samples = self.audio_buffer.len(),
                "Transcribing audio"
            );

            // Prepare transcription options
            let opts = TranscribeOptions {
                language: self.language.clone(),
                task: WhisperTask::Transcribe,
                temperature: 0.0, // Greedy decoding
                enable_timestamps: self
                    .active_config
                    .as_ref()
                    .map(|cfg| cfg.include_words)
                    .unwrap_or(false),
            };

            // Transcribe
            let transcript = engine
                .transcribe_pcm16(&self.audio_buffer, &opts)
                .map_err(|e| SttError::TranscriptionFailed(format!("Transcription failed: {}", e)))?;

            // Convert to full text
            let text = transcript
                .segments
                .iter()
                .map(|s| s.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");

            // Convert segments to word info if timestamps are enabled
            //
            // NAMING INCONSISTENCY: Segments vs Words
            // - The Candle implementation produces "segments" (sentence-level)
            // - WordInfo is named "words" suggesting word-level granularity
            // - This is a deliberate compromise: word-level timestamps require
            //   additional processing that's not yet implemented
            // - For now, we treat segments as "pseudo-words" to maintain API compatibility
            //
            // TODO: Implement true word-level timestamps by:
            // 1. Analyzing attention weights to align tokens to audio frames
            // 2. Merging subword tokens (BPE) into full words
            // 3. Detecting word boundaries in the token sequence
            let words = if opts.enable_timestamps && !transcript.segments.is_empty() {
                Some(
                    transcript
                        .segments
                        .iter()
                        .map(|seg| WordInfo {
                            start: seg.start_seconds,
                            end: seg.end_seconds,
                            conf: (1.0 - seg.no_speech_prob).clamp(0.0, 1.0),
                            text: seg.text.clone(),
                        })
                        .collect(),
                )
            } else {
                None
            };

            self.audio_buffer.clear();

            // UTTERANCE_ID: Always 0
            // TODO: Implement proper utterance tracking
            // - Should increment for each finalize() call
            // - Useful for distinguishing multiple utterances in a session
            // - Currently hardcoded to 0 as a placeholder
            Ok(Some(TranscriptionEvent::Final {
                utterance_id: 0,
                text,
                words,
            }))
        }

        #[cfg(not(feature = "whisper"))]
        {
            Err(SttError::NotAvailable {
                plugin: "whisper-candle".to_string(),
                reason: "Whisper feature not compiled".to_string(),
            }
            .into())
        }
    }

    async fn reset(&mut self) -> Result<(), ColdVoxError> {
        #[cfg(feature = "whisper")]
        {
            self.audio_buffer.clear();
            Ok(())
        }

        #[cfg(not(feature = "whisper"))]
        {
            Err(SttError::NotAvailable {
                plugin: "whisper-candle".to_string(),
                reason: "Whisper feature not compiled".to_string(),
            }
            .into())
        }
    }

    async fn load_model(&mut self, model_path: Option<&Path>) -> Result<(), ColdVoxError> {
        if let Some(path) = model_path {
            self.model_path = Some(path.to_path_buf());
        }
        Ok(())
    }

    async fn unload(&mut self) -> Result<(), ColdVoxError> {
        #[cfg(feature = "whisper")]
        {
            self.engine = None;
            self.audio_buffer.clear();
            self.initialized = false;
            Ok(())
        }

        #[cfg(not(feature = "whisper"))]
        {
            Err(SttError::NotAvailable {
                plugin: "whisper-candle".to_string(),
                reason: "Whisper feature not compiled".to_string(),
            }
            .into())
        }
    }
}

/// Factory for creating WhisperCandlePlugin instances
pub struct WhisperCandlePluginFactory {
    model_path: Option<PathBuf>,
    model_size: WhisperModelSize,
    language: Option<String>,
    device: String,
}

impl WhisperCandlePluginFactory {
    pub fn new() -> Self {
        Self {
            model_path: std::env::var("WHISPER_MODEL_PATH").ok().map(PathBuf::from),
            model_size: WhisperModelSize::default(),
            language: std::env::var("WHISPER_LANGUAGE")
                .ok()
                .or(Some("en".to_string())),
            device: std::env::var("WHISPER_DEVICE").unwrap_or_else(|_| "cpu".to_string()),
        }
    }

    pub fn with_model_path(mut self, path: PathBuf) -> Self {
        self.model_path = Some(path);
        self
    }

    pub fn with_model_size(mut self, size: WhisperModelSize) -> Self {
        self.model_size = size;
        self
    }

    pub fn with_language(mut self, language: String) -> Self {
        self.language = Some(language);
        self
    }

    pub fn with_device<S: Into<String>>(mut self, device: S) -> Self {
        self.device = device.into();
        self
    }
}

impl Default for WhisperCandlePluginFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl SttPluginFactory for WhisperCandlePluginFactory {
    fn create(&self) -> Result<Box<dyn SttPlugin>, ColdVoxError> {
        let mut plugin = WhisperCandlePlugin::new()
            .with_model_size(self.model_size)
            .with_device(self.device.clone());

        if let Some(ref path) = self.model_path {
            plugin = plugin.with_model_path(path.clone());
        }

        if let Some(ref lang) = self.language {
            plugin = plugin.with_language(lang.clone());
        }

        Ok(Box::new(plugin))
    }

    fn plugin_info(&self) -> PluginInfo {
        WhisperCandlePlugin::new()
            .with_model_size(self.model_size)
            .info()
    }

    fn check_requirements(&self) -> Result<(), ColdVoxError> {
        #[cfg(not(feature = "whisper"))]
        {
            return Err(SttError::NotAvailable {
                plugin: "whisper-candle".to_string(),
                reason: "Whisper feature not compiled".to_string(),
            }
            .into());
        }

        #[cfg(feature = "whisper")]
        {
            if let Some(ref path) = self.model_path {
                if !path.exists() {
                    return Err(SttError::ModelNotFound { path: path.clone() }.into());
                }
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_max_audio_buffer_constant() {
        // Verify buffer limit is reasonable (10 minutes at 16kHz)
        assert_eq!(MAX_AUDIO_BUFFER_SAMPLES, 9_600_000);
        assert_eq!(MAX_AUDIO_BUFFER_SAMPLES / 16000, 600); // 600 seconds = 10 minutes
    }

    #[test]
    fn test_whisper_model_size_memory() {
        assert_eq!(WhisperModelSize::Tiny.memory_usage_mb(), 100);
        assert_eq!(WhisperModelSize::Base.memory_usage_mb(), 200);
        assert_eq!(WhisperModelSize::Small.memory_usage_mb(), 500);
        assert_eq!(WhisperModelSize::Medium.memory_usage_mb(), 1500);
        assert_eq!(WhisperModelSize::Large.memory_usage_mb(), 3000);
        assert_eq!(WhisperModelSize::LargeV2.memory_usage_mb(), 3000);
        assert_eq!(WhisperModelSize::LargeV3.memory_usage_mb(), 3000);
    }

    #[test]
    fn test_whisper_model_size_default() {
        assert_eq!(WhisperModelSize::default(), WhisperModelSize::Base);
    }

    #[test]
    fn test_plugin_default() {
        let plugin = WhisperCandlePlugin::default();
        assert!(!plugin.initialized);
        assert_eq!(plugin.device, "cpu");
        assert_eq!(plugin.model_size, WhisperModelSize::Base);
    }

    #[test]
    fn test_plugin_builder() {
        use std::path::PathBuf;

        let plugin = WhisperCandlePlugin::new()
            .with_model_size(WhisperModelSize::Small)
            .with_language("es".to_string())
            .with_model_path(PathBuf::from("/tmp/model"))
            .with_device("cuda:0");

        assert_eq!(plugin.model_size, WhisperModelSize::Small);
        assert_eq!(plugin.language, Some("es".to_string()));
        assert_eq!(plugin.model_path, Some(PathBuf::from("/tmp/model")));
        assert_eq!(plugin.device, "cuda:0");
    }

    #[cfg(feature = "whisper")]
    #[test]
    fn test_parse_device_cpu() {
        let plugin = WhisperCandlePlugin::new().with_device("cpu");
        let device = plugin.parse_device();
        assert_eq!(device, WhisperDevice::Cpu);
    }

    #[cfg(feature = "whisper")]
    #[test]
    fn test_parse_device_cuda() {
        let plugin = WhisperCandlePlugin::new().with_device("cuda");
        let device = plugin.parse_device();
        assert_eq!(device, WhisperDevice::Cuda(0));

        let plugin = WhisperCandlePlugin::new().with_device("cuda:2");
        let device = plugin.parse_device();
        assert_eq!(device, WhisperDevice::Cuda(2));
    }

    #[cfg(feature = "whisper")]
    #[test]
    fn test_parse_device_metal() {
        let plugin = WhisperCandlePlugin::new().with_device("metal");
        let device = plugin.parse_device();
        assert_eq!(device, WhisperDevice::Metal);
    }

    #[test]
    fn test_plugin_info() {
        let plugin = WhisperCandlePlugin::new();
        let info = plugin.info();

        assert_eq!(info.id, "whisper-candle");
        assert_eq!(info.name, "Whisper (Candle)");
        assert!(!info.requires_network);
        assert!(info.is_local);
        assert!(info.supported_languages.contains(&"en".to_string()));
    }

    #[test]
    fn test_plugin_capabilities() {
        let plugin = WhisperCandlePlugin::new();
        let caps = plugin.capabilities();

        assert!(!caps.streaming, "Candle implementation is batch-only");
        assert!(caps.batch);
        assert!(!caps.word_timestamps, "Word-level timestamps not yet implemented");
        assert!(caps.confidence_scores);
        assert!(caps.auto_punctuation);
        assert!(!caps.speaker_diarization);
        assert!(!caps.custom_vocabulary);
    }

    #[test]
    fn test_factory_default() {
        std::env::remove_var("WHISPER_MODEL_PATH");
        std::env::remove_var("WHISPER_LANGUAGE");
        std::env::remove_var("WHISPER_DEVICE");

        let factory = WhisperCandlePluginFactory::default();
        assert_eq!(factory.device, "cpu");
        assert_eq!(factory.model_size, WhisperModelSize::Base);
    }

    #[test]
    fn test_factory_builder() {
        use std::path::PathBuf;

        let factory = WhisperCandlePluginFactory::new()
            .with_model_path(PathBuf::from("/models/whisper"))
            .with_model_size(WhisperModelSize::Medium)
            .with_language("fr".to_string())
            .with_device("cuda:1");

        assert_eq!(factory.model_path, Some(PathBuf::from("/models/whisper")));
        assert_eq!(factory.model_size, WhisperModelSize::Medium);
        assert_eq!(factory.language, Some("fr".to_string()));
        assert_eq!(factory.device, "cuda:1");
    }
}
