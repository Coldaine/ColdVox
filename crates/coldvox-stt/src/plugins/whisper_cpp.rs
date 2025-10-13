//! Whisper.cpp - Lightweight C++ implementation of OpenAI Whisper
//!
//! This plugin wraps whisper.cpp, a lightweight C++ port of OpenAI's Whisper
//! that uses ggml quantization for efficient inference on CPU.

use async_trait::async_trait;
use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

use crate::plugin::*;
use crate::plugin_types::*;
use crate::types::{TranscriptionConfig, TranscriptionEvent};

/// Whisper model types (ggml quantized)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhisperModelType {
    /// Tiny model - 39MB, fastest, lower accuracy
    Tiny,
    /// Tiny English-only - 39MB, optimized for English
    TinyEn,
    /// Base model - 74MB, balanced
    Base,
    /// Base English-only - 74MB
    BaseEn,
    /// Small model - 244MB, good accuracy
    Small,
    /// Small English-only - 244MB
    SmallEn,
    /// Medium model - 769MB, better accuracy
    Medium,
    /// Medium English-only - 769MB
    MediumEn,
    /// Large model - 1550MB, best accuracy
    Large,
}

impl WhisperModelType {
    pub fn model_size_mb(&self) -> u32 {
        match self {
            Self::Tiny | Self::TinyEn => 39,
            Self::Base | Self::BaseEn => 74,
            Self::Small | Self::SmallEn => 244,
            Self::Medium | Self::MediumEn => 769,
            Self::Large => 1550,
        }
    }

    pub fn expected_accuracy(&self) -> AccuracyLevel {
        match self {
            Self::Tiny | Self::TinyEn => AccuracyLevel::Low,
            Self::Base | Self::BaseEn => AccuracyLevel::Medium,
            Self::Small | Self::SmallEn => AccuracyLevel::High,
            Self::Medium | Self::MediumEn => AccuracyLevel::High,
            Self::Large => AccuracyLevel::VeryHigh,
        }
    }

    pub fn is_english_only(&self) -> bool {
        matches!(
            self,
            Self::TinyEn | Self::BaseEn | Self::SmallEn | Self::MediumEn
        )
    }

    pub fn filename(&self) -> &str {
        match self {
            Self::Tiny => "ggml-tiny.bin",
            Self::TinyEn => "ggml-tiny.en.bin",
            Self::Base => "ggml-base.bin",
            Self::BaseEn => "ggml-base.en.bin",
            Self::Small => "ggml-small.bin",
            Self::SmallEn => "ggml-small.en.bin",
            Self::Medium => "ggml-medium.bin",
            Self::MediumEn => "ggml-medium.en.bin",
            Self::Large => "ggml-large.bin",
        }
    }
}

/// Whisper.cpp configuration
#[derive(Debug, Clone)]
pub struct WhisperCppConfig {
    /// Model type to use
    pub model_type: WhisperModelType,
    /// Path to model file
    pub model_path: Option<PathBuf>,
    /// Target language (ISO 639-1)
    pub language: String,
    /// Enable word-level timestamps
    pub enable_timestamps: bool,
    /// Number of threads for inference
    pub num_threads: u32,
    /// Use GPU if available (requires CUDA/Metal build)
    pub use_gpu: bool,
    /// Beam size for decoding
    pub beam_size: u32,
    /// Temperature for sampling
    pub temperature: f32,
}

impl Default for WhisperCppConfig {
    fn default() -> Self {
        Self {
            model_type: WhisperModelType::TinyEn,
            model_path: None,
            language: "en".to_string(),
            enable_timestamps: true,
            num_threads: 4,
            use_gpu: false,
            beam_size: 5,
            temperature: 0.0,
        }
    }
}

/// Whisper.cpp STT Plugin
///
/// This is a stub implementation for whisper.cpp integration.
/// Once implemented, it will provide:
/// - Quantized model support (ggml format)
/// - CPU-optimized inference
/// - Multiple model sizes for different accuracy/speed tradeoffs
#[derive(Debug)]
pub struct WhisperCppPlugin {
    config: WhisperCppConfig,
    state: Arc<RwLock<PluginState>>,
    // Future: Add actual whisper.cpp context
    // context: Option<*mut WhisperContext>,
}

impl WhisperCppPlugin {
    pub fn new() -> Self {
        Self::with_config(WhisperCppConfig::default())
    }

    pub fn with_config(config: WhisperCppConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(PluginState::Uninitialized)),
        }
    }

    pub fn enhanced_info() -> EnhancedPluginInfo {
        let config = WhisperCppConfig::default();

        EnhancedPluginInfo {
            id: "whisper-cpp".to_string(),
            name: "Whisper.cpp".to_string(),
            description: "Lightweight C++ implementation of OpenAI Whisper with quantized models"
                .to_string(),
            version: "1.5.0".to_string(),
            author: "ggerganov".to_string(),
            license: "MIT".to_string(),
            homepage: Some("https://github.com/ggerganov/whisper.cpp".to_string()),

            accuracy_level: config.model_type.expected_accuracy(),
            latency_profile: LatencyProfile {
                avg_ms: 100,
                p95_ms: 200,
                p99_ms: 400,
                rtf: 0.25,
            },
            resource_profile: ResourceProfile {
                peak_memory_mb: config.model_type.model_size_mb() + 100,
                avg_cpu_percent: 30.0,
                uses_gpu: config.use_gpu,
                disk_space_mb: config.model_type.model_size_mb(),
            },
            model_size: ModelSize::from_mb(config.model_type.model_size_mb()),

            languages: if config.model_type.is_english_only() {
                vec![LanguageSupport {
                    code: "en".to_string(),
                    name: "English".to_string(),
                    quality: LanguageQuality::Premium,
                    variants: vec!["en-US".to_string(), "en-GB".to_string()],
                }]
            } else {
                // Whisper supports 99+ languages
                vec![LanguageSupport {
                    code: "multi".to_string(),
                    name: "Multilingual".to_string(),
                    quality: LanguageQuality::Stable,
                    variants: vec![],
                }]
            },

            requires_internet: false,
            requires_gpu: false,
            requires_license_key: false,

            is_beta: false,
            is_deprecated: false,
            source: PluginSource::BuiltIn,

            metrics: None,
        }
    }
}

impl Default for WhisperCppPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SttPlugin for WhisperCppPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: "whisper-cpp".to_string(),
            name: "Whisper.cpp".to_string(),
            description: "Lightweight Whisper implementation (not yet available)".to_string(),
            requires_network: false,
            is_local: true,
            is_available: false, // Not yet implemented
            supported_languages: if self.config.model_type.is_english_only() {
                vec!["en".to_string()]
            } else {
                vec!["multi".to_string()]
            },
            memory_usage_mb: Some(self.config.model_type.model_size_mb()),
        }
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            streaming: true,
            batch: true,
            word_timestamps: self.config.enable_timestamps,
            confidence_scores: true,
            speaker_diarization: false,
            auto_punctuation: true, // Whisper includes punctuation
            custom_vocabulary: false,
        }
    }

    async fn is_available(&self) -> Result<bool, SttPluginError> {
        // Check if whisper.cpp library is available
        // In the future, check for:
        // 1. whisper.cpp shared library
        // 2. Model file existence
        // 3. CPU features (AVX, etc.)

        Ok(false) // Not yet implemented
    }

    async fn initialize(&mut self, _config: TranscriptionConfig) -> Result<(), SttPluginError> {
        info!("Whisper.cpp plugin is a stub - not yet implemented");

        // Future implementation:
        // 1. Find or download model
        // 2. Initialize whisper context
        // 3. Configure parameters
        // 4. Warm up with test audio

        Err(SttPluginError::NotAvailable {
            reason: "Whisper.cpp integration not yet implemented".to_string(),
        })
    }

    async fn process_audio(
        &mut self,
        _samples: &[i16],
    ) -> Result<Option<TranscriptionEvent>, SttPluginError> {
        Err(SttPluginError::NotAvailable {
            reason: "Whisper.cpp plugin not yet implemented".to_string(),
        })
    }

    async fn finalize(&mut self) -> Result<Option<TranscriptionEvent>, SttPluginError> {
        Ok(None)
    }

    async fn reset(&mut self) -> Result<(), SttPluginError> {
        *self.state.write() = PluginState::Ready;
        Ok(())
    }
}

/// Factory for creating Whisper.cpp plugin instances
pub struct WhisperCppPluginFactory {
    config: WhisperCppConfig,
}

impl WhisperCppPluginFactory {
    pub fn new() -> Self {
        Self {
            config: WhisperCppConfig::default(),
        }
    }

    pub fn with_config(config: WhisperCppConfig) -> Self {
        Self { config }
    }

    pub fn with_model(model_type: WhisperModelType) -> Self {
        let config = WhisperCppConfig {
            model_type,
            ..Default::default()
        };
        Self { config }
    }
}

impl Default for WhisperCppPluginFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl SttPluginFactory for WhisperCppPluginFactory {
    fn create(&self) -> Result<Box<dyn SttPlugin>, SttPluginError> {
        Ok(Box::new(WhisperCppPlugin::with_config(self.config.clone())))
    }

    fn plugin_info(&self) -> PluginInfo {
        WhisperCppPlugin::new().info()
    }

    fn check_requirements(&self) -> Result<(), SttPluginError> {
        // Check for whisper.cpp library
        // Check for model files
        // Check CPU features

        Err(SttPluginError::NotAvailable {
            reason: "Whisper.cpp not yet integrated".to_string(),
        })
    }
}

// Future implementation notes:
//
// Integration with whisper.cpp will require:
//
// 1. FFI Bindings:
//    - Create Rust bindings for whisper.cpp C API
//    - Handle memory management safely
//    - Implement streaming interface
//
// 2. Model Management:
//    - Download models from Hugging Face
//    - Convert models to ggml format if needed
//    - Cache models efficiently
//
// 3. Performance Optimizations:
//    - Use CPU SIMD instructions (AVX, NEON)
//    - Implement batch processing
//    - Add model quantization options
//
// 4. Advanced Features:
//    - Language detection
//    - Translation mode
//    - Diarization (future whisper.cpp feature)
//
// Example usage:
// ```rust
// let plugin = WhisperCppPlugin::with_config(WhisperCppConfig {
//     model_type: WhisperModelType::Small,
//     language: "en".to_string(),
//     use_gpu: true,
//     ..Default::default()
// });
// ```
