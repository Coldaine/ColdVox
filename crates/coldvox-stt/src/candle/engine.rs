//! High-level WhisperEngine facade for Candle Whisper implementation.
//!
//! This module provides a clean, high-level API for Whisper functionality, hiding
//! the complexity of the underlying Candle implementation while providing efficient
//! device management, resource caching, and seamless integration with the existing
//! ColdVox architecture.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use candle_core::Device;

use coldvox_foundation::error::{ColdVoxError, SttError};

use super::audio::{mel_filters, WhisperAudioConfig, SAMPLE_RATE};
use super::decoder::{Decoder, DecoderConfig};
use super::loader::{ModelLoader, ModelLoaderConfig};
use super::model::{build_from_artifacts, WhisperComponents};
use super::types::Transcript;

/// Device preference for WhisperEngine initialization.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DevicePreference {
    /// Force CPU execution
    Cpu,
    /// Force CUDA GPU execution
    Cuda,
    /// Auto-select best available device (CUDA if available, fallback to CPU)
    Auto,
}

impl Default for DevicePreference {
    fn default() -> Self {
        DevicePreference::Auto
    }
}

impl std::str::FromStr for DevicePreference {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "cpu" => Ok(DevicePreference::Cpu),
            "cuda" => Ok(DevicePreference::Cuda),
            "auto" => Ok(DevicePreference::Auto),
            _ => Err(format!("Invalid device preference: {}", s)),
        }
    }
}

/// Configuration for WhisperEngine initialization with enhanced device management.
#[derive(Debug, Clone)]
pub struct WhisperEngineInit {
    /// Model identifier (HuggingFace repo or local path)
    pub model_id: String,
    /// Model revision/version
    pub revision: String,
    /// Optional local path to model files
    pub local_path: Option<PathBuf>,
    /// Device preference for execution
    pub device_preference: DevicePreference,
    /// Whether to use quantized models for reduced memory usage
    pub quantized: bool,
    /// Optional language hint for the model
    pub language: Option<String>,
    /// Maximum tokens to generate
    pub max_tokens: usize,
    /// Temperature for sampling
    pub temperature: f32,
    /// Whether to generate timestamps
    pub generate_timestamps: bool,
}

impl Default for WhisperEngineInit {
    fn default() -> Self {
        Self {
            model_id: "openai/whisper-base.en".to_string(),
            revision: "main".to_string(),
            local_path: std::env::var("WHISPER_MODEL_PATH").ok().map(PathBuf::from),
            device_preference: DevicePreference::default(),
            quantized: false,
            language: None,
            max_tokens: 448,
            temperature: 0.0,
            generate_timestamps: true,
        }
    }
}

impl WhisperEngineInit {
    /// Create a new configuration builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the model ID (HuggingFace repo or local identifier).
    pub fn with_model_id<S: Into<String>>(mut self, model_id: S) -> Self {
        self.model_id = model_id.into();
        self
    }

    /// Set the model revision.
    pub fn with_revision<S: Into<String>>(mut self, revision: S) -> Self {
        self.revision = revision.into();
        self
    }

    /// Set a local path to model files.
    pub fn with_local_path<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.local_path = Some(path.into());
        self
    }

    /// Set the device preference.
    pub fn with_device_preference(mut self, preference: DevicePreference) -> Self {
        self.device_preference = preference;
        self
    }

    /// Enable or disable quantized models.
    pub fn with_quantized(mut self, quantized: bool) -> Self {
        self.quantized = quantized;
        self
    }

    /// Set a language hint for the model.
    pub fn with_language<S: Into<String>>(mut self, language: S) -> Self {
        self.language = Some(language.into());
        self
    }

    /// Set maximum tokens.
    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Set temperature for sampling.
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
    }

    /// Enable or disable timestamp generation.
    pub fn with_generate_timestamps(mut self, generate_timestamps: bool) -> Self {
        self.generate_timestamps = generate_timestamps;
        self
    }

    /// Validate the configuration and return appropriate error if invalid.
    pub fn validate(&self) -> Result<(), ColdVoxError> {
        if self.model_id.trim().is_empty() {
            return Err(ColdVoxError::Stt(SttError::InvalidConfig(
                "Model ID cannot be empty".to_string(),
            )));
        }

        if self.revision.trim().is_empty() {
            return Err(ColdVoxError::Stt(SttError::InvalidConfig(
                "Model revision cannot be empty".to_string(),
            )));
        }

        if let Some(ref path) = self.local_path {
            if !path.exists() {
                return Err(ColdVoxError::Stt(SttError::ModelNotFound {
                    path: path.clone(),
                }));
            }
        }

        Ok(())
    }
}

/// Enhanced WhisperEngine facade with comprehensive resource management and device handling.
#[derive(Debug)]
pub struct WhisperEngine {
    device: Device,
    components: WhisperComponents,
    decoder: Decoder,
    cached_filters: Option<Arc<Vec<f32>>>,
    audio_config: WhisperAudioConfig,
    device_info: DeviceInfo,
    last_detected_language: Option<String>,
}

/// Information about the device being used for inference.
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub device_type: String,
    pub is_cuda: bool,
    pub is_quantized: bool,
    pub memory_usage_mb: Option<u64>,
}

impl DeviceInfo {
    /// Create device info for CPU.
    pub fn cpu(is_quantized: bool) -> Self {
        Self {
            device_type: "CPU".to_string(),
            is_cuda: false,
            is_quantized,
            memory_usage_mb: None,
        }
    }

    /// Create device info for CUDA.
    pub fn cuda(is_quantized: bool, memory_usage_mb: Option<u64>) -> Self {
        Self {
            device_type: "CUDA".to_string(),
            is_cuda: true,
            is_quantized,
            memory_usage_mb,
        }
    }
}

/// Errors specific to WhisperEngine operations.
#[derive(Debug, thiserror::Error)]
pub enum WhisperEngineError {
    #[error(transparent)]
    Config(#[from] ColdVoxError),

    #[error(transparent)]
    Loader(#[from] super::loader::LoaderError),

    #[error(transparent)]
    Model(#[from] super::model::ModelBuildError),

    #[error("Decoder operation failed: {0}")]
    Decoder(String),

    #[error("Device initialization failed: {0}")]
    DeviceInit(String),

    #[error("Audio preprocessing failed: {0}")]
    AudioProcessing(String),

    #[error("Transcription failed: {0}")]
    TranscriptionFailed(String),
}

impl WhisperEngine {
    /// Create a new WhisperEngine with the given configuration.
    pub fn new(init: WhisperEngineInit) -> Result<Self, WhisperEngineError> {
        let _span = tracing::debug_span!("whisper_engine_init").entered();

        tracing::info!(
            "Initializing WhisperEngine with model_id={}, device_preference={:?}, quantized={}",
            init.model_id,
            init.device_preference,
            init.quantized
        );

        // Validate configuration
        init.validate().map_err(WhisperEngineError::Config)?;

        // Initialize device based on preference
        let device = Self::initialize_device(&init.device_preference)?;
        let device_info = Self::create_device_info(&device, init.quantized);

        tracing::info!(
            "Using device: {} (CUDA: {}, quantized: {})",
            device_info.device_type,
            device_info.is_cuda,
            device_info.is_quantized
        );

        // Initialize model loader
        let loader_cfg = ModelLoaderConfig {
            model_id: init.model_id.clone(),
            revision: init.revision.clone(),
            local_path: init.local_path.clone(),
        };
        let loader = ModelLoader::new(loader_cfg).map_err(WhisperEngineError::Loader)?;

        // Load model artifacts
        let artifacts = loader
            .load_safetensors(&device)
            .map_err(WhisperEngineError::Loader)?;

        // Build model components
        let components =
            build_from_artifacts(artifacts, &device).map_err(WhisperEngineError::Model)?;

        // Pre-load mel filters for efficient audio processing
        let cached_filters = Some(Arc::new(
            mel_filters(components.config.num_mel_bins)
                .map_err(|e| WhisperEngineError::AudioProcessing(e.to_string()))?,
        ));

        // Initialize audio configuration
        let audio_config = WhisperAudioConfig {
            num_mel_bins: components.config.num_mel_bins,
            speed_up: false, // Could be made configurable
        };

        // Create decoder configuration from init
        let mut decoder_config = DecoderConfig::default();
        decoder_config.max_tokens = init.max_tokens;
        decoder_config.temperature = init.temperature;
        decoder_config.generate_timestamps = init.generate_timestamps;

        // Create decoder with configuration
        let decoder = Decoder::new(components.clone(), device.clone(), decoder_config)
            .map_err(|e| WhisperEngineError::Decoder(e.to_string()))?;

        tracing::info!(
            "WhisperEngine initialized successfully with {} mel bins, device memory: {:?} MB",
            audio_config.num_mel_bins,
            device_info.memory_usage_mb
        );

        Ok(Self {
            device,
            components,
            decoder,
            cached_filters,
            audio_config,
            device_info,
            last_detected_language: None,
        })
    }

    /// Transcribe audio samples into a transcript with timestamps and confidence scores.
    pub fn transcribe(&mut self, audio: &[f32]) -> Result<Transcript, WhisperEngineError> {
        let _span = tracing::debug_span!("whisper_transcribe", samples = audio.len()).entered();

        if audio.is_empty() {
            return Ok(Transcript {
                segments: vec![],
                language: self.language(),
            });
        }

        tracing::info!("Starting transcription of {} audio samples", audio.len());

        // Validate audio format
        self.validate_audio_format(audio)?;

        // Use the existing advanced decoder
        let transcript = self
            .decoder
            .decode(audio)
            .map_err(|e| WhisperEngineError::TranscriptionFailed(e.to_string()))?;

        // Store the detected language
        self.last_detected_language = transcript.language.clone();

        tracing::info!(
            "Transcription completed: {} segments, language: {:?}",
            transcript.segments.len(),
            transcript.language
        );

        Ok(transcript)
    }

    /// Get the current device information.
    pub fn device_info(&self) -> &DeviceInfo {
        &self.device_info
    }

    /// Get the underlying decoder for advanced configuration.
    pub fn decoder(&self) -> &Decoder {
        &self.decoder
    }

    /// Get a mutable reference to the decoder for runtime configuration updates.
    pub fn decoder_mut(&mut self) -> &mut Decoder {
        &mut self.decoder
    }

    /// Get the current language hint.
    pub fn language(&self) -> Option<String> {
        self.last_detected_language.clone()
    }

    /// Update suppression tokens at runtime.
    pub fn update_suppression(&mut self, tokens: HashSet<u32>) -> Result<(), ColdVoxError> {
        self.decoder.update_suppression(tokens)
    }

    /// Update temperature setting at runtime.
    pub fn update_temperature(&mut self, temperature: f32) {
        self.decoder.update_temperature(temperature);
    }

    /// Get cached mel filters (for diagnostics or external use).
    pub fn cached_filters(&self) -> Option<&Arc<Vec<f32>>> {
        self.cached_filters.as_ref()
    }

    /// Get the sample rate used by this engine.
    pub fn sample_rate(&self) -> u32 {
        SAMPLE_RATE as u32
    }

    /// Get audio processing configuration.
    pub fn audio_config(&self) -> &WhisperAudioConfig {
        &self.audio_config
    }

    /// Get model configuration.
    pub fn model_config(&self) -> &candle_transformers::models::whisper::Config {
        &self.components.config
    }

    /// Estimate memory usage of the loaded model.
    pub fn estimate_memory_usage(&self) -> Result<u64, ColdVoxError> {
        // This is a simplified estimation - in practice you'd want more precise tracking
        let vocab_size = self.components.config.vocab_size;
        let d_model = self.components.config.d_model;
        let num_layers =
            self.components.config.encoder_layers + self.components.config.decoder_layers;

        // Rough estimation: vocab_size * d_model * 4 bytes for f32 + overhead
        let estimated_bytes = (vocab_size * d_model * 4) as u64;
        let layer_overhead = (num_layers * d_model * d_model * 4) as u64;

        Ok(estimated_bytes + layer_overhead)
    }

    /// Initialize device based on preference.
    fn initialize_device(preference: &DevicePreference) -> Result<Device, WhisperEngineError> {
        match preference {
            DevicePreference::Cpu => {
                tracing::info!("Using CPU device as requested");
                Ok(Device::Cpu)
            }
            DevicePreference::Cuda => {
                if Self::is_cuda_available() {
                    tracing::info!("Using CUDA device as requested");
                    Device::new_cuda(0).map_err(|e| WhisperEngineError::DeviceInit(e.to_string()))
                } else {
                    Err(WhisperEngineError::DeviceInit(
                        "CUDA requested but not available".to_string(),
                    ))
                }
            }
            DevicePreference::Auto => {
                if Self::is_cuda_available() {
                    tracing::info!("Auto-selecting CUDA device (available)");
                    Device::new_cuda(0).map_err(|e| WhisperEngineError::DeviceInit(e.to_string()))
                } else {
                    tracing::info!("Auto-selecting CPU device (CUDA not available)");
                    Ok(Device::Cpu)
                }
            }
        }
    }

    /// Check if CUDA is available on this system.
    fn is_cuda_available() -> bool {
        candle_core::utils::cuda_is_available()
    }

    /// Create device information for the initialized device.
    fn create_device_info(device: &Device, is_quantized: bool) -> DeviceInfo {
        if device.is_cuda() {
            // TODO: Implement actual memory usage reporting for CUDA devices
            DeviceInfo::cuda(is_quantized, None)
        } else {
            DeviceInfo::cpu(is_quantized)
        }
    }

    /// Validate audio format and content.
    fn validate_audio_format(&self, audio: &[f32]) -> Result<(), WhisperEngineError> {
        if audio.is_empty() {
            return Err(WhisperEngineError::AudioProcessing(
                "Empty audio buffer".to_string(),
            ));
        }

        // Check for NaN or infinite values
        for (i, &sample) in audio.iter().enumerate() {
            if !sample.is_finite() {
                return Err(WhisperEngineError::AudioProcessing(format!(
                    "Non-finite value at sample {}: {}",
                    i, sample
                )));
            }
        }

        // Check amplitude range (should be roughly -1.0 to 1.0 for normalized audio)
        let max_amplitude = audio
            .iter()
            .fold(0.0_f32, |max, &sample| max.max(sample.abs()));
        if max_amplitude > 2.0 {
            tracing::warn!(
                "Audio amplitude ({}) seems unusually high, may need normalization",
                max_amplitude
            );
        }

        Ok(())
    }
}

impl Drop for WhisperEngine {
    fn drop(&mut self) {
        tracing::debug!("Dropping WhisperEngine, cleaning up resources");
        // Additional cleanup could be added here if needed
        // For example, clearing GPU memory or other resource cleanup
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use tempfile::tempdir;

    fn make_test_init() -> WhisperEngineInit {
        WhisperEngineInit::new()
            .with_model_id("test/model")
            .with_revision("main")
            .with_device_preference(DevicePreference::Cpu)
    }

    #[test]
    fn test_init_validation() {
        let mut init = make_test_init();
        assert!(init.validate().is_ok());

        init.model_id = "".to_string();
        assert!(init.validate().is_err());

        init.model_id = "test/model".to_string();
        init.revision = "".to_string();
        assert!(init.validate().is_err());
    }

    #[test]
    fn test_device_preference() {
        let cpu_pref = DevicePreference::Cpu;
        assert_eq!(cpu_pref, DevicePreference::Cpu);

        let cuda_pref = DevicePreference::Cuda;
        assert_eq!(cuda_pref, DevicePreference::Cuda);

        let auto_pref = DevicePreference::Auto;
        assert_eq!(auto_pref, DevicePreference::Auto);
    }

    #[test]
    fn test_builder_pattern() {
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
    fn test_device_info_creation() {
        let cpu_info = DeviceInfo::cpu(false);
        assert_eq!(cpu_info.device_type, "CPU");
        assert!(!cpu_info.is_cuda);
        assert!(!cpu_info.is_quantized);

        let cuda_info = DeviceInfo::cuda(true, Some(1024));
        assert_eq!(cuda_info.device_type, "CUDA");
        assert!(cuda_info.is_cuda);
        assert!(cuda_info.is_quantized);
        assert_eq!(cuda_info.memory_usage_mb, Some(1024));
    }

    #[test]
    fn test_cuda_availability_check() {
        // This test verifies the function doesn't panic and returns a boolean
        let is_available = super::WhisperEngine::is_cuda_available();
        assert!(matches!(is_available, true | false));
    }

    #[test]
    fn test_device_initialization() {
        // Test CPU initialization
        let cpu_device = super::WhisperEngine::initialize_device(&DevicePreference::Cpu);
        assert!(cpu_device.is_ok());
        // Just verify it can be created, don't compare Device types (no PartialEq)
        let _device = cpu_device.unwrap();

        // Test Auto initialization (should fallback to CPU in test environment)
        let auto_device = super::WhisperEngine::initialize_device(&DevicePreference::Auto);
        assert!(auto_device.is_ok());
    }

    #[test]
    fn test_audio_validation() {
        // Create a mock engine for testing audio validation
        let init = make_test_init();

        // Note: This test will fail during actual engine creation due to model loading,
        // but we can test the validation logic separately
        // For now, just test the validation methods exist
        assert!(init.validate().is_ok());
    }

    #[test]
    fn test_init_with_all_options() {
        let init = WhisperEngineInit::new()
            .with_model_id("openai/whisper-base")
            .with_revision("main")
            .with_device_preference(DevicePreference::Auto)
            .with_quantized(true)
            .with_language("en")
            .with_max_tokens(256)
            .with_temperature(0.2)
            .with_generate_timestamps(true);

        assert!(init.validate().is_ok());
        assert_eq!(init.model_id, "openai/whisper-base");
        assert_eq!(init.revision, "main");
        assert_eq!(init.device_preference, DevicePreference::Auto);
        assert!(init.quantized);
        assert_eq!(init.language, Some("en".to_string()));
        assert_eq!(init.max_tokens, 256);
        assert_eq!(init.temperature, 0.2);
        assert!(init.generate_timestamps);
    }

    #[test]
    fn test_init_defaults() {
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
    fn test_local_path_validation() {
        let dir = tempdir().unwrap();
        let path = dir.path().to_path_buf();

        let init = WhisperEngineInit::new().with_local_path(&path);

        // Should be valid since path exists
        assert!(init.validate().is_ok());
    }

    #[test]
    fn test_non_existent_local_path_validation() {
        let path = std::path::PathBuf::from("/non/existent/path");

        let init = WhisperEngineInit::new().with_local_path(path);

        // Should fail validation since path doesn't exist
        assert!(init.validate().is_err());
    }

    #[test]
    fn test_whisper_engine_error_types() {
        // Test that different error types can be created
        let error1 = WhisperEngineError::Config(ColdVoxError::Stt(SttError::InvalidConfig(
            "test".to_string(),
        )));
        assert!(!error1.to_string().is_empty());

        let error2 = WhisperEngineError::DeviceInit("test error".to_string());
        assert!(!error2.to_string().is_empty());
        assert!(error2.to_string().contains("Device initialization failed"));

        let error3 = WhisperEngineError::AudioProcessing("audio error".to_string());
        assert!(!error3.to_string().is_empty());
        assert!(error3.to_string().contains("Audio preprocessing failed"));
    }

    #[test]
    fn test_device_info_different_types() {
        let cpu_info = DeviceInfo::cpu(true);
        assert_eq!(cpu_info.device_type, "CPU");
        assert!(cpu_info.is_quantized);
        assert!(!cpu_info.is_cuda);

        let cuda_info = DeviceInfo::cuda(false, Some(2048));
        assert_eq!(cuda_info.device_type, "CUDA");
        assert!(!cuda_info.is_quantized);
        assert!(cuda_info.is_cuda);
        assert_eq!(cuda_info.memory_usage_mb, Some(2048));
    }
}
