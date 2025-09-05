use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Audio subsystem error: {0}")]
    Audio(#[from] AudioError),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Component failed health check: {component}")]
    HealthCheckFailed { component: String },

    #[error("Shutdown requested")]
    ShutdownRequested,

    #[error("Fatal error, cannot recover: {0}")]
    Fatal(String),

    #[error("Transient error, will retry: {0}")]
    Transient(String),
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
    DeviceSwitchFailed { attempted: String, fallback: Option<String> },
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

impl AppError {
    pub fn recovery_strategy(&self) -> RecoveryStrategy {
        match self {
            AppError::Audio(AudioError::DeviceDisconnected) => RecoveryStrategy::Retry {
                max_attempts: 5,
                delay: Duration::from_secs(2),
            },
            AppError::Audio(AudioError::DeviceNotFound { .. }) => RecoveryStrategy::Fallback {
                to: "default".into(),
            },
            AppError::Audio(AudioError::BufferOverflow { .. }) => RecoveryStrategy::Ignore,
            AppError::Fatal(_) | AppError::ShutdownRequested => RecoveryStrategy::Fatal,
            _ => RecoveryStrategy::Restart,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AudioConfig {
    pub silence_threshold: i16,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            silence_threshold: 100,
        }
    }
}
