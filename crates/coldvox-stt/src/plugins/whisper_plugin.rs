//! Faster-Whisper speech-to-text plugin implementation.
//!
//! This plugin provides a local transcription backend powered by the
//! `faster-whisper` project. It relies on the `faster-whisper-rs`
//! bindings which bridge to the Python implementation. At this stage we
//! intentionally focus on providing a functional baseline capable of
//! loading a model, buffering audio produced by the VAD pipeline, and
//! performing batch transcription when the VAD signals the end of an
//! utterance. Follow-up work will iterate on streaming partials,
//! fine-grained error handling, and production hardening.

use crate::plugin::*;
use crate::types::{TranscriptionConfig, TranscriptionEvent, WordInfo};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
#[allow(unused_imports)]
use tracing::{debug, info, warn};

#[cfg(feature = "whisper")]
use faster_whisper_rs::{
    config::{VadConfig, WhisperConfig},
    WhisperModel,
};
#[cfg(feature = "whisper")]
use pyo3::Python;
#[cfg(feature = "whisper")]
use tempfile::Builder;

/// Whisper-based STT plugin backed by faster-whisper.
#[derive(Debug)]
pub struct WhisperPlugin {
    model_path: Option<PathBuf>,
    model_size: WhisperModelSize,
    language: Option<String>,
    device: String,
    compute_type: String,
    initialized: bool,
    #[cfg(feature = "whisper")]
    model: Option<WhisperModel>,
    #[cfg(feature = "whisper")]
    audio_buffer: Vec<i16>,
    #[cfg(feature = "whisper")]
    active_config: Option<TranscriptionConfig>,
}

impl WhisperPlugin {
    pub fn new() -> Self {
        Self {
            model_path: None,
            model_size: WhisperModelSize::default(),
            language: None,
            device: "cpu".to_string(),
            compute_type: "int8".to_string(),
            initialized: false,
            #[cfg(feature = "whisper")]
            model: None,
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

    pub fn with_compute_type<S: Into<String>>(mut self, compute_type: S) -> Self {
        self.compute_type = compute_type.into();
        self
    }

    #[cfg(feature = "whisper")]
    fn resolve_model_identifier(&self, config: &TranscriptionConfig) -> Result<String, SttPluginError> {
        let path_candidate = if !config.model_path.is_empty() {
            Some(PathBuf::from(&config.model_path))
        } else {
            self.model_path.clone()
        };

        if let Some(path) = path_candidate {
            if path.exists() {
                return Ok(path.to_string_lossy().to_string());
            }

            warn!(
                target: "coldvox::stt::whisper",
                candidate = %path.display(),
                "Configured Whisper model path does not exist; falling back to builtin model size"
            );
        }

        Ok(self.model_size.model_identifier())
    }

    #[cfg(feature = "whisper")]
    fn build_whisper_config(&self, config: &TranscriptionConfig) -> WhisperConfig {
        let mut whisper_config = WhisperConfig::default();
        whisper_config.language = self.language.clone();
        whisper_config.beam_size = config.max_alternatives.max(1) as usize;
        whisper_config.best_of = config.max_alternatives.max(1) as usize;
        whisper_config.vad = VadConfig {
            active: config.streaming,
            ..Default::default()
        };
        whisper_config
    }
}

impl Default for WhisperPlugin {
    fn default() -> Self {
        Self::new()
    }
}

/// Available Whisper model sizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhisperModelSize {
    Tiny,
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

    fn model_identifier(&self) -> String {
        match self {
            Self::Tiny => "tiny".to_string(),
            Self::Base => "base.en".to_string(),
            Self::Small => "small.en".to_string(),
            Self::Medium => "medium.en".to_string(),
            Self::Large => "large".to_string(),
            Self::LargeV2 => "large-v2".to_string(),
            Self::LargeV3 => "large-v3".to_string(),
        }
    }
}

impl Default for WhisperModelSize {
    fn default() -> Self {
        Self::Base
    }
}

#[async_trait]
impl SttPlugin for WhisperPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: "whisper".to_string(),
            name: "Faster Whisper".to_string(),
            description: "Local transcription via faster-whisper".to_string(),
            requires_network: false,
            is_local: true,
            is_available: check_whisper_available(),
            supported_languages: vec![
                "auto".to_string(),
                "en".to_string(),
            ],
            memory_usage_mb: Some(self.model_size.memory_usage_mb()),
        }
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            streaming: false,
            batch: true,
            word_timestamps: true,
            confidence_scores: true,
            speaker_diarization: false,
            auto_punctuation: true,
            custom_vocabulary: false,
        }
    }

    async fn is_available(&self) -> Result<bool, SttPluginError> {
        Ok(check_whisper_available())
    }

    async fn initialize(&mut self, config: TranscriptionConfig) -> Result<(), SttPluginError> {
        #[cfg(feature = "whisper")]
        {
            let model_id = self.resolve_model_identifier(&config)?;
            let mut whisper_config = self.build_whisper_config(&config);
            if whisper_config.language.is_none() {
                whisper_config.language = self.language.clone();
            }

            debug!(
                target: "coldvox::stt::whisper",
                model = %model_id,
                device = %self.device,
                compute = %self.compute_type,
                "Initializing Faster Whisper model"
            );

            let model = WhisperModel::new(
                model_id,
                self.device.clone(),
                self.compute_type.clone(),
                whisper_config,
            )
            .map_err(|err| SttPluginError::ModelLoadFailed(err.to_string()))?;

            self.model = Some(model);
            self.audio_buffer.clear();
            self.active_config = Some(config);
            self.initialized = true;
            info!(
                target: "coldvox::stt::whisper",
                "Faster Whisper plugin initialized"
            );
            return Ok(());
        }

        #[cfg(not(feature = "whisper"))]
        {
            let _ = config;
            Err(SttPluginError::NotAvailable {
                reason: "Whisper feature not compiled".to_string(),
            })
        }
    }

    async fn process_audio(
        &mut self,
        samples: &[i16],
    ) -> Result<Option<TranscriptionEvent>, SttPluginError> {
        #[cfg(feature = "whisper")]
        {
            if !self.initialized {
                return Err(SttPluginError::InitializationFailed(
                    "Faster Whisper plugin not initialized".to_string(),
                ));
            }

            self.audio_buffer.extend_from_slice(samples);
            Ok(None)
        }

        #[cfg(not(feature = "whisper"))]
        {
            let _ = samples;
            Err(SttPluginError::InitializationFailed(
                "Whisper feature not compiled".to_string(),
            ))
        }
    }

    async fn finalize(&mut self) -> Result<Option<TranscriptionEvent>, SttPluginError> {
        #[cfg(feature = "whisper")]
        {
            if !self.initialized {
                return Ok(None);
            }

            if self.audio_buffer.is_empty() {
                return Ok(None);
            }

            let temp = Builder::new()
                .prefix("coldvox-whisper-")
                .suffix(".wav")
                .tempfile()
                .map_err(|err| SttPluginError::ProcessingError(err.to_string()))?;
            let temp_path = temp.path().to_path_buf();

            {
                let spec = hound::WavSpec {
                    channels: 1,
                    sample_rate: crate::constants::SAMPLE_RATE_HZ,
                    bits_per_sample: 16,
                    sample_format: hound::SampleFormat::Int,
                };
                let mut writer = hound::WavWriter::create(&temp_path, spec)
                    .map_err(|err| SttPluginError::ProcessingError(err.to_string()))?;
                for sample in &self.audio_buffer {
                    writer
                        .write_sample(*sample)
                        .map_err(|err| SttPluginError::ProcessingError(err.to_string()))?;
                }
                writer
                    .finalize()
                    .map_err(|err| SttPluginError::ProcessingError(err.to_string()))?;
            }

            let transcription = self
                .model
                .as_ref()
                .ok_or_else(|| {
                    SttPluginError::ProcessingError(
                        "Faster Whisper model not loaded".to_string(),
                    )
                })?
                .transcribe(temp_path.to_string_lossy().to_string())
                .map_err(|err| SttPluginError::TranscriptionFailed(err.to_string()))?;

            let mut text = transcription.to_string();
            if text.ends_with('\n') {
                text.pop();
            }

            let include_words = self
                .active_config
                .as_ref()
                .map(|cfg| cfg.include_words)
                .unwrap_or(false);

            let words = if include_words {
                Some(
                    transcription
                        .1
                        .iter()
                        .map(|segment| WordInfo {
                            start: segment.start,
                            end: segment.end,
                            conf: (1.0 - segment.no_speech_prob).clamp(0.0, 1.0) as f32,
                            text: segment.text.clone(),
                        })
                        .collect(),
                )
            } else {
                None
            };

            self.audio_buffer.clear();
            // Ensure the temporary file is cleaned up.
            if let Err(err) = temp.close() {
                warn!(
                    target: "coldvox::stt::whisper",
                    error = %err,
                    "Failed to remove temporary whisper audio file"
                );
            }

            Ok(Some(TranscriptionEvent::Final {
                utterance_id: 0,
                text,
                words,
            }))
        }

        #[cfg(not(feature = "whisper"))]
        {
            Err(SttPluginError::InitializationFailed(
                "Whisper feature not compiled".to_string(),
            ))
        }
    }

    async fn reset(&mut self) -> Result<(), SttPluginError> {
        #[cfg(feature = "whisper")]
        {
            self.audio_buffer.clear();
            Ok(())
        }

        #[cfg(not(feature = "whisper"))]
        {
            Err(SttPluginError::InitializationFailed(
                "Whisper feature not compiled".to_string(),
            ))
        }
    }

    async fn load_model(&mut self, model_path: Option<&Path>) -> Result<(), SttPluginError> {
        if let Some(path) = model_path {
            self.model_path = Some(path.to_path_buf());
        }
        Ok(())
    }

    async fn unload(&mut self) -> Result<(), SttPluginError> {
        #[cfg(feature = "whisper")]
        {
            self.model = None;
            self.audio_buffer.clear();
            self.initialized = false;
            Ok(())
        }

        #[cfg(not(feature = "whisper"))]
        {
            Err(SttPluginError::InitializationFailed(
                "Whisper feature not compiled".to_string(),
            ))
        }
    }
}

/// Factory for creating WhisperPlugin instances.
pub struct WhisperPluginFactory {
    model_path: Option<PathBuf>,
    model_size: WhisperModelSize,
    language: Option<String>,
    device: String,
    compute_type: String,
}

impl WhisperPluginFactory {
    pub fn new() -> Self {
        Self {
            model_path: std::env::var("WHISPER_MODEL_PATH").ok().map(PathBuf::from),
            model_size: WhisperModelSize::Base,
            language: std::env::var("WHISPER_LANGUAGE").ok(),
            device: std::env::var("WHISPER_DEVICE").unwrap_or_else(|_| "cpu".to_string()),
            compute_type: std::env::var("WHISPER_COMPUTE").unwrap_or_else(|_| "int8".to_string()),
        }
    }

    pub fn with_model_size(mut self, size: WhisperModelSize) -> Self {
        self.model_size = size;
        self
    }

    pub fn with_model_path(mut self, path: PathBuf) -> Self {
        self.model_path = Some(path);
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

    pub fn with_compute_type<S: Into<String>>(mut self, compute_type: S) -> Self {
        self.compute_type = compute_type.into();
        self
    }
}

impl Default for WhisperPluginFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl SttPluginFactory for WhisperPluginFactory {
    fn create(&self) -> Result<Box<dyn SttPlugin>, SttPluginError> {
        let mut plugin = WhisperPlugin::new()
            .with_model_size(self.model_size)
            .with_device(self.device.clone())
            .with_compute_type(self.compute_type.clone());

        if let Some(ref path) = self.model_path {
            plugin = plugin.with_model_path(path.clone());
        }

        if let Some(ref lang) = self.language {
            plugin = plugin.with_language(lang.clone());
        }

        Ok(Box::new(plugin))
    }

    fn plugin_info(&self) -> PluginInfo {
        WhisperPlugin::new()
            .with_model_size(self.model_size)
            .with_device(self.device.clone())
            .with_compute_type(self.compute_type.clone())
            .info()
    }

    fn check_requirements(&self) -> Result<(), SttPluginError> {
        if !check_whisper_available() {
            return Err(SttPluginError::NotAvailable {
                reason: "The faster-whisper Python module is not available. Install the `faster-whisper` package.".to_string(),
            });
        }

        if let Some(ref path) = self.model_path {
            if !path.exists() {
                return Err(SttPluginError::NotAvailable {
                    reason: format!("Model not found at {:?}", path),
                });
            }
        }

        Ok(())
    }
}

#[cfg(feature = "whisper")]
fn check_whisper_available() -> bool {
    Python::with_gil(|py| py.import("faster_whisper").is_ok())
}

#[cfg(not(feature = "whisper"))]
fn check_whisper_available() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_size_identifier_mapping() {
        assert_eq!(WhisperModelSize::Tiny.model_identifier(), "tiny");
        assert_eq!(WhisperModelSize::Base.model_identifier(), "base.en");
        assert_eq!(WhisperModelSize::LargeV3.model_identifier(), "large-v3");
    }
}
