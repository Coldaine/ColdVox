//! Parakeet STT plugin implementation using NVIDIA's Parakeet model via parakeet-rs.
//!
//! This plugin provides GPU-accelerated transcription using the NVIDIA Parakeet model.
//! It requires a CUDA-capable GPU and provides automatic CPU fallback at the ONNX Runtime level.
//!
//! # Model Variants
//!
//! Two separate model types are available (different APIs):
//! - **Parakeet CTC**: 600M parameters, English-only, faster inference (uses `Parakeet` struct)
//! - **Parakeet TDT**: 1.1B parameters, multilingual with 25 languages (uses `ParakeetTDT` struct)
//!
//! This plugin currently implements the **CTC variant** for simplicity. TDT support can be added later.
//!
//! Environment variables:
//! - `PARAKEET_MODEL_PATH`: Path to model directory (required - must be downloaded manually)
//! - `PARAKEET_DEVICE`: "cuda" (default) or "tensorrt" (optimized)
//!
//! # GPU Support
//!
//! While GPU is strongly preferred, parakeet-rs automatically falls back to CPU if CUDA fails.
//! We check for nvidia-smi to verify GPU availability before initialization.

use crate::plugin::*;
use crate::types::{TranscriptionConfig, TranscriptionEvent, WordInfo};
use async_trait::async_trait;
use coldvox_foundation::error::{ColdVoxError, SttError};
use std::env;
use std::path::{Path, PathBuf};
use tracing::{debug, error, info, warn};

#[cfg(feature = "parakeet")]
use parakeet_rs::{ExecutionConfig, ExecutionProvider, Parakeet, TimestampMode};

/// GPU execution provider type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuProvider {
    /// CUDA execution (default)
    Cuda,
    /// TensorRT execution (optimized, preferred if available)
    TensorRt,
}

impl GpuProvider {
    #[cfg(feature = "parakeet")]
    fn to_execution_provider(&self) -> ExecutionProvider {
        match self {
            Self::Cuda => ExecutionProvider::Cuda,
            // TensorRT requires separate feature flag in parakeet-rs
            // Fall back to CUDA if TensorRT not available
            Self::TensorRt => {
                #[cfg(feature = "tensorrt")]
                {
                    ExecutionProvider::TensorRT
                }
                #[cfg(not(feature = "tensorrt"))]
                {
                    warn!(
                        target: "coldvox::stt::parakeet",
                        "TensorRT requested but not compiled. Falling back to CUDA."
                    );
                    ExecutionProvider::Cuda
                }
            }
        }
    }
}

/// Parakeet-based STT plugin backed by parakeet-rs (CTC variant)
pub struct ParakeetPlugin {
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

// Manual Debug implementation since Parakeet doesn't implement Debug
impl std::fmt::Debug for ParakeetPlugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParakeetPlugin")
            .field("model_path", &self.model_path)
            .field("gpu_provider", &self.gpu_provider)
            .field("initialized", &self.initialized)
            .field("model", &"<Parakeet>")
            .finish()
    }
}

impl ParakeetPlugin {
    pub fn new() -> Self {
        Self {
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
        // 4. Error - user must provide model directory

        let path_candidate = if !config.model_path.is_empty() {
            Some(PathBuf::from(&config.model_path))
        } else {
            self.model_path.clone()
        };

        if let Some(path) = path_candidate {
            if path.exists() {
                return Ok(path);
            } else {
                return Err(SttError::LoadFailed(format!(
                    "Parakeet model path does not exist: {}. Please download model files.",
                    path.display()
                ))
                .into());
            }
        }

        // Check env var
        if let Ok(env_path) = env::var("PARAKEET_MODEL_PATH") {
            let path = PathBuf::from(env_path);
            if path.exists() {
                return Ok(path);
            } else {
                return Err(SttError::LoadFailed(format!(
                    "PARAKEET_MODEL_PATH does not exist: {}. Please download model files.",
                    path.display()
                ))
                .into());
            }
        }

        Err(SttError::LoadFailed(
            "No Parakeet model path configured. Set PARAKEET_MODEL_PATH or provide model_path in config.".to_string()
        )
        .into())
    }

    /// Verify GPU is available (CUDA preferred but not required)
    #[cfg(feature = "parakeet")]
    fn verify_gpu_available() -> Result<(), ColdVoxError> {
        // Check for nvidia-smi to verify CUDA is available
        let output = std::process::Command::new("nvidia-smi")
            .output()
            .map_err(|e| {
                warn!(
                    target: "coldvox::stt::parakeet",
                    error = %e,
                    "nvidia-smi not found - CUDA GPU may not be available. Will fallback to CPU."
                );
                SttError::LoadFailed(format!(
                    "nvidia-smi not found: {}. GPU preferred but will fallback to CPU if needed.",
                    e
                ))
            })?;

        if !output.status.success() {
            warn!(
                target: "coldvox::stt::parakeet",
                "nvidia-smi check failed - CUDA GPU may not be available. Will fallback to CPU."
            );
            return Err(SttError::LoadFailed(
                "nvidia-smi check failed. GPU preferred but will fallback to CPU if needed.".to_string(),
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
            name: "NVIDIA Parakeet CTC (GPU-accelerated)".to_string(),
            description: "GPU-accelerated transcription via parakeet-rs (CUDA/TensorRT, with CPU fallback)"
                .to_string(),
            requires_network: false, // Model must be downloaded manually
            is_local: true,
            is_available: check_parakeet_available(),
            supported_languages: vec!["en".to_string()], // CTC variant is English-only
            memory_usage_mb: Some(2500), // ~600M parameters ≈ 2.5GB
        }
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            streaming: false, // Batch processing only
            batch: true,
            word_timestamps: true, // parakeet-rs provides token-level timestamps
            confidence_scores: false, // parakeet-rs does NOT provide confidence scores
            speaker_diarization: false,
            auto_punctuation: true, // CTC variant includes punctuation
            custom_vocabulary: false,
        }
    }

    async fn is_available(&self) -> Result<bool, ColdVoxError> {
        Ok(check_parakeet_available())
    }

    async fn initialize(&mut self, config: TranscriptionConfig) -> Result<(), ColdVoxError> {
        #[cfg(feature = "parakeet")]
        {
            // Verify GPU is available (warning only, not fatal)
            let _ = Self::verify_gpu_available();

            let model_path = self.resolve_model_path(&config)?;

            info!(
                target: "coldvox::stt::parakeet",
                model_path = %model_path.display(),
                gpu_provider = ?self.gpu_provider,
                "Initializing Parakeet CTC model"
            );

            // Build parakeet-rs execution configuration
            let exec_config = ExecutionConfig::new()
                .with_execution_provider(self.gpu_provider.to_execution_provider())
                .with_intra_threads(4)
                .with_inter_threads(1);

            // Initialize the model from local directory
            let model = Parakeet::from_pretrained(&model_path, Some(exec_config))
                .map_err(|err| {
                    error!(
                        target: "coldvox::stt::parakeet",
                        error = %err,
                        "Failed to load Parakeet model"
                    );
                    SttError::LoadFailed(format!(
                        "Failed to load Parakeet model: {}. Ensure model files exist at {}",
                        err,
                        model_path.display()
                    ))
                })?;

            self.model = Some(model);
            self.audio_buffer.clear();
            self.active_config = Some(config);
            self.initialized = true;

            info!(
                target: "coldvox::stt::parakeet",
                "Parakeet plugin initialized successfully"
            );

            Ok(())
        }

        #[cfg(not(feature = "parakeet"))]
        {
            let _ = config;
            Err(SttError::NotAvailable {
                plugin: "parakeet".to_string(),
                reason: "Parakeet feature not compiled. Build with --features parakeet".to_string(),
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

            // Buffer audio samples - parakeet-rs only supports batch processing
            self.audio_buffer.extend_from_slice(samples);
            Ok(None) // Return results on finalize only
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

            // Convert i16 samples to f32 for parakeet-rs (normalize to [-1.0, 1.0])
            let samples_f32: Vec<f32> = self
                .audio_buffer
                .iter()
                .map(|&s| s as f32 / 32768.0)
                .collect();

            // Transcribe using parakeet-rs
            let result = self
                .model
                .as_mut()
                .ok_or_else(|| {
                    SttError::TranscriptionFailed("Parakeet model not loaded".to_string())
                })?
                .transcribe_samples(
                    samples_f32,
                    crate::constants::SAMPLE_RATE_HZ,
                    1, // mono
                    Some(TimestampMode::Words),
                )
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
                            conf: 1.0, // Placeholder - parakeet-rs doesn't provide confidence
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
    model_path: Option<PathBuf>,
    gpu_provider: GpuProvider,
}

impl ParakeetPluginFactory {
    pub fn new() -> Self {
        // Check environment variables for configuration
        let gpu_provider = env::var("PARAKEET_DEVICE")
            .ok()
            .and_then(|d| match d.to_lowercase().as_str() {
                "tensorrt" => Some(GpuProvider::TensorRt),
                "cuda" => Some(GpuProvider::Cuda),
                _ => {
                    warn!(
                        target: "coldvox::stt::parakeet",
                        "Invalid PARAKEET_DEVICE: {}. Using CUDA by default.", d
                    );
                    None
                }
            })
            .unwrap_or(GpuProvider::Cuda);

        Self {
            model_path: env::var("PARAKEET_MODEL_PATH").ok().map(PathBuf::from),
            gpu_provider,
        }
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
            .with_gpu_provider(self.gpu_provider);

        if let Some(ref path) = self.model_path {
            plugin = plugin.with_model_path(path.clone());
        }

        Ok(Box::new(plugin))
    }

    fn plugin_info(&self) -> PluginInfo {
        ParakeetPlugin::new()
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

        // Verify GPU availability (warning only, not fatal)
        #[cfg(feature = "parakeet")]
        let _ = ParakeetPlugin::verify_gpu_available();

        if let Some(ref path) = self.model_path {
            if !path.exists() {
                return Err(SttError::LoadFailed(format!(
                    "Parakeet model path does not exist: {}. Please download model files.",
                    path.display()
                ))
                .into());
            }
        }

        Ok(())
    }
}

#[cfg(feature = "parakeet")]
fn check_parakeet_available() -> bool {
    // Parakeet feature is compiled
    true
}

#[cfg(not(feature = "parakeet"))]
fn check_parakeet_available() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_info_is_valid() {
        let plugin = ParakeetPlugin::new();
        let info = plugin.info();
        assert_eq!(info.id, "parakeet");
        assert!(info.description.contains("GPU"));
    }

    #[test]
    fn capabilities_are_correct() {
        let plugin = ParakeetPlugin::new();
        let caps = plugin.capabilities();
        assert!(!caps.streaming); // Batch only
        assert!(caps.batch);
        assert!(caps.word_timestamps);
        assert!(!caps.confidence_scores); // parakeet-rs doesn't provide confidence
    }

    #[test]
    fn factory_respects_env_vars() {
        env::set_var("PARAKEET_DEVICE", "tensorrt");

        let factory = ParakeetPluginFactory::new();
        assert_eq!(factory.gpu_provider, GpuProvider::TensorRt);

        env::remove_var("PARAKEET_DEVICE");
    }
}
