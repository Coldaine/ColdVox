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
//!
//! # GPU Detection Caching
//!
//! The plugin implements GPU detection caching using `OnceLock` to avoid
//! repeated Python round-trips during `WhisperPluginFactory` construction.
//! This significantly improves performance when creating multiple factory
//! instances, which is common in testing scenarios.
//!
//! The caching mechanism:
//! - Uses a static `OnceLock<String>` to cache the GPU detection result
//! - Performs GPU detection only once on the first call to `detect_device()`
//! - Returns the cached result for all subsequent calls
//! - Is thread-safe and handles concurrent access correctly
//! - Can still be overridden by setting the `WHISPER_DEVICE` environment variable
//!
//! This approach eliminates the overhead of shell-outs to Python/PyTorch
//! while maintaining the flexibility to override the detected device.

use crate::plugin::*;
use crate::types::{TranscriptionConfig, TranscriptionEvent};
#[cfg(feature = "whisper")]
use crate::WordInfo;
use async_trait::async_trait;
use coldvox_foundation::env::{detect_environment, Environment};
use std::env;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
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

use coldvox_foundation::error::{ColdVoxError, SttError};

/// Static cache for GPU detection result to avoid repeated Python round-trips
///
/// This cache stores the result of GPU detection to avoid repeated shell-outs
/// to Python/PyTorch during `WhisperPluginFactory` construction. The cache is
/// initialized once using `OnceLock` and then reused for all subsequent calls.
///
/// The cache is thread-safe and handles concurrent access correctly. The cached
/// value can still be overridden by setting the `WHISPER_DEVICE` environment
/// variable before creating a factory instance.
static GPU_DETECTION_CACHE: OnceLock<String> = OnceLock::new();

/// Whisper-based STT plugin backed by faster-whisper.
#[derive(Debug)]
pub struct WhisperPlugin {
    model_path: Option<PathBuf>,
    model_size: WhisperModelSize,
    language: Option<String>,
    device: String,
    compute_type: String,
    #[allow(dead_code)]
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
    fn resolve_model_identifier(
        &self,
        config: &TranscriptionConfig,
    ) -> Result<String, ColdVoxError> {
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
        WhisperConfig {
            language: self.language.clone(),
            beam_size: config.max_alternatives.max(1) as usize,
            best_of: config.max_alternatives.max(1) as usize,
            vad: VadConfig {
                active: config.streaming,
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

impl Default for WhisperPlugin {
    fn default() -> Self {
        Self::new()
    }
}

/// Available Whisper model sizes.
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

    #[allow(dead_code)]
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

/// Get the default model size for the given environment
fn default_model_size_for_environment(env: Environment) -> WhisperModelSize {
    match env {
        Environment::CI => {
            // In CI, use the smallest model to conserve resources
            WhisperModelSize::Tiny
        }
        Environment::Development => {
            // In development, check available memory and choose accordingly
            if let Some(available_mb) = WhisperPluginFactory::get_available_memory_mb() {
                // On high-performance developer workstations, prefer the largest model for accuracy
                // Use a conservative threshold (>= 12 GB available) to avoid impacting typical laptops
                if available_mb >= 12_000 {
                    WhisperModelSize::LargeV3
                } else {
                    WhisperPluginFactory::get_model_size_for_memory(available_mb)
                }
            } else {
                // If we can't determine memory, use a small model
                WhisperModelSize::Base
            }
        }
        Environment::Production => {
            // In production, check available memory and choose accordingly
            if let Some(available_mb) = WhisperPluginFactory::get_available_memory_mb() {
                WhisperPluginFactory::get_model_size_for_memory(available_mb)
            } else {
                // If we can't determine memory, use a balanced model
                WhisperModelSize::Small
            }
        }
    }
}

#[async_trait]
impl SttPlugin for WhisperPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: "whisper".to_string(),
            name: "Candle Whisper".to_string(),
            description: "Local transcription via Candle Whisper".to_string(),
            requires_network: false,
            is_local: true,
            is_available: check_whisper_available(),
            supported_languages: vec!["auto".to_string(), "en".to_string()],
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

    async fn is_available(&self) -> Result<bool, ColdVoxError> {
        Ok(check_whisper_available())
    }

    async fn initialize(&mut self, config: TranscriptionConfig) -> Result<(), ColdVoxError> {
        #[cfg(feature = "whisper")]
        {
            let model_id = self.resolve_model_identifier(&config)?;
            let mut whisper_config = self.build_whisper_config(&config);
            if whisper_config.language.is_none() {
                whisper_config.language = self.language.clone();
            }

            // If the selected model is English-only (e.g., base.en/small.en/medium.en)
            // and no language was set explicitly, default to "en" to avoid runtime warnings.
            if whisper_config.language.is_none() && model_id.to_lowercase().contains(".en") {
                whisper_config.language = Some("en".to_string());
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
            .map_err(|err| SttError::LoadFailed(err.to_string()))?;

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
            Err(SttError::NotAvailable {
                plugin: "whisper".to_string(),
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
                    plugin: "whisper".to_string(),
                    reason: "Faster Whisper plugin not initialized".to_string(),
                }
                .into());
            }

            self.audio_buffer.extend_from_slice(samples);
            Ok(None)
        }

        #[cfg(not(feature = "whisper"))]
        {
            let _ = samples;
            Err(SttError::NotAvailable {
                plugin: "whisper".to_string(),
                reason: "Whisper feature not compiled".to_string(),
            }
            .into())
        }
    }

    async fn finalize(&mut self) -> Result<Option<TranscriptionEvent>, ColdVoxError> {
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
                .map_err(|err| SttError::TranscriptionFailed(err.to_string()))?;
            let temp_path = temp.path().to_path_buf();

            {
                let spec = hound::WavSpec {
                    channels: 1,
                    sample_rate: crate::constants::SAMPLE_RATE_HZ,
                    bits_per_sample: 16,
                    sample_format: hound::SampleFormat::Int,
                };
                let mut writer = hound::WavWriter::create(&temp_path, spec)
                    .map_err(|err| SttError::TranscriptionFailed(err.to_string()))?;
                for sample in &self.audio_buffer {
                    writer
                        .write_sample(*sample)
                        .map_err(|err| SttError::TranscriptionFailed(err.to_string()))?;
                }
                writer
                    .finalize()
                    .map_err(|err| SttError::TranscriptionFailed(err.to_string()))?;
            }

            let transcription = self
                .model
                .as_ref()
                .ok_or_else(|| {
                    SttError::TranscriptionFailed("Faster Whisper model not loaded".to_string())
                })?
                .transcribe(temp_path.to_string_lossy().to_string())
                .map_err(|err| SttError::TranscriptionFailed(err.to_string()))?;

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
                            conf: (1.0 - segment.no_speech_prob).clamp(0.0, 1.0),
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
            Err(SttError::NotAvailable {
                plugin: "whisper".to_string(),
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
                plugin: "whisper".to_string(),
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
            self.model = None;
            self.audio_buffer.clear();
            self.initialized = false;
            Ok(())
        }

        #[cfg(not(feature = "whisper"))]
        {
            Err(SttError::NotAvailable {
                plugin: "whisper".to_string(),
                reason: "Whisper feature not compiled".to_string(),
            }
            .into())
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
        // Check for WHISPER_MODEL_SIZE environment variable first
        let model_size = if let Ok(model_size_str) = env::var("WHISPER_MODEL_SIZE") {
            Self::parse_model_size(&model_size_str).unwrap_or_else(|_| {
                warn!(
                    target: "coldvox::stt::whisper",
                    "Invalid WHISPER_MODEL_SIZE value: {}, using default", model_size_str
                );
                default_model_size_for_environment(detect_environment())
            })
        } else {
            default_model_size_for_environment(detect_environment())
        };

        let device = std::env::var("WHISPER_DEVICE").unwrap_or_else(|_| Self::detect_device());
        let compute_type = std::env::var("WHISPER_COMPUTE").unwrap_or_else(|_| {
            if device == "cuda" {
                "float16".to_string()
            } else {
                "int8".to_string()
            }
        });

        Self {
            model_path: std::env::var("WHISPER_MODEL_PATH").ok().map(PathBuf::from),
            model_size,
            language: std::env::var("WHISPER_LANGUAGE")
                .ok()
                .or(Some("en".to_string())),
            device,
            compute_type,
        }
    }

    /// Detect GPU availability and return appropriate device
    ///
    /// This function uses `OnceLock` to cache the GPU detection result and avoid
    /// repeated Python round-trips. The first call performs the actual detection
    /// by shell-ing out to Python/PyTorch to check CUDA availability. Subsequent
    /// calls return the cached result.
    ///
    /// The detection process:
    /// 1. Checks if CUDA is available using PyTorch's `torch.cuda.is_available()`
    /// 2. Returns "cuda" if GPU is available, "cpu" otherwise
    /// 3. Caches the result to avoid repeated shell-outs
    ///
    /// # Thread Safety
    ///
    /// This function is thread-safe and can be called concurrently from multiple
    /// threads. The `OnceLock` ensures that only one thread performs the actual
    /// detection, while others wait for and receive the cached result.
    ///
    /// # Environment Override
    ///
    /// The `WHISPER_DEVICE` environment variable can still override this detection
    /// when creating a `WhisperPluginFactory`, as the factory checks this variable
    /// before calling this function.
    ///
    /// # Returns
    ///
    /// Returns either "cuda" if a compatible GPU is detected, or "cpu" if no GPU
    /// is available or detection fails.
    pub fn detect_device() -> String {
        GPU_DETECTION_CACHE.get_or_init(|| {
            // Check for CUDA availability using PyTorch
            if let Ok(output) = std::process::Command::new("python3")
                .arg("-c")
                .arg("import torch; print('cuda' if torch.cuda.is_available() else 'cpu')")
                .output()
            {
                if output.status.success() {
                    let device = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if device == "cuda" {
                        info!(target: "coldvox::stt::whisper", "CUDA GPU detected, using GPU acceleration");
                        return device;
                    }
                }
            }

            warn!(target: "coldvox::stt::whisper", "No GPU detected, falling back to CPU");
            "cpu".to_string()
        }).clone()
    }

    /// Parse model size from string
    fn parse_model_size(size_str: &str) -> Result<WhisperModelSize, ()> {
        match size_str.to_lowercase().as_str() {
            "tiny" => Ok(WhisperModelSize::Tiny),
            "base" => Ok(WhisperModelSize::Base),
            "small" => Ok(WhisperModelSize::Small),
            "medium" => Ok(WhisperModelSize::Medium),
            "large" => Ok(WhisperModelSize::Large),
            "large-v2" => Ok(WhisperModelSize::LargeV2),
            "large-v3" => Ok(WhisperModelSize::LargeV3),
            _ => Err(()),
        }
    }

    /// Get available memory in MB
    fn get_available_memory_mb() -> Option<u32> {
        // Test/override hook: allow forcing a specific available memory size via env var
        // Useful for unit tests and local validation without relying on /proc/meminfo.
        if let Ok(fake_mb) = env::var("WHISPER_AVAILABLE_MEM_MB") {
            if let Ok(val) = fake_mb.parse::<u32>() {
                return Some(val);
            }
        }

        #[cfg(unix)]
        {
            use std::fs;
            match fs::read_to_string("/proc/meminfo") {
                Ok(content) => {
                    for line in content.lines() {
                        if line.starts_with("MemAvailable:") {
                            let parts: Vec<&str> = line.split_whitespace().collect();
                            if parts.len() >= 2 {
                                if let Ok(kb) = parts[1].parse::<u32>() {
                                    return Some(kb / 1024); // Convert KB to MB
                                }
                            }
                        }
                    }
                    None
                }
                Err(_) => None,
            }
        }

        #[cfg(not(unix))]
        {
            // For non-Unix systems, return None
            None
        }
    }

    /// Get appropriate model size based on available memory
    fn get_model_size_for_memory(available_mb: u32) -> WhisperModelSize {
        if available_mb < 500 {
            WhisperModelSize::Tiny
        } else if available_mb < 1000 {
            WhisperModelSize::Base
        } else if available_mb < 2000 {
            WhisperModelSize::Small
        } else if available_mb < 4000 {
            WhisperModelSize::Medium
        } else {
            WhisperModelSize::Base // Default to Base even with lots of memory for stability
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
    fn create(&self) -> Result<Box<dyn SttPlugin>, ColdVoxError> {
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

    fn check_requirements(&self) -> Result<(), ColdVoxError> {
        if !check_whisper_available() {
            return Err(SttError::NotAvailable {
                plugin: "whisper".to_string(),
                reason: "The Candle Whisper implementation is not available.".to_string(),
            }
            .into());
        }

        if let Some(ref path) = self.model_path {
            if !path.exists() {
                return Err(SttError::ModelNotFound { path: path.clone() }.into());
            }
        }

        Ok(())
    }
}

#[cfg(feature = "whisper")]
fn check_whisper_available() -> bool {
    Python::with_gil(|py| py.import_bound("faster_whisper").is_ok())
}

#[cfg(not(feature = "whisper"))]
fn check_whisper_available() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn model_size_identifier_mapping() {
        assert_eq!(WhisperModelSize::Tiny.model_identifier(), "tiny");
        assert_eq!(WhisperModelSize::Base.model_identifier(), "base.en");
        assert_eq!(WhisperModelSize::LargeV3.model_identifier(), "large-v3");
    }

    #[test]
    fn parse_model_size() {
        assert_eq!(
            WhisperPluginFactory::parse_model_size("tiny").unwrap(),
            WhisperModelSize::Tiny
        );
        assert_eq!(
            WhisperPluginFactory::parse_model_size("large-v3").unwrap(),
            WhisperModelSize::LargeV3
        );
        assert!(WhisperPluginFactory::parse_model_size("invalid").is_err());
        assert!(WhisperPluginFactory::parse_model_size("").is_err());
    }

    #[test]
    fn environment_detection() {
        // Test CI detection
        env::set_var("CI", "true");
        assert_eq!(detect_environment(), Environment::CI);
        env::remove_var("CI");

        // Test development detection
        env::set_var("DEBUG", "1");
        assert_eq!(detect_environment(), Environment::Development);
        env::remove_var("DEBUG");

        // Default to production when no indicators are present
        assert_eq!(detect_environment(), Environment::Production);
    }

    #[test]
    fn model_size_for_memory() {
        // Test memory-based model selection
        assert_eq!(
            WhisperPluginFactory::get_model_size_for_memory(300),
            WhisperModelSize::Tiny
        );
        assert_eq!(
            WhisperPluginFactory::get_model_size_for_memory(750),
            WhisperModelSize::Base
        );
        assert_eq!(
            WhisperPluginFactory::get_model_size_for_memory(1500),
            WhisperModelSize::Small
        );
        assert_eq!(
            WhisperPluginFactory::get_model_size_for_memory(3000),
            WhisperModelSize::Medium
        );
        assert_eq!(
            WhisperPluginFactory::get_model_size_for_memory(8000),
            WhisperModelSize::Base
        );
    }

    #[test]
    fn environment_default_model_sizes() {
        // Test default model sizes for each environment
        assert_eq!(
            default_model_size_for_environment(Environment::CI),
            WhisperModelSize::Tiny
        );

        // Development and production depend on memory, so we can't test exact values
        // without mocking memory detection
    }

    #[test]
    fn development_env_prefers_large_on_beefy_machine() {
        // Simulate development environment
        env::set_var("DEBUG", "1");
        // Simulate a beefy machine with lots of available memory
        env::set_var("WHISPER_AVAILABLE_MEM_MB", "16384");

        assert_eq!(detect_environment(), Environment::Development);
        let chosen = default_model_size_for_environment(Environment::Development);
        assert_eq!(chosen, WhisperModelSize::LargeV3);

        env::remove_var("WHISPER_AVAILABLE_MEM_MB");
        env::remove_var("DEBUG");
    }

    #[test]
    fn production_env_does_not_escalate_to_large_by_default() {
        // Ensure no CI or dev markers are present
        for var in [
            "CI",
            "CONTINUOUS_INTEGRATION",
            "GITHUB_ACTIONS",
            "GITLAB_CI",
            "TRAVIS",
            "CIRCLECI",
            "JENKINS_URL",
            "BUILDKITE",
            "RUST_BACKTRACE",
            "DEBUG",
            "DEV",
        ] {
            env::remove_var(var);
        }

        // Simulate lots of memory
        env::set_var("WHISPER_AVAILABLE_MEM_MB", "16384");
        assert_eq!(detect_environment(), Environment::Production);
        let chosen = default_model_size_for_environment(Environment::Production);
        assert_ne!(chosen, WhisperModelSize::LargeV3);
        env::remove_var("WHISPER_AVAILABLE_MEM_MB");
    }

    #[test]
    fn whisper_model_size_env_var() {
        // Test that WHISPER_MODEL_SIZE environment variable is respected
        env::set_var("WHISPER_MODEL_SIZE", "large-v2");
        let factory = WhisperPluginFactory::new();
        assert_eq!(factory.model_size, WhisperModelSize::LargeV2);
        env::remove_var("WHISPER_MODEL_SIZE");

        // Test with invalid value - should fall back to environment default
        env::set_var("WHISPER_MODEL_SIZE", "invalid-size");
        let factory = WhisperPluginFactory::new();
        // Should not panic and should use a valid default based on environment
        assert!(matches!(
            factory.model_size,
            WhisperModelSize::Tiny | WhisperModelSize::Base | WhisperModelSize::Small
        ));
        env::remove_var("WHISPER_MODEL_SIZE");
    }

    #[test]
    fn gpu_detection_caching() {
        // Ensure WHISPER_DEVICE is not set to test detection
        env::remove_var("WHISPER_DEVICE");

        // First call should trigger detection
        let device1 = WhisperPluginFactory::detect_device();

        // Second call should return cached result without re-running detection
        let device2 = WhisperPluginFactory::detect_device();

        // Both calls should return the same result
        assert_eq!(device1, device2);

        // Verify the device is either "cuda" or "cpu"
        assert!(device1 == "cuda" || device1 == "cpu");
    }

    #[test]
    fn whisper_device_env_var_overrides_cache() {
        // Set WHISPER_DEVICE to override detection
        env::set_var("WHISPER_DEVICE", "cuda:1");

        let factory = WhisperPluginFactory::new();
        assert_eq!(factory.device, "cuda:1");

        env::remove_var("WHISPER_DEVICE");
    }

    #[test]
    fn gpu_detection_thread_safety() {
        use std::thread;

        // Ensure WHISPER_DEVICE is not set to test detection
        env::remove_var("WHISPER_DEVICE");

        let handles: Vec<_> = (0..10)
            .map(|_| thread::spawn(WhisperPluginFactory::detect_device))
            .collect();

        // All threads should get the same result
        let results: Vec<String> = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect();

        // All results should be identical
        let first_result = &results[0];
        assert!(results.iter().all(|r| r == first_result));

        // Verify the device is either "cuda" or "cpu"
        assert!(first_result == "cuda" || first_result == "cpu");
    }
}
