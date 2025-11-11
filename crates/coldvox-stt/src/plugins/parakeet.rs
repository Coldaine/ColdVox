//! Parakeet STT plugin implementation using NVIDIA's Parakeet model via parakeet-rs.
//!
//! This plugin provides GPU-accelerated transcription using the largest available
//! Parakeet model (nvidia/parakeet-tdt-1.1b). It requires a CUDA-capable GPU and
//! does not fallback to CPU execution.
//!
//! # GPU-Only Philosophy
//!
//! This plugin is designed for high-performance GPU-only transcription:
//! - Requires CUDA execution provider
//! - No CPU fallback - fails if GPU unavailable
//! - Optimized for the largest model (1.1B parameters)
//! - TensorRT execution when available
//!
//! # Model Variants
//!
//! - **TDT (default)**: nvidia/parakeet-tdt-1.1b - Multilingual (25 languages), auto-detection
//! - **CTC**: nvidia/parakeet-ctc-1.1b - English-only, faster inference
//!
//! Environment variables:
//! - `PARAKEET_MODEL_PATH`: Override model path (default: auto-download)
//! - `PARAKEET_VARIANT`: "tdt" or "ctc" (default: "tdt")
//! - `PARAKEET_DEVICE`: Must be "cuda" or "tensorrt" (CPU not supported)

use crate::plugin::*;
use crate::types::{TranscriptionConfig, TranscriptionEvent, WordInfo};
use async_trait::async_trait;
use coldvox_foundation::error::{ColdVoxError, SttError};
use std::env;
use std::path::{Path, PathBuf};
use tracing::{debug, error, info, warn};

#[cfg(feature = "parakeet")]
use parakeet_rs::{ExecutionProvider, Parakeet, ParakeetConfig, ParakeetVariant};

/// Parakeet model variant
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParakeetModelVariant {
    /// TDT (Token-and-Duration Transducer) - Multilingual, 25 languages with auto-detection
    Tdt,
    /// CTC (Connectionist Temporal Classification) - English-only, faster
    Ctc,
}

impl ParakeetModelVariant {
    fn model_identifier(&self) -> &'static str {
        match self {
            Self::Tdt => "nvidia/parakeet-tdt-1.1b",
            Self::Ctc => "nvidia/parakeet-ctc-1.1b",
        }
    }

    fn memory_usage_mb(&self) -> u32 {
        // Both 1.1B parameter models have similar memory requirements
        // Approximate: 1.1B * 4 bytes (fp32) ≈ 4.4GB + overhead
        5000 // 5GB to be safe
    }

    #[cfg(feature = "parakeet")]
    fn to_parakeet_variant(&self) -> ParakeetVariant {
        match self {
            Self::Tdt => ParakeetVariant::Tdt,
            Self::Ctc => ParakeetVariant::Ctc,
        }
    }
}

impl Default for ParakeetModelVariant {
    fn default() -> Self {
        // Default to TDT for multilingual support
        Self::Tdt
    }
}

/// GPU execution provider type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuProvider {
    /// CUDA execution (required)
    Cuda,
    /// TensorRT execution (optimized, preferred if available)
    TensorRt,
}

impl GpuProvider {
    #[cfg(feature = "parakeet")]
    fn to_execution_provider(&self) -> ExecutionProvider {
        match self {
            Self::Cuda => ExecutionProvider::Cuda,
            Self::TensorRt => ExecutionProvider::TensorRt,
        }
    }
}

/// Parakeet-based STT plugin backed by parakeet-rs
#[derive(Debug)]
pub struct ParakeetPlugin {
    variant: ParakeetModelVariant,
    model_path: Option<PathBuf>,
    gpu_provider: GpuProvider,
    initialized: bool,
    #[cfg(feature = "parakeet")]
    model: Option<Parakeet>,
    #[cfg(feature = "parakeet")]
    audio_buffer: Vec<i16>,
    #[cfg(feature = "parakeet")]
    active_config: Option<TranscriptionConfig>,
}

impl ParakeetPlugin {
    pub fn new() -> Self {
        Self {
            variant: ParakeetModelVariant::default(),
            model_path: None,
            gpu_provider: GpuProvider::Cuda, // Default to CUDA
            initialized: false,
            #[cfg(feature = "parakeet")]
            model: None,
            #[cfg(feature = "parakeet")]
            audio_buffer: Vec::new(),
            #[cfg(feature = "parakeet")]
            active_config: None,
        }
    }

    pub fn with_variant(mut self, variant: ParakeetModelVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn with_model_path(mut self, path: PathBuf) -> Self {
        self.model_path = Some(path);
        self
    }

    pub fn with_gpu_provider(mut self, provider: GpuProvider) -> Self {
        self.gpu_provider = provider;
        self
    }

    #[cfg(feature = "parakeet")]
    fn resolve_model_path(&self, config: &TranscriptionConfig) -> Result<PathBuf, ColdVoxError> {
        // Priority:
        // 1. Config model_path (if set and exists)
        // 2. Plugin model_path (if set and exists)
        // 3. PARAKEET_MODEL_PATH env var (if set and exists)
        // 4. Auto-download to cache (parakeet-rs handles this)

        let path_candidate = if !config.model_path.is_empty() {
            Some(PathBuf::from(&config.model_path))
        } else {
            self.model_path.clone()
        };

        if let Some(path) = path_candidate {
            if path.exists() {
                return Ok(path);
            }

            warn!(
                target: "coldvox::stt::parakeet",
                candidate = %path.display(),
                "Configured Parakeet model path does not exist; will auto-download"
            );
        }

        // Return cache directory - parakeet-rs will download to ~/.cache/parakeet/
        let cache_dir = dirs::cache_dir()
            .ok_or_else(|| SttError::LoadFailed("Cannot determine cache directory".to_string()))?
            .join("parakeet");

        std::fs::create_dir_all(&cache_dir)
            .map_err(|e| SttError::LoadFailed(format!("Failed to create cache dir: {}", e)))?;

        Ok(cache_dir.join(self.variant.model_identifier()))
    }

    /// Verify GPU is available (CUDA required)
    #[cfg(feature = "parakeet")]
    fn verify_gpu_available() -> Result<(), ColdVoxError> {
        // Check for nvidia-smi to verify CUDA is available
        let output = std::process::Command::new("nvidia-smi")
            .output()
            .map_err(|e| {
                SttError::LoadFailed(format!(
                    "GPU-only mode: nvidia-smi not found or failed: {}. CUDA GPU required.",
                    e
                ))
            })?;

        if !output.status.success() {
            return Err(SttError::LoadFailed(
                "GPU-only mode: nvidia-smi check failed. CUDA GPU required.".to_string(),
            )
            .into());
        }

        info!(
            target: "coldvox::stt::parakeet",
            "GPU verification passed - CUDA GPU detected"
        );

        Ok(())
    }
}

impl Default for ParakeetPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SttPlugin for ParakeetPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: "parakeet".to_string(),
            name: format!("NVIDIA Parakeet {} (GPU-only)", self.variant.model_identifier()),
            description: "GPU-accelerated transcription via parakeet-rs (CUDA/TensorRT required)"
                .to_string(),
            requires_network: false, // Model downloads on first use, then cached
            is_local: true,
            is_available: check_parakeet_available(),
            supported_languages: match self.variant {
                ParakeetModelVariant::Tdt => vec![
                    "auto".to_string(),
                    "en".to_string(),
                    "es".to_string(),
                    "fr".to_string(),
                    "de".to_string(),
                    "it".to_string(),
                    "pt".to_string(),
                    "pl".to_string(),
                    "tr".to_string(),
                    "ru".to_string(),
                    "nl".to_string(),
                    "cs".to_string(),
                    "ar".to_string(),
                    "zh".to_string(),
                    "ja".to_string(),
                    "hu".to_string(),
                    "ko".to_string(),
                ],
                ParakeetModelVariant::Ctc => vec!["en".to_string()],
            },
            memory_usage_mb: Some(self.variant.memory_usage_mb()),
        }
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            streaming: false, // Batch processing only for now
            batch: true,
            word_timestamps: true, // parakeet-rs provides token-level timestamps
            confidence_scores: true,
            speaker_diarization: false, // Can be added later via pyannote
            auto_punctuation: true, // Both variants support punctuation
            custom_vocabulary: false,
        }
    }

    async fn is_available(&self) -> Result<bool, ColdVoxError> {
        Ok(check_parakeet_available())
    }

    async fn initialize(&mut self, config: TranscriptionConfig) -> Result<(), ColdVoxError> {
        #[cfg(feature = "parakeet")]
        {
            // Verify GPU is available (REQUIRED)
            Self::verify_gpu_available()?;

            let model_path = self.resolve_model_path(&config)?;

            info!(
                target: "coldvox::stt::parakeet",
                variant = ?self.variant,
                model_path = %model_path.display(),
                gpu_provider = ?self.gpu_provider,
                "Initializing Parakeet model (GPU-only)"
            );

            // Build parakeet-rs configuration
            let parakeet_config = ParakeetConfig {
                variant: self.variant.to_parakeet_variant(),
                execution_provider: self.gpu_provider.to_execution_provider(),
                // Auto-download from HuggingFace if not in cache
                model_path: Some(model_path.to_string_lossy().to_string()),
            };

            // Initialize the model
            let model = Parakeet::from_pretrained(self.variant.model_identifier(), Some(parakeet_config))
                .map_err(|err| {
                    error!(
                        target: "coldvox::stt::parakeet",
                        error = %err,
                        "Failed to load Parakeet model"
                    );
                    SttError::LoadFailed(format!(
                        "Failed to load Parakeet model: {}. Ensure CUDA GPU is available.",
                        err
                    ))
                })?;

            self.model = Some(model);
            self.audio_buffer.clear();
            self.active_config = Some(config);
            self.initialized = true;

            info!(
                target: "coldvox::stt::parakeet",
                "Parakeet plugin initialized successfully (GPU-only mode)"
            );

            Ok(())
        }

        #[cfg(not(feature = "parakeet"))]
        {
            let _ = config;
            Err(SttError::NotAvailable {
                plugin: "parakeet".to_string(),
                reason: "Parakeet feature not compiled".to_string(),
            }
            .into())
        }
    }

    async fn process_audio(
        &mut self,
        samples: &[i16],
    ) -> Result<Option<TranscriptionEvent>, ColdVoxError> {
        #[cfg(feature = "parakeet")]
        {
            if !self.initialized {
                return Err(SttError::NotAvailable {
                    plugin: "parakeet".to_string(),
                    reason: "Parakeet plugin not initialized".to_string(),
                }
                .into());
            }

            // Buffer audio samples
            self.audio_buffer.extend_from_slice(samples);
            Ok(None) // Return partial results on finalize only
        }

        #[cfg(not(feature = "parakeet"))]
        {
            let _ = samples;
            Err(SttError::NotAvailable {
                plugin: "parakeet".to_string(),
                reason: "Parakeet feature not compiled".to_string(),
            }
            .into())
        }
    }

    async fn finalize(&mut self) -> Result<Option<TranscriptionEvent>, ColdVoxError> {
        #[cfg(feature = "parakeet")]
        {
            if !self.initialized {
                return Ok(None);
            }

            if self.audio_buffer.is_empty() {
                return Ok(None);
            }

            let buffer_size = self.audio_buffer.len();
            info!(
                target: "coldvox::stt::parakeet",
                "Transcribing buffered audio: {} samples ({:.2}s)",
                buffer_size,
                buffer_size as f32 / crate::constants::SAMPLE_RATE_HZ as f32
            );

            // Convert i16 samples to f32 for parakeet-rs
            let samples_f32: Vec<f32> = self
                .audio_buffer
                .iter()
                .map(|&s| s as f32 / 32768.0)
                .collect();

            // Transcribe using parakeet-rs
            let result = self
                .model
                .as_ref()
                .ok_or_else(|| {
                    SttError::TranscriptionFailed("Parakeet model not loaded".to_string())
                })?
                .transcribe_samples(&samples_f32, crate::constants::SAMPLE_RATE_HZ)
                .map_err(|err| {
                    error!(
                        target: "coldvox::stt::parakeet",
                        error = %err,
                        "Parakeet transcription failed"
                    );
                    SttError::TranscriptionFailed(format!("Parakeet transcription failed: {}", err))
                })?;

            let text = result.text.trim().to_string();

            debug!(
                target: "coldvox::stt::parakeet",
                text = %text,
                token_count = result.tokens.len(),
                "Parakeet transcription complete"
            );

            // Extract word-level information from tokens if requested
            let include_words = self
                .active_config
                .as_ref()
                .map(|cfg| cfg.include_words)
                .unwrap_or(false);

            let words = if include_words && !result.tokens.is_empty() {
                Some(
                    result
                        .tokens
                        .iter()
                        .filter(|token| !token.text.trim().is_empty())
                        .map(|token| WordInfo {
                            start: token.start,
                            end: token.end,
                            conf: token.confidence.unwrap_or(1.0), // Default to high confidence if not provided
                            text: token.text.clone(),
                        })
                        .collect(),
                )
            } else {
                None
            };

            self.audio_buffer.clear();

            Ok(Some(TranscriptionEvent::Final {
                utterance_id: 0,
                text,
                words,
            }))
        }

        #[cfg(not(feature = "parakeet"))]
        {
            Err(SttError::NotAvailable {
                plugin: "parakeet".to_string(),
                reason: "Parakeet feature not compiled".to_string(),
            }
            .into())
        }
    }

    async fn reset(&mut self) -> Result<(), ColdVoxError> {
        #[cfg(feature = "parakeet")]
        {
            self.audio_buffer.clear();
            Ok(())
        }

        #[cfg(not(feature = "parakeet"))]
        {
            Err(SttError::NotAvailable {
                plugin: "parakeet".to_string(),
                reason: "Parakeet feature not compiled".to_string(),
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
        #[cfg(feature = "parakeet")]
        {
            self.model = None;
            self.audio_buffer.clear();
            self.initialized = false;
            Ok(())
        }

        #[cfg(not(feature = "parakeet"))]
        {
            Err(SttError::NotAvailable {
                plugin: "parakeet".to_string(),
                reason: "Parakeet feature not compiled".to_string(),
            }
            .into())
        }
    }
}

/// Factory for creating ParakeetPlugin instances
pub struct ParakeetPluginFactory {
    variant: ParakeetModelVariant,
    model_path: Option<PathBuf>,
    gpu_provider: GpuProvider,
}

impl ParakeetPluginFactory {
    pub fn new() -> Self {
        // Check environment variables for configuration
        let variant = env::var("PARAKEET_VARIANT")
            .ok()
            .and_then(|v| match v.to_lowercase().as_str() {
                "ctc" => Some(ParakeetModelVariant::Ctc),
                "tdt" => Some(ParakeetModelVariant::Tdt),
                _ => {
                    warn!(
                        target: "coldvox::stt::parakeet",
                        "Invalid PARAKEET_VARIANT: {}, using default TDT", v
                    );
                    None
                }
            })
            .unwrap_or_default();

        let gpu_provider = env::var("PARAKEET_DEVICE")
            .ok()
            .and_then(|d| match d.to_lowercase().as_str() {
                "tensorrt" => Some(GpuProvider::TensorRt),
                "cuda" => Some(GpuProvider::Cuda),
                _ => {
                    warn!(
                        target: "coldvox::stt::parakeet",
                        "Invalid PARAKEET_DEVICE: {}. GPU-only mode requires 'cuda' or 'tensorrt'.", d
                    );
                    None
                }
            })
            .unwrap_or(GpuProvider::Cuda);

        Self {
            variant,
            model_path: env::var("PARAKEET_MODEL_PATH").ok().map(PathBuf::from),
            gpu_provider,
        }
    }

    pub fn with_variant(mut self, variant: ParakeetModelVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn with_model_path(mut self, path: PathBuf) -> Self {
        self.model_path = Some(path);
        self
    }

    pub fn with_gpu_provider(mut self, provider: GpuProvider) -> Self {
        self.gpu_provider = provider;
        self
    }
}

impl Default for ParakeetPluginFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl SttPluginFactory for ParakeetPluginFactory {
    fn create(&self) -> Result<Box<dyn SttPlugin>, ColdVoxError> {
        let mut plugin = ParakeetPlugin::new()
            .with_variant(self.variant)
            .with_gpu_provider(self.gpu_provider);

        if let Some(ref path) = self.model_path {
            plugin = plugin.with_model_path(path.clone());
        }

        Ok(Box::new(plugin))
    }

    fn plugin_info(&self) -> PluginInfo {
        ParakeetPlugin::new()
            .with_variant(self.variant)
            .with_gpu_provider(self.gpu_provider)
            .info()
    }

    fn check_requirements(&self) -> Result<(), ColdVoxError> {
        if !check_parakeet_available() {
            return Err(SttError::NotAvailable {
                plugin: "parakeet".to_string(),
                reason: "Parakeet feature not compiled. Build with --features parakeet".to_string(),
            }
            .into());
        }

        // Verify GPU availability
        #[cfg(feature = "parakeet")]
        ParakeetPlugin::verify_gpu_available()?;

        if let Some(ref path) = self.model_path {
            if !path.exists() {
                warn!(
                    target: "coldvox::stt::parakeet",
                    path = %path.display(),
                    "Model path does not exist; will auto-download on first use"
                );
            }
        }

        Ok(())
    }
}

#[cfg(feature = "parakeet")]
fn check_parakeet_available() -> bool {
    // Check if CUDA is available via nvidia-smi
    std::process::Command::new("nvidia-smi")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[cfg(not(feature = "parakeet"))]
fn check_parakeet_available() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_variant_identifiers() {
        assert_eq!(
            ParakeetModelVariant::Tdt.model_identifier(),
            "nvidia/parakeet-tdt-1.1b"
        );
        assert_eq!(
            ParakeetModelVariant::Ctc.model_identifier(),
            "nvidia/parakeet-ctc-1.1b"
        );
    }

    #[test]
    fn default_variant_is_tdt() {
        assert_eq!(ParakeetModelVariant::default(), ParakeetModelVariant::Tdt);
    }

    #[test]
    fn memory_usage_estimation() {
        // 1.1B parameters should require ~5GB
        assert!(ParakeetModelVariant::Tdt.memory_usage_mb() >= 4000);
        assert!(ParakeetModelVariant::Ctc.memory_usage_mb() >= 4000);
    }

    #[test]
    fn plugin_info_contains_gpu_requirement() {
        let plugin = ParakeetPlugin::new();
        let info = plugin.info();
        assert!(info.description.contains("GPU"));
        assert!(info.description.contains("CUDA") || info.description.contains("TensorRT"));
    }

    #[test]
    fn factory_respects_env_vars() {
        env::set_var("PARAKEET_VARIANT", "ctc");
        env::set_var("PARAKEET_DEVICE", "tensorrt");

        let factory = ParakeetPluginFactory::new();
        assert_eq!(factory.variant, ParakeetModelVariant::Ctc);
        assert_eq!(factory.gpu_provider, GpuProvider::TensorRt);

        env::remove_var("PARAKEET_VARIANT");
        env::remove_var("PARAKEET_DEVICE");
    }
}
