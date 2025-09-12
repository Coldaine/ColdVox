//! Complete Vosk STT plugin implementation
//! 
//! This plugin wraps the existing VoskTranscriber to provide
//! a full-featured STT engine with the plugin interface.

use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::RwLock;
use tracing::{debug, info, warn};

use crate::plugin::*;
use crate::plugin_types::*;
use crate::types::{TranscriptionEvent, TranscriptionConfig};

// Note: The actual VoskTranscriber is in the coldvox-stt-vosk crate
// This plugin provides the interface, the app level integrates the actual implementation

/// Vosk plugin configuration
#[derive(Debug, Clone)]
pub struct VoskConfig {
    /// Path to Vosk model
    pub model_path: PathBuf,
    /// Sample rate for audio
    pub sample_rate: f32,
    /// Enable GPU acceleration if available
    pub enable_gpu: bool,
    /// Maximum alternatives to generate
    pub max_alternatives: u32,
    /// Include word-level timestamps
    pub include_words: bool,
    /// Enable partial results
    pub partial_results: bool,
}

impl VoskConfig {
    /// Create optimal configuration for the current system
    pub fn optimal_for_system() -> Result<Self, String> {
        let model_path = Self::find_model_path()?;
        
        Ok(Self {
            model_path,
            sample_rate: 16000.0,
            enable_gpu: false, // GPU support requires special build
            max_alternatives: 1,
            include_words: true,
            partial_results: true,
        })
    }
    
    /// Find the Vosk model path
    fn find_model_path() -> Result<PathBuf, String> {
        // Check environment variable first
        if let Ok(path) = std::env::var("VOSK_MODEL_PATH") {
            let path = PathBuf::from(path);
            if path.exists() {
                info!(
                    target: "coldvox::stt",
                    plugin_id = "vosk",
                    event = "model_path_env",
                    model_path = ?path,
                    "Using Vosk model from VOSK_MODEL_PATH"
                );
                return Ok(path);
            }
        }
        
        // Check common locations
        let candidates = vec![
            PathBuf::from("models/vosk-model-small-en-us-0.15"),
            PathBuf::from("models/vosk-model-en-us-0.22"),
            PathBuf::from("/usr/share/vosk-models/small-en-us"),
            PathBuf::from("/usr/local/share/vosk-models/small-en-us"),
            dirs::data_dir().map(|d| d.join("vosk-models/small-en-us")).unwrap_or_default(),
        ];
        
        for path in candidates {
            if path.exists() {
                info!(
                    target: "coldvox::stt",
                    plugin_id = "vosk",
                    event = "model_path_found",
                    model_path = ?path,
                    "Found Vosk model"
                );
                return Ok(path);
            }
        }
        
        Err("Vosk model not found. Set VOSK_MODEL_PATH or download a model.".to_string())
    }
}

/// Complete Vosk STT plugin
/// 
/// This is a stub that indicates Vosk support. The actual VoskTranscriber
/// implementation is in coldvox-stt-vosk and should be integrated at the
/// application level to avoid circular dependencies.
#[derive(Debug)]
pub struct VoskPlugin {
    config: VoskConfig,
    metrics: Arc<RwLock<PluginMetrics>>,
    state: Arc<RwLock<PluginState>>,
    start_time: Option<std::time::Instant>,
    initialized: bool,
}

impl VoskPlugin {
    /// Create a new Vosk plugin
    pub fn new() -> Result<Self, SttPluginError> {
        let config = VoskConfig::optimal_for_system()
            .map_err(|e| SttPluginError::ConfigurationError(e))?;
        
        Ok(Self { config, metrics: Arc::new(RwLock::new(PluginMetrics::default())), state: Arc::new(RwLock::new(PluginState::Uninitialized)), start_time: None, initialized: false })
    }
    
    /// Create with specific configuration
    pub fn with_config(config: VoskConfig) -> Self {
        Self { config, metrics: Arc::new(RwLock::new(PluginMetrics::default())), state: Arc::new(RwLock::new(PluginState::Uninitialized)), start_time: None, initialized: false }
    }
    
    /// Get enhanced plugin information
    pub fn enhanced_info() -> EnhancedPluginInfo {
        EnhancedPluginInfo {
            id: "vosk".to_string(),
            name: "Vosk Speech Recognition".to_string(),
            description: "Offline speech recognition using Kaldi-based Vosk models".to_string(),
            version: "0.3.45".to_string(),
            author: "Alpha Cephei".to_string(),
            license: "Apache-2.0".to_string(),
            homepage: Some("https://alphacephei.com/vosk/".to_string()),
            
            accuracy_level: AccuracyLevel::High,
            latency_profile: LatencyProfile {
                avg_ms: 150,
                p95_ms: 300,
                p99_ms: 500,
                rtf: 0.3,
            },
            resource_profile: ResourceProfile {
                peak_memory_mb: 600,
                avg_cpu_percent: 25.0,
                uses_gpu: false,
                disk_space_mb: 500,
            },
            model_size: ModelSize::Large,
            
            languages: vec![
                LanguageSupport {
                    code: "en".to_string(),
                    name: "English".to_string(),
                    quality: LanguageQuality::Premium,
                    variants: vec!["en-US".to_string(), "en-GB".to_string()],
                },
                LanguageSupport {
                    code: "ru".to_string(),
                    name: "Russian".to_string(),
                    quality: LanguageQuality::Stable,
                    variants: vec![],
                },
                LanguageSupport {
                    code: "de".to_string(),
                    name: "German".to_string(),
                    quality: LanguageQuality::Stable,
                    variants: vec![],
                },
                // Add more languages as needed
            ],
            
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

#[async_trait]
impl SttPlugin for VoskPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: "vosk".to_string(),
            name: "Vosk Speech Recognition".to_string(),
            description: "Offline speech recognition using Vosk models".to_string(),
            requires_network: false,
            is_local: true,
            is_available: check_vosk_available(),
            supported_languages: vec![
                "en".to_string(), "ru".to_string(), "de".to_string(), 
                "es".to_string(), "fr".to_string(), "it".to_string(),
                "nl".to_string(), "pt".to_string(), "tr".to_string(),
                "cn".to_string(), "ja".to_string(), "hi".to_string(),
            ],
            memory_usage_mb: Some(500),
        }
    }
    
    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            streaming: true,
            batch: true,
            word_timestamps: true,
            confidence_scores: true,
            speaker_diarization: false,
            auto_punctuation: false,
            custom_vocabulary: true,
        }
    }
    
    async fn is_available(&self) -> Result<bool, SttPluginError> {
        #[cfg(not(feature = "vosk"))]
        {
            return Ok(false);
        }
        
        #[cfg(feature = "vosk")]
        {
            if !check_vosk_available() {
                return Ok(false);
            }
            
            // Check if model exists
            if !self.config.model_path.exists() {
                debug!("Vosk model not found at {:?}", self.config.model_path);
                return Ok(false);
            }
            
            Ok(true)
        }
    }
    
    async fn initialize(&mut self, config: TranscriptionConfig) -> Result<(), SttPluginError> {
        if !check_vosk_available() {
            return Err(SttPluginError::NotAvailable { reason: "Vosk library not found on system".to_string() });
        }

        *self.state.write() = PluginState::Loading;
        // Stub mode: actual transcriber lives in coldvox-stt-vosk crate to avoid circular dependency.
        warn!("Vosk plugin operating in stub mode (no internal transcriber)");
        self.initialized = true;
        *self.state.write() = PluginState::Ready;
        Ok(())
    }
    
    async fn process_audio(&mut self, samples: &[i16]) -> Result<Option<TranscriptionEvent>, SttPluginError> {
        // Stub always returns NotAvailable for processing (no internal engine)
        Err(SttPluginError::NotAvailable { reason: "Vosk processing not available in stub plugin; use coldvox-stt-vosk crate".to_string() })
    }
    
    async fn finalize(&mut self) -> Result<Option<TranscriptionEvent>, SttPluginError> {
        Ok(None)
    }
    
    async fn reset(&mut self) -> Result<(), SttPluginError> {
        self.start_time = None;
        if self.initialized {
            *self.state.write() = PluginState::Ready;
        }
        Ok(())
    }
    
    async fn load_model(&mut self, model_path: Option<&Path>) -> Result<(), SttPluginError> {
        if let Some(path) = model_path {
            if !path.exists() {
                return Err(SttPluginError::ModelLoadFailed(
                    format!("Model not found at {:?}", path)
                ));
            }
            self.config.model_path = path.to_path_buf();
        }
        
        info!(
            target: "coldvox::stt",
            plugin_id = "vosk",
            event = "model_path_updated",
            model_path = ?model_path,
            "Vosk model path updated (stub mode)"
        );
        Ok(())
    }
    
    async fn unload(&mut self) -> Result<(), SttPluginError> {
        // Check if already unloaded
        if !self.initialized {
            return Err(SttPluginError::AlreadyUnloaded(
                "Vosk plugin is already unloaded".to_string()
            ));
        }
        
        // Reset plugin state
        self.initialized = false;
        self.start_time = None;
        *self.state.write() = PluginState::Uninitialized;
        
        // Reset metrics
        *self.metrics.write() = PluginMetrics::default();
        
        info!(
            target: "coldvox::stt",
            plugin_id = "vosk",
            event = "plugin_unloaded",
            "Vosk plugin unloaded successfully"
        );
        Ok(())
    }
}

fn check_vosk_available() -> bool {
    #[cfg(not(feature = "vosk"))]
    {
        return false;
    }
    
    #[cfg(feature = "vosk")]
    {
        // Check if libvosk is available on the system
        #[cfg(target_os = "linux")]
        {
            std::path::Path::new("/usr/lib/libvosk.so").exists() ||
            std::path::Path::new("/usr/local/lib/libvosk.so").exists() ||
            std::path::Path::new("/usr/lib/x86_64-linux-gnu/libvosk.so").exists()
        }
        
        #[cfg(target_os = "macos")]
        {
            std::path::Path::new("/usr/local/lib/libvosk.dylib").exists() ||
            std::path::Path::new("/opt/homebrew/lib/libvosk.dylib").exists()
        }
        
        #[cfg(target_os = "windows")]
        {
            std::path::Path::new("C:\\Program Files\\Vosk\\vosk.dll").exists() ||
            std::path::Path::new("vosk.dll").exists()
        }
        
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            false
        }
    }
}

/// Factory for creating VoskPlugin instances
pub struct VoskPluginFactory {
    config: Option<VoskConfig>,
}

impl VoskPluginFactory {
    pub fn new() -> Self {
        Self {
            config: VoskConfig::optimal_for_system().ok(),
        }
    }
    
    pub fn with_config(config: VoskConfig) -> Self {
        Self {
            config: Some(config),
        }
    }
}

impl SttPluginFactory for VoskPluginFactory {
    fn create(&self) -> Result<Box<dyn SttPlugin>, SttPluginError> {
        if let Some(ref config) = self.config {
            Ok(Box::new(VoskPlugin::with_config(config.clone())))
        } else {
            Ok(Box::new(VoskPlugin::new()?))
        }
    }
    
    fn plugin_info(&self) -> PluginInfo {
        VoskPlugin::new()
            .map(|p| p.info())
            .unwrap_or_else(|_| PluginInfo {
                id: "vosk".to_string(),
                name: "Vosk Speech Recognition".to_string(),
                description: "Offline speech recognition (unavailable)".to_string(),
                requires_network: false,
                is_local: true,
                is_available: false,
                supported_languages: vec![],
                memory_usage_mb: Some(500),
            })
    }
    
    fn check_requirements(&self) -> Result<(), SttPluginError> {
        if !check_vosk_available() {
            return Err(SttPluginError::NotAvailable {
                reason: "libvosk not found on system".to_string(),
            });
        }
        
        if let Some(ref config) = self.config {
            if !config.model_path.exists() {
                return Err(SttPluginError::NotAvailable {
                    reason: format!("Model not found at {:?}", config.model_path),
                });
            }
        }
        
        Ok(())
    }
}