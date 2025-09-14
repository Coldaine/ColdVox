//! Picovoice Leopard - Commercial ultra-lightweight STT
//!
//! Leopard is Picovoice's on-device speech-to-text engine optimized for
//! resource-constrained environments with excellent accuracy.

use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::RwLock;

use crate::plugin::*;
use crate::plugin_types::*;
use crate::types::{TranscriptionEvent, TranscriptionConfig};

/// Leopard configuration
#[derive(Debug, Clone)]
pub struct LeopardConfig {
    /// Picovoice access key (required for commercial use)
    pub access_key: String,
    /// Path to Leopard model file (.pv)
    pub model_path: PathBuf,
    /// Enable automatic punctuation
    pub enable_punctuation: bool,
    /// Enable diarization (speaker identification)
    pub enable_diarization: bool,
}

impl Default for LeopardConfig {
    fn default() -> Self {
        Self {
            access_key: std::env::var("PICOVOICE_ACCESS_KEY").unwrap_or_default(),
            model_path: PathBuf::from("models/leopard/leopard-en.pv"),
            enable_punctuation: true,
            enable_diarization: false,
        }
    }
}

/// Picovoice Leopard STT Plugin
/// 
/// Commercial ultra-lightweight STT with:
/// - ~30MB model size
/// - Excellent accuracy for English
/// - Very low latency
/// - Minimal resource usage
#[derive(Debug)]
pub struct LeopardPlugin {
    config: LeopardConfig,
    state: Arc<RwLock<PluginState>>,
    metrics: Arc<RwLock<PluginMetrics>>,
    // Future: Add Leopard SDK
    // leopard: Option<Leopard>,
}

impl LeopardPlugin {
    pub fn new() -> Self {
        Self::with_config(LeopardConfig::default())
    }
    
    pub fn with_config(config: LeopardConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(PluginState::Uninitialized)),
            metrics: Arc::new(RwLock::new(PluginMetrics::default())),
        }
    }
    
    pub fn enhanced_info() -> EnhancedPluginInfo {
        EnhancedPluginInfo {
            id: "leopard".to_string(),
            name: "Picovoice Leopard".to_string(),
            description: "Commercial ultra-lightweight on-device STT".to_string(),
            version: "2.0.0".to_string(),
            author: "Picovoice".to_string(),
            license: "Commercial".to_string(),
            homepage: Some("https://picovoice.ai/platform/leopard/".to_string()),
            
            accuracy_level: AccuracyLevel::High,
            latency_profile: LatencyProfile {
                avg_ms: 40,
                p95_ms: 80,
                p99_ms: 150,
                rtf: 0.1, // Very fast
            },
            resource_profile: ResourceProfile {
                peak_memory_mb: 80,
                avg_cpu_percent: 8.0,
                uses_gpu: false,
                disk_space_mb: 30,
            },
            model_size: ModelSize::Tiny,
            
            languages: vec![
                LanguageSupport {
                    code: "en".to_string(),
                    name: "English".to_string(),
                    quality: LanguageQuality::Premium,
                    variants: vec!["en-US".to_string(), "en-GB".to_string()],
                },
            ],
            
            requires_internet: false,
            requires_gpu: false,
            requires_license_key: true,
            
            is_beta: false,
            is_deprecated: false,
            source: PluginSource::BuiltIn,
            
            metrics: None,
        }
    }
}

#[async_trait]
impl SttPlugin for LeopardPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: "leopard".to_string(),
            name: "Picovoice Leopard".to_string(),
            description: "Commercial ultra-lightweight STT (requires license)".to_string(),
            requires_network: false,
            is_local: true,
            is_available: false,
            supported_languages: vec!["en".to_string()],
            memory_usage_mb: Some(30),
        }
    }
    
    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            streaming: false, // Leopard is file-based
            batch: true,
            word_timestamps: true,
            confidence_scores: true,
            speaker_diarization: self.config.enable_diarization,
            auto_punctuation: self.config.enable_punctuation,
            custom_vocabulary: false,
        }
    }
    
    async fn is_available(&self) -> Result<bool, SttPluginError> {
        // Check for access key
        if self.config.access_key.is_empty() {
            return Ok(false);
        }
        Ok(false) // Not yet implemented
    }
    
    async fn initialize(&mut self, _config: TranscriptionConfig) -> Result<(), SttPluginError> {
        if self.config.access_key.is_empty() {
            return Err(SttPluginError::ConfigurationError(
                "PICOVOICE_ACCESS_KEY required for Leopard".to_string()
            ));
        }
        
        Err(SttPluginError::NotAvailable {
            reason: "Leopard SDK integration not yet implemented".to_string(),
        })
    }
    
    async fn process_audio(&mut self, _samples: &[i16]) -> Result<Option<TranscriptionEvent>, SttPluginError> {
        Err(SttPluginError::NotAvailable {
            reason: "Leopard plugin not yet implemented".to_string(),
        })
    }
    
    async fn finalize(&mut self) -> Result<Option<TranscriptionEvent>, SttPluginError> {
        Ok(None)
    }
    
    async fn reset(&mut self) -> Result<(), SttPluginError> {
        Ok(())
    }
}

pub struct LeopardPluginFactory {
    config: LeopardConfig,
}

impl LeopardPluginFactory {
    pub fn new() -> Self {
        Self {
            config: LeopardConfig::default(),
        }
    }
}

impl SttPluginFactory for LeopardPluginFactory {
    fn create(&self) -> Result<Box<dyn SttPlugin>, SttPluginError> {
        Ok(Box::new(LeopardPlugin::with_config(self.config.clone())))
    }
    
    fn plugin_info(&self) -> PluginInfo {
        LeopardPlugin::new().info()
    }
    
    fn check_requirements(&self) -> Result<(), SttPluginError> {
        if self.config.access_key.is_empty() {
            return Err(SttPluginError::NotAvailable {
                reason: "Picovoice access key required".to_string(),
            });
        }
        
        Err(SttPluginError::NotAvailable {
            reason: "Leopard SDK not yet integrated".to_string(),
        })
    }
}