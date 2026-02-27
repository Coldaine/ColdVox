//! Configuration for audio quality monitoring.

use std::time::Duration;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Configuration for audio quality monitoring.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct QualityConfig {
    /// Sample rate in Hz (e.g., 16000 for 16kHz).
    pub sample_rate: u32,

    /// Threshold for "too quiet" warning in dBFS.
    /// Typical range: -60 to -30 dBFS
    /// Default: -40 dBFS
    pub too_quiet_threshold_dbfs: f32,

    /// Threshold for clipping warning in dBFS.
    /// Typical range: -3 to 0 dBFS
    /// Default: -1 dBFS
    pub clipping_threshold_dbfs: f32,

    /// Optimal RMS range (for UI visualization).
    pub optimal_min_dbfs: f32,
    pub optimal_max_dbfs: f32,

    /// Rolling window duration for RMS calculation in milliseconds.
    /// Default: 500ms (balances smoothness vs responsiveness)
    pub rms_window_ms: u64,

    /// Peak hold duration in milliseconds.
    /// Default: 1000ms (1 second)
    pub peak_hold_ms: u64,

    /// Enable off-axis detection via spectral analysis.
    /// Default: true
    pub enable_off_axis_detection: bool,

    /// Threshold for off-axis detection.
    /// Ratio of high-freq (4-8kHz) to mid-freq (500Hz-2kHz) energy.
    /// Below this threshold indicates speaker is off-axis.
    /// Typical range: 0.2 to 0.5
    /// Default: 0.3 (empirically tuned for cardioid mics)
    pub off_axis_threshold: f32,

    /// Minimum time between warnings to prevent spam.
    /// Default: 2 seconds
    pub warning_cooldown: Duration,
}

impl Default for QualityConfig {
    fn default() -> Self {
        Self {
            sample_rate: 16000,
            too_quiet_threshold_dbfs: -40.0,
            clipping_threshold_dbfs: -1.0,
            optimal_min_dbfs: -25.0,
            optimal_max_dbfs: -12.0,
            rms_window_ms: 500,
            peak_hold_ms: 1000,
            enable_off_axis_detection: true,
            off_axis_threshold: 0.3,
            warning_cooldown: Duration::from_secs(2),
        }
    }
}

impl QualityConfig {
    /// Create a new configuration builder.
    pub fn builder() -> QualityConfigBuilder {
        QualityConfigBuilder::default()
    }

    /// Create configuration optimized for HyperX QuadCast in cardioid mode.
    pub fn hyperx_quadcast_cardioid() -> Self {
        Self {
            // QuadCast is sensitive, tends to be louder
            too_quiet_threshold_dbfs: -35.0, // Slightly higher threshold
            off_axis_threshold: 0.35,         // QuadCast has tight cardioid pattern
            ..Default::default()
        }
    }

    /// Create configuration optimized for omnidirectional microphones.
    pub fn omnidirectional() -> Self {
        Self {
            // Omni mics don't have off-axis issues
            enable_off_axis_detection: false,
            ..Default::default()
        }
    }

    /// Load configuration from environment variables.
    ///
    /// Supported variables:
    /// - `COLDVOX_TOO_QUIET_THRESHOLD`: Threshold in dBFS
    /// - `COLDVOX_CLIPPING_THRESHOLD`: Threshold in dBFS
    /// - `COLDVOX_OFF_AXIS_THRESHOLD`: Spectral ratio threshold
    /// - `COLDVOX_DISABLE_OFF_AXIS`: Set to "1" to disable
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(val) = std::env::var("COLDVOX_TOO_QUIET_THRESHOLD") {
            if let Ok(threshold) = val.parse::<f32>() {
                config.too_quiet_threshold_dbfs = threshold;
            }
        }

        if let Ok(val) = std::env::var("COLDVOX_CLIPPING_THRESHOLD") {
            if let Ok(threshold) = val.parse::<f32>() {
                config.clipping_threshold_dbfs = threshold;
            }
        }

        if let Ok(val) = std::env::var("COLDVOX_OFF_AXIS_THRESHOLD") {
            if let Ok(threshold) = val.parse::<f32>() {
                config.off_axis_threshold = threshold;
            }
        }

        if let Ok(val) = std::env::var("COLDVOX_DISABLE_OFF_AXIS") {
            if val == "1" || val.eq_ignore_ascii_case("true") {
                config.enable_off_axis_detection = false;
            }
        }

        config
    }
}

/// Builder for QualityConfig.
#[derive(Debug, Default)]
pub struct QualityConfigBuilder {
    sample_rate: Option<u32>,
    too_quiet_threshold_dbfs: Option<f32>,
    clipping_threshold_dbfs: Option<f32>,
    optimal_min_dbfs: Option<f32>,
    optimal_max_dbfs: Option<f32>,
    rms_window_ms: Option<u64>,
    peak_hold_ms: Option<u64>,
    enable_off_axis_detection: Option<bool>,
    off_axis_threshold: Option<f32>,
    warning_cooldown: Option<Duration>,
}

impl QualityConfigBuilder {
    pub fn sample_rate(mut self, sample_rate: u32) -> Self {
        self.sample_rate = Some(sample_rate);
        self
    }

    pub fn too_quiet_threshold_dbfs(mut self, threshold: f32) -> Self {
        self.too_quiet_threshold_dbfs = Some(threshold);
        self
    }

    pub fn clipping_threshold_dbfs(mut self, threshold: f32) -> Self {
        self.clipping_threshold_dbfs = Some(threshold);
        self
    }

    pub fn enable_off_axis_detection(mut self, enable: bool) -> Self {
        self.enable_off_axis_detection = Some(enable);
        self
    }

    pub fn off_axis_threshold(mut self, threshold: f32) -> Self {
        self.off_axis_threshold = Some(threshold);
        self
    }

    pub fn build(self) -> QualityConfig {
        let defaults = QualityConfig::default();

        QualityConfig {
            sample_rate: self.sample_rate.unwrap_or(defaults.sample_rate),
            too_quiet_threshold_dbfs: self
                .too_quiet_threshold_dbfs
                .unwrap_or(defaults.too_quiet_threshold_dbfs),
            clipping_threshold_dbfs: self
                .clipping_threshold_dbfs
                .unwrap_or(defaults.clipping_threshold_dbfs),
            optimal_min_dbfs: self.optimal_min_dbfs.unwrap_or(defaults.optimal_min_dbfs),
            optimal_max_dbfs: self.optimal_max_dbfs.unwrap_or(defaults.optimal_max_dbfs),
            rms_window_ms: self.rms_window_ms.unwrap_or(defaults.rms_window_ms),
            peak_hold_ms: self.peak_hold_ms.unwrap_or(defaults.peak_hold_ms),
            enable_off_axis_detection: self
                .enable_off_axis_detection
                .unwrap_or(defaults.enable_off_axis_detection),
            off_axis_threshold: self
                .off_axis_threshold
                .unwrap_or(defaults.off_axis_threshold),
            warning_cooldown: self
                .warning_cooldown
                .unwrap_or(defaults.warning_cooldown),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = QualityConfig::default();

        assert_eq!(config.sample_rate, 16000);
        assert_eq!(config.too_quiet_threshold_dbfs, -40.0);
        assert_eq!(config.clipping_threshold_dbfs, -1.0);
        assert!(config.enable_off_axis_detection);
    }

    #[test]
    fn test_config_builder() {
        let config = QualityConfig::builder()
            .sample_rate(48000)
            .too_quiet_threshold_dbfs(-50.0)
            .enable_off_axis_detection(false)
            .build();

        assert_eq!(config.sample_rate, 48000);
        assert_eq!(config.too_quiet_threshold_dbfs, -50.0);
        assert!(!config.enable_off_axis_detection);
    }

    #[test]
    fn test_hyperx_quadcast_preset() {
        let config = QualityConfig::hyperx_quadcast_cardioid();

        assert_eq!(config.too_quiet_threshold_dbfs, -35.0);
        assert_eq!(config.off_axis_threshold, 0.35);
    }

    #[test]
    fn test_omnidirectional_preset() {
        let config = QualityConfig::omnidirectional();

        assert!(!config.enable_off_axis_detection);
    }
}
