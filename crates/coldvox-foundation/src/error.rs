use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ColdVoxError {
    #[error(transparent)]
    Audio(#[from] AudioError),

    #[error(transparent)]
    Config(#[from] ConfigError),

    #[error(transparent)]
    Stt(#[from] SttError),

    #[error(transparent)]
    Vad(#[from] VadError),

    #[error(transparent)]
    Injection(#[from] InjectionError),

    #[error(transparent)]
    Plugin(#[from] PluginError),

    #[error("Component failed health check: {component}")]
    HealthCheckFailed { component: String },

    #[error("Shutdown requested")]
    ShutdownRequested,

    #[error("Fatal error, cannot recover: {0}")]
    Fatal(String),

    #[error("Transient error, will retry: {0}")]
    Transient(String),
}

// From trait implementations for common error types
impl From<std::io::Error> for ColdVoxError {
    fn from(err: std::io::Error) -> Self {
        ColdVoxError::Injection(InjectionError::Io(err))
    }
}

impl From<tokio::task::JoinError> for ColdVoxError {
    fn from(err: tokio::task::JoinError) -> Self {
        ColdVoxError::Transient(format!("Task join failed: {}", err))
    }
}

#[derive(Error, Debug)]
pub enum AudioError {
    #[error("Device not found: {name:?}")]
    DeviceNotFound { name: Option<String> },

    #[error("Device disconnected")]
    DeviceDisconnected,

    #[error("Format not supported: {format}")]
    FormatNotSupported { format: String },

    #[error("Buffer overflow, dropped {count} samples")]
    BufferOverflow { count: usize },

    #[error("No audio data for {duration:?}")]
    NoDataTimeout { duration: Duration },

    #[error("Silence detected for {duration:?}")]
    SilenceDetected { duration: Duration },

    #[error("CPAL error: {0}")]
    Cpal(#[from] cpal::StreamError),

    #[error("Build stream error: {0}")]
    BuildStream(#[from] cpal::BuildStreamError),

    #[error("Play stream error: {0}")]
    PlayStream(#[from] cpal::PlayStreamError),

    #[error("Fatal error, cannot recover: {0}")]
    Fatal(String),

    #[error("Supported stream configs error: {0}")]
    SupportedStreamConfigs(#[from] cpal::SupportedStreamConfigsError),
}

#[derive(Debug, thiserror::Error)]
pub enum SttError {
    #[error("Plugin not available: {reason}")]
    NotAvailable { plugin: String, reason: String },

    #[error("Transcription failed: {0}")]
    TranscriptionFailed(String),

    #[error("Plugin load failed: {0}")]
    LoadFailed(String),

    #[error("Model not found: {path}")]
    ModelNotFound { path: PathBuf },

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

#[derive(Debug, thiserror::Error)]
pub enum VadError {
    #[error("Processing failed: {0}")]
    ProcessingFailed(String),

    #[error("Invalid frame size: expected {expected}, got {actual}")]
    InvalidFrameSize { expected: usize, actual: usize },

    #[error("Model initialization failed: {0}")]
    ModelInitFailed(String),
}

#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("STT plugin error: {0}")]
    Stt(#[from] SttError),

    #[error("VAD plugin error: {0}")]
    Vad(#[from] VadError),

    #[error("Generic plugin error: {0}")]
    Generic(String),

    #[error("Plugin lifecycle error: {operation} failed for {plugin}: {reason}")]
    Lifecycle {
        plugin: String,
        operation: String,
        reason: String,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Configuration parsing error: {0}")]
    Parse(#[from] config::ConfigError),

    #[error("Validation failed: {field}: {reason}")]
    Validation { field: String, reason: String },

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Environment variable error: {0}")]
    EnvVar(String),
}

#[derive(Debug, thiserror::Error)]
pub enum InjectionError {
    #[error("No editable focus found")]
    NoEditableFocus,

    #[error("Method not available: {0}")]
    MethodNotAvailable(String),

    #[error("Timeout after {0}ms")]
    Timeout(u64),

    #[error("All methods failed: {0}")]
    AllMethodsFailed(String),

    #[error("Method unavailable: {0}")]
    MethodUnavailable(String),

    #[error("Method failed: {0}")]
    MethodFailed(String),

    #[error("Budget exhausted")]
    BudgetExhausted,

    #[error("Clipboard error: {0}")]
    Clipboard(String),

    #[error("Process error: {0}")]
    Process(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Other error: {0}")]
    Other(String),
}

/// Device status events for monitoring audio device changes
#[derive(Debug, Clone, PartialEq)]
pub enum DeviceEvent {
    /// A new device was detected
    DeviceAdded { name: String },
    /// A device was removed
    DeviceRemoved { name: String },
    /// Current device was disconnected
    CurrentDeviceDisconnected { name: String },
    /// Successfully switched to a new device
    DeviceSwitched { from: Option<String>, to: String },
    /// Failed to switch device, using fallback
    DeviceSwitchFailed {
        attempted: String,
        fallback: Option<String>,
    },
    /// Request to manually switch to a specific device
    DeviceSwitchRequested { target: String },
}

/// Device status information
#[derive(Debug, Clone)]
pub struct DeviceStatus {
    pub name: String,
    pub is_current: bool,
    pub is_available: bool,
    pub is_default: bool,
    pub last_seen: std::time::Instant,
}

#[derive(Debug, Clone)]
pub enum RecoveryStrategy {
    Retry { max_attempts: u32, delay: Duration },
    Fallback { to: String },
    Restart,
    Ignore,
    Fatal,
}

impl ColdVoxError {
    pub fn recovery_strategy(&self) -> RecoveryStrategy {
        match self {
            ColdVoxError::Audio(AudioError::DeviceDisconnected) => RecoveryStrategy::Retry {
                max_attempts: 5,
                delay: Duration::from_secs(2),
            },
            ColdVoxError::Audio(AudioError::DeviceNotFound { .. }) => RecoveryStrategy::Fallback {
                to: "default".into(),
            },
            ColdVoxError::Audio(AudioError::BufferOverflow { .. }) => RecoveryStrategy::Ignore,
            ColdVoxError::Fatal(_) | ColdVoxError::ShutdownRequested => RecoveryStrategy::Fatal,
            _ => RecoveryStrategy::Restart,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AudioConfig {
    pub silence_threshold: i16,
    /// Ring buffer capacity in samples. At 16kHz mono, 65536 samples ≈ 4.1 seconds.
    /// Larger buffers provide more headroom for downstream processing spikes but increase
    /// worst-case latency. The default (65536) is sized to prevent overflows during
    /// typical STT/VAD/text-injection processing.
    pub capture_buffer_samples: usize,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            silence_threshold: 100,
            capture_buffer_samples: 65_536, // 16_384 * 4, ~4.1s at 16kHz
        }
    }
}
