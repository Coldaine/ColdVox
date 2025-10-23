//! Enhanced plugin types and capabilities for STT system

use std::fmt::Debug;

/// Accuracy level of an STT plugin
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AccuracyLevel {
    /// ~70% accuracy - Very basic recognition (e.g., Pocketsphinx)
    Low,
    /// ~85% accuracy - Acceptable for non-critical use (e.g., Parakeet, Silero)
    Medium,
    /// ~95% accuracy - Good for most applications (e.g., Whisper, Coqui)
    High,
    /// ~98%+ accuracy - Professional grade (e.g., Whisper, Cloud services)
    VeryHigh,
}

/// Latency profile of an STT plugin
#[derive(Debug, Clone)]
pub struct LatencyProfile {
    /// Average latency in milliseconds
    pub avg_ms: u32,
    /// P95 latency in milliseconds
    pub p95_ms: u32,
    /// P99 latency in milliseconds
    pub p99_ms: u32,
    /// Real-time factor (processing time / audio duration)
    pub rtf: f32,
}

impl Default for LatencyProfile {
    fn default() -> Self {
        Self {
            avg_ms: 100,
            p95_ms: 200,
            p99_ms: 500,
            rtf: 0.5,
        }
    }
}

/// Resource usage profile
#[derive(Debug, Clone)]
pub struct ResourceProfile {
    /// Peak memory usage in MB
    pub peak_memory_mb: u32,
    /// Average CPU usage percentage
    pub avg_cpu_percent: f32,
    /// GPU usage if applicable
    pub uses_gpu: bool,
    /// Disk space for models in MB
    pub disk_space_mb: u32,
}

impl Default for ResourceProfile {
    fn default() -> Self {
        Self {
            peak_memory_mb: 100,
            avg_cpu_percent: 10.0,
            uses_gpu: false,
            disk_space_mb: 50,
        }
    }
}

/// Model size categories
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelSize {
    /// < 50MB - Ultra lightweight
    Tiny,
    /// 50-200MB - Lightweight
    Small,
    /// 200-500MB - Medium
    Medium,
    /// 500MB-1GB - Large
    Large,
    /// > 1GB - Extra large
    ExtraLarge,
}

impl ModelSize {
    pub fn from_mb(size_mb: u32) -> Self {
        match size_mb {
            0..=49 => Self::Tiny,
            50..=199 => Self::Small,
            200..=499 => Self::Medium,
            500..=999 => Self::Large,
            _ => Self::ExtraLarge,
        }
    }
}

/// Language support with confidence levels
#[derive(Debug, Clone)]
pub struct LanguageSupport {
    /// ISO 639-1 language code (e.g., "en", "es", "zh")
    pub code: String,
    /// Full language name
    pub name: String,
    /// Quality of support for this language
    pub quality: LanguageQuality,
    /// Available dialects/variants
    pub variants: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageQuality {
    /// Experimental support
    Alpha,
    /// Basic support, may have issues
    Beta,
    /// Production ready
    Stable,
    /// Optimized and well-tested
    Premium,
}

/// Plugin runtime metrics
#[derive(Debug, Clone)]
pub struct PluginMetrics {
    /// Total audio processed in milliseconds
    pub total_audio_ms: u64,
    /// Total number of transcriptions
    pub total_transcriptions: u64,
    /// Total number of errors
    pub total_errors: u64,
    /// Average latency in milliseconds
    pub avg_latency_ms: f64,
    /// Current memory usage in MB
    pub current_memory_mb: u32,
    /// Peak memory usage in MB
    pub peak_memory_mb: u32,
    /// Average CPU usage percentage
    pub avg_cpu_percent: f32,
    /// Average confidence score (0-1)
    pub avg_confidence: f32,
    /// Last update timestamp
    pub last_updated: std::time::Instant,
}

impl Default for PluginMetrics {
    fn default() -> Self {
        Self {
            total_audio_ms: 0,
            total_transcriptions: 0,
            total_errors: 0,
            avg_latency_ms: 0.0,
            current_memory_mb: 0,
            peak_memory_mb: 0,
            avg_cpu_percent: 0.0,
            avg_confidence: 0.0,
            last_updated: std::time::Instant::now(),
        }
    }
}

impl PluginMetrics {
    /// Update metrics with a new transcription
    pub fn record_transcription(
        &mut self,
        audio_duration_ms: u64,
        latency_ms: u64,
        confidence: f32,
    ) {
        self.total_audio_ms += audio_duration_ms;
        self.total_transcriptions += 1;

        // Update average latency (running average)
        let n = self.total_transcriptions as f64;
        self.avg_latency_ms = (self.avg_latency_ms * (n - 1.0) + latency_ms as f64) / n;

        // Update average confidence
        self.avg_confidence = (self.avg_confidence * (n - 1.0) as f32 + confidence) / n as f32;

        self.last_updated = std::time::Instant::now();
    }

    /// Record an error
    pub fn record_error(&mut self) {
        self.total_errors += 1;
        self.last_updated = std::time::Instant::now();
    }

    /// Update resource usage
    pub fn update_resources(&mut self, memory_mb: u32, cpu_percent: f32) {
        self.current_memory_mb = memory_mb;
        if memory_mb > self.peak_memory_mb {
            self.peak_memory_mb = memory_mb;
        }

        // Update average CPU (exponential moving average)
        self.avg_cpu_percent = self.avg_cpu_percent * 0.9 + cpu_percent * 0.1;

        self.last_updated = std::time::Instant::now();
    }

    /// Calculate success rate
    pub fn success_rate(&self) -> f32 {
        let total_attempts = self.total_transcriptions + self.total_errors;
        if total_attempts == 0 {
            return 1.0;
        }
        self.total_transcriptions as f32 / total_attempts as f32
    }
}

/// Plugin selection constraints
#[derive(Debug, Clone, Default)]
pub struct SelectionConstraints {
    /// Maximum memory usage in MB
    pub max_memory_mb: Option<u32>,
    /// Maximum latency in milliseconds
    pub max_latency_ms: Option<u32>,
    /// Minimum accuracy level
    pub min_accuracy: Option<AccuracyLevel>,
    /// Require offline operation (no cloud)
    pub require_offline: bool,
    /// Require streaming support
    pub require_streaming: bool,
    /// Required language support
    pub required_languages: Vec<String>,
    /// Maximum model size
    pub max_model_size: Option<ModelSize>,
    /// Require GPU support
    pub require_gpu: Option<bool>,
}

/// User preferences for plugin selection
#[derive(Debug, Clone)]
pub struct PreferenceWeights {
    /// Weight for accuracy (0-1)
    pub accuracy_weight: f32,
    /// Weight for speed (0-1)
    pub speed_weight: f32,
    /// Weight for memory usage (0-1)
    pub memory_weight: f32,
    /// Weight for stability (0-1)
    pub stability_weight: f32,
}

impl Default for PreferenceWeights {
    fn default() -> Self {
        Self {
            accuracy_weight: 0.4,
            speed_weight: 0.3,
            memory_weight: 0.2,
            stability_weight: 0.1,
        }
    }
}

/// Feedback from user for adaptive learning
#[derive(Debug, Clone)]
pub enum UserFeedback {
    /// User corrected a transcription
    Correction {
        plugin_id: String,
        original: String,
        corrected: String,
    },
    /// User reported slow performance
    TooSlow {
        plugin_id: String,
        audio_duration_ms: u64,
        actual_latency_ms: u64,
    },
    /// User manually selected a plugin
    ManualSelection {
        from_plugin: String,
        to_plugin: String,
        reason: Option<String>,
    },
    /// User reported good experience
    Satisfied { plugin_id: String },
}

/// Plugin state for lifecycle management
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginState {
    /// Plugin is not initialized
    Uninitialized,
    /// Plugin is loading resources
    Loading,
    /// Plugin is ready to use
    Ready,
    /// Plugin is currently processing
    Processing,
    /// Plugin encountered an error
    Error,
    /// Plugin is shutting down
    ShuttingDown,
}

/// Plugin discovery source
#[derive(Debug, Clone)]
pub enum PluginSource {
    /// Built into the application
    BuiltIn,
    /// Loaded from system path
    System(std::path::PathBuf),
    /// Downloaded from URL
    Downloaded(String),
    /// Loaded as dynamic library
    DynamicLibrary(std::path::PathBuf),
    /// Cloud service
    CloudService(String),
}

/// Enhanced plugin information
#[derive(Debug, Clone)]
pub struct EnhancedPluginInfo {
    /// Basic plugin info (existing)
    pub id: String,
    pub name: String,
    pub description: String,

    /// Enhanced metadata
    pub version: String,
    pub author: String,
    pub license: String,
    pub homepage: Option<String>,

    /// Performance characteristics
    pub accuracy_level: AccuracyLevel,
    pub latency_profile: LatencyProfile,
    pub resource_profile: ResourceProfile,
    pub model_size: ModelSize,

    /// Language support
    pub languages: Vec<LanguageSupport>,

    /// Requirements
    pub requires_internet: bool,
    pub requires_gpu: bool,
    pub requires_license_key: bool,

    /// Status
    pub is_beta: bool,
    pub is_deprecated: bool,
    pub source: PluginSource,

    /// Metrics (if available)
    pub metrics: Option<PluginMetrics>,
}
