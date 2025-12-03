//! Moonshine CPU STT plugin using PyO3/HuggingFace Transformers.
//!
//! This is the PRIMARY Moonshine backend for CPU-based transcription:
//! - Uses HuggingFace Transformers via PyO3 Python bindings
//! - Automatic model downloading from HuggingFace Hub
//! - Production-quality inference without custom DSP/tokenizer work
//! - 5x faster than Whisper on CPU
//! - English-only, optimized for 16kHz audio

use crate::constants::SAMPLE_RATE_HZ;
use crate::plugin::*;
use crate::types::{TranscriptionConfig, TranscriptionEvent};
use async_trait::async_trait;
use coldvox_foundation::error::{ColdVoxError, SttError};
use std::env;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::{debug, info, warn};

#[cfg(feature = "moonshine")]
use pyo3::{types::PyModule, types::PyAnyMethods, Python};
#[cfg(feature = "moonshine")]
use tempfile::NamedTempFile;

// Maximum audio buffer size (30 seconds at 16kHz)
const MAX_AUDIO_BUFFER_SAMPLES: usize = 16000 * 30;
// Transcription timeout
const TRANSCRIPTION_TIMEOUT_SECS: u64 = 30;

/// Moonshine model variant
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoonshineModelSize {
    /// 27M parameters, 2.71% WER, faster inference
    Tiny,
    /// 61M parameters, ~2.5% WER, best quality (default)
    Base,
}

impl MoonshineModelSize {
    pub fn model_identifier(&self) -> &'static str {
        match self {
            Self::Tiny => "UsefulSensors/moonshine-tiny",
            Self::Base => "UsefulSensors/moonshine-base",
        }
    }

    pub fn memory_usage_mb(&self) -> u32 {
        match self {
            Self::Tiny => 300,  // ~100MB model + 200MB inference overhead
            Self::Base => 500,  // ~200MB model + 300MB inference overhead
        }
    }
}

impl Default for MoonshineModelSize {
    fn default() -> Self {
        Self::Base // Best quality by default
    }
}

/// Moonshine CPU plugin using PyO3/HuggingFace
#[derive(Debug)]
pub struct MoonshinePlugin {
    model_size: MoonshineModelSize,
    model_path: Option<PathBuf>,
    initialized: bool,
    #[cfg(feature = "moonshine")]
    audio_buffer: Vec<i16>,
    #[cfg(feature = "moonshine")]
    active_config: Option<TranscriptionConfig>,
}

impl MoonshinePlugin {
    pub fn new() -> Self {
        Self {
            model_size: MoonshineModelSize::default(),
            model_path: None,
            initialized: false,
            #[cfg(feature = "moonshine")]
            audio_buffer: Vec::new(),
            #[cfg(feature = "moonshine")]
            active_config: None,
        }
    }

    pub fn with_model_size(mut self, size: MoonshineModelSize) -> Self {
        self.model_size = size;
        self
    }

    pub fn with_model_path(mut self, path: PathBuf) -> Self {
        self.model_path = Some(path);
        self
    }

    #[cfg(feature = "moonshine")]
    fn verify_sample_rate(&self) -> Result<(), ColdVoxError> {
        const REQUIRED_SAMPLE_RATE: u32 = 16000;

        if SAMPLE_RATE_HZ != REQUIRED_SAMPLE_RATE {
            return Err(SttError::LoadFailed(format!(
                "Moonshine requires {}Hz audio, but SAMPLE_RATE_HZ is {}Hz",
                REQUIRED_SAMPLE_RATE, SAMPLE_RATE_HZ
            ))
            .into());
        }
        Ok(())
    }

    #[cfg(feature = "moonshine")]
    fn verify_python_environment() -> Result<(), ColdVoxError> {
        Python::with_gil(|py| {
            // Check transformers
            PyModule::import_bound(py, "transformers")
                .map_err(|_| SttError::LoadFailed(
                    "transformers not installed. Run: pip install transformers>=4.35.0".to_string()
                ))?;

            // Check torch
            PyModule::import_bound(py, "torch")
                .map_err(|_| SttError::LoadFailed(
                    "torch not installed. Run: pip install torch>=2.0.0".to_string()
                ))?;

            // Check librosa
            PyModule::import_bound(py, "librosa")
                .map_err(|_| SttError::LoadFailed(
                    "librosa not installed. Run: pip install librosa>=0.10.0".to_string()
                ))?;

            info!(target: "coldvox::stt::moonshine", "Python environment verified");
            Ok(())
        })
    }

    #[cfg(feature = "moonshine")]
    fn transcribe_via_python(&self, audio_path: &Path) -> Result<String, ColdVoxError> {
        Python::with_gil(|py| {
            let locals = pyo3::types::PyDict::new_bound(py);
            locals.set_item("model_id", self.model_size.model_identifier())
                .map_err(|e| SttError::TranscriptionFailed(format!("Failed to set model_id: {}", e)))?;
            locals.set_item("audio_path_str", audio_path.to_str().ok_or_else(|| SttError::TranscriptionFailed("Invalid path".to_string()))?)
                .map_err(|e| SttError::TranscriptionFailed(format!("Failed to set audio_path: {}", e)))?;

            let code = include_str!("moonshine_transcribe.py");

            py.run_bound(code, None, Some(&locals))
                .map_err(|e| SttError::TranscriptionFailed(format!("Python error: {}", e)))?;

            let result = locals.get_item("transcription")
                .map_err(|e| SttError::TranscriptionFailed(format!("Failed to get transcription result: {}", e)))?;

            let text: String = result.extract()
                .map_err(|e| SttError::TranscriptionFailed(format!("Failed to extract text: {}", e)))?;

            Ok(text)
        })
    }

    #[cfg(feature = "moonshine")]
    fn save_audio_to_wav(&self, samples: &[i16]) -> Result<NamedTempFile, ColdVoxError> {
        let temp_file = NamedTempFile::new()
            .map_err(|e| SttError::TranscriptionFailed(format!("Failed to create temp file: {}", e)))?;

        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: SAMPLE_RATE_HZ,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = hound::WavWriter::new(temp_file.reopen()?, spec)
            .map_err(|e| SttError::TranscriptionFailed(format!("Failed to create WAV writer: {}", e)))?;

        for &sample in samples {
            writer.write_sample(sample)
                .map_err(|e| SttError::TranscriptionFailed(format!("Failed to write sample: {}", e)))?;
        }

        writer.finalize()
            .map_err(|e| SttError::TranscriptionFailed(format!("Failed to finalize WAV: {}", e)))?;

        Ok(temp_file)
    }
}

impl Default for MoonshinePlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SttPlugin for MoonshinePlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: "moonshine".to_string(),
            name: format!("Moonshine {} CPU",
                match self.model_size {
                    MoonshineModelSize::Tiny => "Tiny",
                    MoonshineModelSize::Base => "Base",
                }),
            description: "CPU-optimized local transcription (English-only, 16kHz, 5x faster than Whisper)".to_string(),
            requires_network: false, // After initial download
            is_local: true,
            is_available: check_moonshine_available(),
            supported_languages: vec!["en".to_string()],
            memory_usage_mb: Some(self.model_size.memory_usage_mb()),
        }
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            streaming: false, // Batch processing only
            batch: true,
            word_timestamps: false, // Not available via transformers pipeline
            confidence_scores: false,
            speaker_diarization: false,
            auto_punctuation: true,
            custom_vocabulary: false,
        }
    }

    async fn is_available(&self) -> Result<bool, ColdVoxError> {
        Ok(check_moonshine_available())
    }

    async fn initialize(&mut self, config: TranscriptionConfig) -> Result<(), ColdVoxError> {
        #[cfg(feature = "moonshine")]
        {
            self.verify_sample_rate()?;
            Self::verify_python_environment()?;

            info!(
                target: "coldvox::stt::moonshine",
                model = %self.model_size.model_identifier(),
                "Initializing Moonshine CPU model via PyO3/HuggingFace"
            );

            // Model will be auto-downloaded by transformers on first use
            self.audio_buffer.clear();
            self.active_config = Some(config);
            self.initialized = true;

            info!(target: "coldvox::stt::moonshine", "Moonshine CPU initialized successfully");
            Ok(())
        }

        #[cfg(not(feature = "moonshine"))]
        {
            let _ = config;
            Err(SttError::NotAvailable {
                plugin: "moonshine".to_string(),
                reason: "Moonshine feature not compiled. Build with --features moonshine".to_string(),
            }.into())
        }
    }

    async fn process_audio(&mut self, samples: &[i16]) -> Result<Option<TranscriptionEvent>, ColdVoxError> {
        #[cfg(feature = "moonshine")]
        {
            if !self.initialized {
                return Err(SttError::NotAvailable {
                    plugin: "moonshine".to_string(),
                    reason: "Plugin not initialized".to_string(),
                }.into());
            }

            // Enforce buffer limit
            if self.audio_buffer.len() + samples.len() > MAX_AUDIO_BUFFER_SAMPLES {
                warn!(target: "coldvox::stt::moonshine", "Audio buffer exceeded limit, clearing");
                self.audio_buffer.clear();
                return Err(SttError::TranscriptionFailed("Audio buffer limit exceeded".to_string()).into());
            }

            self.audio_buffer.extend_from_slice(samples);
            Ok(None)
        }

        #[cfg(not(feature = "moonshine"))]
        {
            let _ = samples;
            Err(SttError::NotAvailable {
                plugin: "moonshine".to_string(),
                reason: "Moonshine feature not compiled".to_string(),
            }.into())
        }
    }

    async fn finalize(&mut self) -> Result<Option<TranscriptionEvent>, ColdVoxError> {
        #[cfg(feature = "moonshine")]
        {
            if !self.initialized || self.audio_buffer.is_empty() {
                return Ok(None);
            }

            let buffer_size = self.audio_buffer.len();
            info!(
                target: "coldvox::stt::moonshine",
                samples = buffer_size,
                duration_secs = %format!("{:.2}", buffer_size as f32 / SAMPLE_RATE_HZ as f32),
                "Transcribing via PyO3/HuggingFace"
            );

            // Save to temporary WAV file
            let temp_file = self.save_audio_to_wav(&self.audio_buffer)?;
            let audio_path = temp_file.path().to_path_buf();

            // Clone necessary data for async task
            let plugin_clone = self.clone_for_task();

            // Spawn blocking task with timeout
            let transcription_result = tokio::time::timeout(
                Duration::from_secs(TRANSCRIPTION_TIMEOUT_SECS),
                tokio::task::spawn_blocking(move || {
                    plugin_clone.transcribe_via_python(&audio_path)
                })
            ).await;

            self.audio_buffer.clear();

            match transcription_result {
                Ok(task_result) => {
                    match task_result {
                        Ok(Ok(text)) => {
                            debug!(target: "coldvox::stt::moonshine", text = %text, "Transcription complete");
                            Ok(Some(TranscriptionEvent::Final {
                                utterance_id: 0,
                                text,
                                words: None,
                            }))
                        },
                        Ok(Err(e)) => Err(e),
                        Err(e) => Err(SttError::TranscriptionFailed(format!("Task join error: {}", e)).into()),
                    }
                },
                Err(_) => {
                     warn!(target: "coldvox::stt::moonshine", "Transcription timed out");
                     Err(SttError::TranscriptionFailed("Transcription timed out".to_string()).into())
                }
            }
        }

        #[cfg(not(feature = "moonshine"))]
        {
            Err(SttError::NotAvailable {
                plugin: "moonshine".to_string(),
                reason: "Moonshine feature not compiled".to_string(),
            }.into())
        }
    }

    async fn reset(&mut self) -> Result<(), ColdVoxError> {
        #[cfg(feature = "moonshine")]
        {
            self.audio_buffer.clear();
            Ok(())
        }

        #[cfg(not(feature = "moonshine"))]
        {
            Err(SttError::NotAvailable {
                plugin: "moonshine".to_string(),
                reason: "Moonshine feature not compiled".to_string(),
            }.into())
        }
    }

    async fn load_model(&mut self, _model_path: Option<&Path>) -> Result<(), ColdVoxError> {
        Ok(())
    }

    async fn unload(&mut self) -> Result<(), ColdVoxError> {
        #[cfg(feature = "moonshine")]
        {
            self.audio_buffer.clear();
            self.initialized = false;
            Ok(())
        }

        #[cfg(not(feature = "moonshine"))]
        {
            Err(SttError::NotAvailable {
                plugin: "moonshine".to_string(),
                reason: "Moonshine feature not compiled".to_string(),
            }.into())
        }
    }
}

#[cfg(feature = "moonshine")]
impl MoonshinePlugin {
    // Helper to clone necessary data for the blocking task
    fn clone_for_task(&self) -> Self {
        Self {
            model_size: self.model_size,
            model_path: self.model_path.clone(),
            initialized: self.initialized,
            audio_buffer: Vec::new(), // Not needed in the task
            active_config: None, // Not needed in the task
        }
    }
}

/// Factory for Moonshine plugin
pub struct MoonshinePluginFactory {
    model_size: MoonshineModelSize,
    model_path: Option<PathBuf>,
}

impl MoonshinePluginFactory {
    pub fn new() -> Self {
        let model_size = env::var("MOONSHINE_MODEL")
            .ok()
            .and_then(|v| match v.to_lowercase().as_str() {
                "tiny" => Some(MoonshineModelSize::Tiny),
                "base" => Some(MoonshineModelSize::Base),
                _ => {
                    warn!(target: "coldvox::stt::moonshine", "Invalid MOONSHINE_MODEL: {}", v);
                    None
                }
            })
            .unwrap_or_default();

        Self {
            model_size,
            model_path: env::var("MOONSHINE_MODEL_PATH").ok().map(PathBuf::from),
        }
    }
}

impl Default for MoonshinePluginFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl SttPluginFactory for MoonshinePluginFactory {
    fn create(&self) -> Result<Box<dyn SttPlugin>, ColdVoxError> {
        let mut plugin = MoonshinePlugin::new()
            .with_model_size(self.model_size);

        if let Some(ref path) = self.model_path {
            plugin = plugin.with_model_path(path.clone());
        }

        Ok(Box::new(plugin))
    }

    fn plugin_info(&self) -> PluginInfo {
        MoonshinePlugin::new()
            .with_model_size(self.model_size)
            .info()
    }

    fn check_requirements(&self) -> Result<(), ColdVoxError> {
        if !check_moonshine_available() {
            return Err(SttError::NotAvailable {
                plugin: "moonshine".to_string(),
                reason: "Python 3.8+ required with transformers, torch, librosa".to_string(),
            }.into());
        }

        #[cfg(feature = "moonshine")]
        MoonshinePlugin::verify_python_environment()?;

        Ok(())
    }
}

#[cfg(feature = "moonshine")]
fn check_moonshine_available() -> bool {
    Python::with_gil(|py| {
        PyModule::import_bound(py, "transformers").is_ok() &&
        PyModule::import_bound(py, "torch").is_ok() &&
        PyModule::import_bound(py, "librosa").is_ok()
    })
}

#[cfg(not(feature = "moonshine"))]
fn check_moonshine_available() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_model_is_base() {
        assert_eq!(MoonshineModelSize::default(), MoonshineModelSize::Base);
    }

    #[test]
    fn model_identifiers_correct() {
        assert_eq!(
            MoonshineModelSize::Tiny.model_identifier(),
            "UsefulSensors/moonshine-tiny"
        );
        assert_eq!(
            MoonshineModelSize::Base.model_identifier(),
            "UsefulSensors/moonshine-base"
        );
    }

    #[test]
    fn memory_usage_reasonable() {
        assert_eq!(MoonshineModelSize::Tiny.memory_usage_mb(), 300);
        assert_eq!(MoonshineModelSize::Base.memory_usage_mb(), 500);
        assert!(MoonshineModelSize::Base.memory_usage_mb() < 1000);
    }

    #[test]
    fn plugin_info_correct() {
        let plugin = MoonshinePlugin::new();
        let info = plugin.info();

        assert_eq!(info.id, "moonshine");
        assert!(info.supported_languages.contains(&"en".to_string()));
        assert!(info.is_local);
    }

    #[test]
    fn capabilities_correct() {
        let plugin = MoonshinePlugin::new();
        let caps = plugin.capabilities();

        assert!(!caps.streaming);
        assert!(caps.batch);
        assert!(caps.auto_punctuation);
    }
}
