//! STT Plugin Architecture
//!
//! This module defines the plugin interface for Speech-to-Text engines.
//! Any STT backend (Vosk, Whisper, Cloud APIs, etc.) implements these traits.

use async_trait::async_trait;
use std::fmt::Debug;
use std::path::Path;
use thiserror::Error;

use crate::types::{TranscriptionConfig, TranscriptionEvent};

/// Errors that can occur in STT plugins
#[derive(Debug, Error)]
pub enum SttPluginError {
    #[error("Plugin not available: {reason}")]
    NotAvailable { reason: String },

    #[error("Initialization failed: {0}")]
    InitializationFailed(String),

    #[error("Model loading failed: {0}")]
    ModelLoadFailed(String),

    #[error("Transcription failed: {0}")]
    TranscriptionFailed(String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error("Backend error: {0}")]
    BackendError(Box<dyn std::error::Error + Send + Sync>),

    #[error("Processing error: {0}")]
    ProcessingError(String),

    /// Transient errors that may be retried (e.g., audio buffer empty, temporary network issues)
    #[error("Transient error: {0}")]
    Transient(String),

    /// Fatal errors that should trigger failover (e.g., model corruption, permanent backend failure)
    #[error("Fatal error: {0}")]
    Fatal(String),

    /// Unload operation failed (e.g., resource cleanup error)
    #[error("Unload failed: {0}")]
    UnloadFailed(String),

    /// Plugin already unloaded or not loaded
    #[error("Already unloaded: {0}")]
    AlreadyUnloaded(String),

    /// Extensible error wrapper for third-party plugins
    #[error("Other error: {0}")]
    Other(Box<dyn std::error::Error + Send + Sync>),
}

/// Metadata about an STT plugin
#[derive(Debug, Clone)]
pub struct PluginInfo {
    /// Unique identifier for the plugin (e.g., "vosk", "whisper", "gcloud")
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// Brief description of the plugin
    pub description: String,

    /// Whether this plugin requires network access
    pub requires_network: bool,

    /// Whether this plugin processes audio locally
    pub is_local: bool,

    /// Whether this plugin is currently available on the system
    pub is_available: bool,

    /// Supported languages (ISO 639-1 codes)
    pub supported_languages: Vec<String>,

    /// Estimated memory usage in MB
    pub memory_usage_mb: Option<u32>,
}

/// Capabilities that an STT plugin might support
#[derive(Debug, Clone, Default)]
pub struct PluginCapabilities {
    /// Supports real-time streaming transcription
    pub streaming: bool,

    /// Supports batch transcription of complete audio
    pub batch: bool,

    /// Can provide word-level timestamps
    pub word_timestamps: bool,

    /// Can provide confidence scores
    pub confidence_scores: bool,

    /// Supports speaker diarization
    pub speaker_diarization: bool,

    /// Can punctuate text automatically
    pub auto_punctuation: bool,

    /// Supports custom vocabulary
    pub custom_vocabulary: bool,
}

/// The main trait that all STT plugins must implement
#[async_trait]
pub trait SttPlugin: Send + Sync + Debug {
    /// Get plugin metadata
    fn info(&self) -> PluginInfo;

    /// Get plugin capabilities
    fn capabilities(&self) -> PluginCapabilities;

    /// Check if the plugin is available and ready to use
    async fn is_available(&self) -> Result<bool, SttPluginError>;

    /// Initialize the plugin with configuration
    async fn initialize(&mut self, config: TranscriptionConfig) -> Result<(), SttPluginError>;

    /// Process a batch of audio samples
    /// Returns None if no transcription is ready yet (for streaming mode)
    async fn process_audio(
        &mut self,
        samples: &[i16],
    ) -> Result<Option<TranscriptionEvent>, SttPluginError>;

    /// Finalize and get any remaining transcription
    async fn finalize(&mut self) -> Result<Option<TranscriptionEvent>, SttPluginError>;

    /// Reset the plugin state for a new session
    async fn reset(&mut self) -> Result<(), SttPluginError>;

    /// Load a model or connect to service
    async fn load_model(&mut self, _model_path: Option<&Path>) -> Result<(), SttPluginError> {
        // Default implementation for plugins that don't need models
        Ok(())
    }

    /// Unload model and free resources
    /// This is called when the plugin is no longer needed or during garbage collection
    ///
    /// # Implementation Guidelines for Plugin Developers:
    ///
    /// ## For Model-Based Plugins (Vosk, Whisper, Coqui, etc.):
    /// - Drop model instances to free GPU/CPU memory
    /// - Close any open file handles or network connections
    /// - Reset internal state to uninitialized
    /// - Clear any cached data or temporary files
    ///
    /// ## For Cloud-Based Plugins:
    /// - Close HTTP connections and connection pools
    /// - Invalidate authentication tokens if appropriate
    /// - Clear any cached responses
    ///
    /// ## For All Plugins:
    /// - Reset metrics and performance counters
    /// - Set state to Uninitialized
    /// - Log the unload operation for debugging
    /// - Handle errors gracefully (don't fail the unload process)
    ///
    /// Default implementation is a no-op for plugins that don't need cleanup
    async fn unload(&mut self) -> Result<(), SttPluginError> {
        Ok(())
    }
}

/// Factory for creating STT plugins
pub trait SttPluginFactory: Send + Sync {
    /// Create a new instance of the plugin
    fn create(&self) -> Result<Box<dyn SttPlugin>, SttPluginError>;

    /// Get plugin info without creating an instance
    fn plugin_info(&self) -> PluginInfo;

    /// Check if the plugin's requirements are met
    fn check_requirements(&self) -> Result<(), SttPluginError>;
}

/// Registry for managing multiple STT plugins
#[derive(Default)]
pub struct SttPluginRegistry {
    factories: Vec<Box<dyn SttPluginFactory>>,
    preferred_order: Vec<String>,
}

impl SttPluginRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new plugin factory
    pub fn register(&mut self, factory: Box<dyn SttPluginFactory>) {
        self.factories.push(factory);
    }

    /// Set the preferred order of plugins to try
    pub fn set_preferred_order(&mut self, order: Vec<String>) {
        self.preferred_order = order;
    }

    /// Get all available plugins
    pub fn available_plugins(&self) -> Vec<PluginInfo> {
        self.factories
            .iter()
            .map(|f| {
                let mut info = f.plugin_info();
                info.is_available = f.check_requirements().is_ok();
                info
            })
            .collect()
    }

    /// Create a plugin by ID
    pub fn create_plugin(&self, id: &str) -> Result<Box<dyn SttPlugin>, SttPluginError> {
        self.factories
            .iter()
            .find(|f| f.plugin_info().id == id)
            .ok_or_else(|| SttPluginError::NotAvailable {
                reason: format!("Plugin '{id}' not found"),
            })?
            .create()
    }

    /// Create the best available plugin based on preferences
    pub fn create_best_available(&self) -> Result<Box<dyn SttPlugin>, SttPluginError> {
        // First try preferred order
        for plugin_id in &self.preferred_order {
            if let Ok(plugin) = self.create_plugin(plugin_id) {
                return Ok(plugin);
            }
        }

        // Then try any available plugin
        for factory in &self.factories {
            if factory.check_requirements().is_ok() {
                if let Ok(plugin) = factory.create() {
                    return Ok(plugin);
                }
            }
        }

        Err(SttPluginError::NotAvailable {
            reason: "No STT plugins available".to_string(),
        })
    }
}

/// Configuration for plugin selection
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginSelectionConfig {
    /// Preferred plugin ID
    pub preferred_plugin: Option<String>,

    /// Fallback plugins to try in order
    pub fallback_plugins: Vec<String>,

    /// Require local processing (no cloud)
    pub require_local: bool,

    /// Maximum memory usage in MB
    pub max_memory_mb: Option<u32>,

    /// Required language support
    pub required_language: Option<String>,

    /// Failover configuration
    pub failover: Option<FailoverConfig>,

    /// Garbage collection policy
    pub gc_policy: Option<GcPolicy>,

    /// Metrics configuration
    pub metrics: Option<MetricsConfig>,

    /// Automatically extract model from a zip archive if not found
    pub auto_extract_model: bool,
}

impl Default for PluginSelectionConfig {
    fn default() -> Self {
        Self {
            preferred_plugin: None,
            fallback_plugins: vec![
                "vosk".to_string(),
                "whisper-local".to_string(),
                "gcloud".to_string(),
            ],
            require_local: false,
            max_memory_mb: None,
            required_language: Some("en".to_string()),
            failover: Some(FailoverConfig::default()),
            gc_policy: Some(GcPolicy::default()),
            metrics: Some(MetricsConfig::default()),
            auto_extract_model: true,
        }
    }
}

/// Configuration for failover behavior between plugins
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FailoverConfig {
    /// Number of consecutive transient errors before switching plugins
    pub failover_threshold: u32,

    /// Cooldown period in seconds before retrying a failed plugin
    pub failover_cooldown_secs: u32,
}

impl Default for FailoverConfig {
    fn default() -> Self {
        Self {
            failover_threshold: 3,
            failover_cooldown_secs: 30,
        }
    }
}

/// Configuration for garbage collection of inactive models
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GcPolicy {
    /// Time to live in seconds for inactive model instances
    pub model_ttl_secs: u32,

    /// Whether garbage collection is enabled
    pub enabled: bool,
}

impl Default for GcPolicy {
    fn default() -> Self {
        Self {
            model_ttl_secs: 300, // 5 minutes
            enabled: true,
        }
    }
}

/// Configuration for STT metrics and monitoring
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MetricsConfig {
    /// Interval in seconds for periodic metrics logging
    pub log_interval_secs: Option<u32>,

    /// Enable debug dumping of transcription events
    pub debug_dump_events: bool,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            log_interval_secs: Some(60), // Log metrics every minute
            debug_dump_events: false,
        }
    }
}

// Implement From trait for easy error conversion to SttPluginError::Other
impl From<Box<dyn std::error::Error + Send + Sync>> for SttPluginError {
    fn from(error: Box<dyn std::error::Error + Send + Sync>) -> Self {
        SttPluginError::Other(error)
    }
}
