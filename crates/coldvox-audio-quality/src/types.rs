//! Types for audio quality status and warnings.

use std::fmt;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Quality status of the current audio frame.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum QualityStatus {
    /// Audio quality is good.
    Good {
        /// Current RMS level in dBFS
        rms_dbfs: f32,
        /// Current peak level in dBFS
        peak_dbfs: f32,
    },
    /// Audio quality has an issue that needs attention.
    Warning(QualityWarning),
}

/// Specific type of quality warning.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum QualityWarning {
    /// Audio level is too quiet.
    TooQuiet {
        /// Current RMS level in dBFS
        rms_dbfs: f32,
    },
    /// Audio is clipping (too loud).
    Clipping {
        /// Peak level that caused clipping, in dBFS
        peak_dbfs: f32,
    },
    /// Speaker is off-axis from microphone.
    OffAxis {
        /// High-freq to mid-freq ratio (< 0.3 typically indicates off-axis)
        spectral_ratio: f32,
    },
}

impl QualityStatus {
    /// Check if this status represents a warning condition.
    pub fn needs_warning(&self) -> bool {
        matches!(self, QualityStatus::Warning(_))
    }

    /// Get a human-readable message for this status.
    pub fn message(&self) -> String {
        match self {
            QualityStatus::Good { rms_dbfs, .. } => {
                format!("Audio quality good ({:.1} dBFS)", rms_dbfs)
            }
            QualityStatus::Warning(warning) => warning.message(),
        }
    }

    /// Get severity level (0 = good, 1 = warning, 2 = critical).
    pub fn severity(&self) -> u8 {
        match self {
            QualityStatus::Good { .. } => 0,
            QualityStatus::Warning(QualityWarning::TooQuiet { .. }) => 1,
            QualityStatus::Warning(QualityWarning::OffAxis { .. }) => 1,
            QualityStatus::Warning(QualityWarning::Clipping { .. }) => 2,
        }
    }
}

impl QualityWarning {
    /// Get a human-readable message for this warning.
    pub fn message(&self) -> String {
        match self {
            QualityWarning::TooQuiet { rms_dbfs } => {
                format!(
                    "Audio too quiet ({:.1} dBFS) - Speak louder or increase mic gain",
                    rms_dbfs
                )
            }
            QualityWarning::Clipping { peak_dbfs } => {
                format!(
                    "Audio clipping ({:.1} dBFS) - Reduce mic gain",
                    peak_dbfs
                )
            }
            QualityWarning::OffAxis { spectral_ratio } => {
                format!(
                    "Speaker off-axis (ratio: {:.2}) - Move in front of microphone",
                    spectral_ratio
                )
            }
        }
    }

    /// Get suggested action for this warning.
    pub fn suggested_action(&self) -> &str {
        match self {
            QualityWarning::TooQuiet { .. } => "Speak louder or increase microphone gain",
            QualityWarning::Clipping { .. } => "Reduce microphone gain (dial on QuadCast)",
            QualityWarning::OffAxis { .. } => "Move back in front of microphone",
        }
    }

    /// Get warning type as string (for metrics/logging).
    pub fn warning_type(&self) -> &str {
        match self {
            QualityWarning::TooQuiet { .. } => "too_quiet",
            QualityWarning::Clipping { .. } => "clipping",
            QualityWarning::OffAxis { .. } => "off_axis",
        }
    }
}

impl fmt::Display for QualityStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl fmt::Display for QualityWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quality_status_good() {
        let status = QualityStatus::Good {
            rms_dbfs: -20.0,
            peak_dbfs: -10.0,
        };

        assert!(!status.needs_warning());
        assert_eq!(status.severity(), 0);
        assert!(status.message().contains("good"));
    }

    #[test]
    fn test_quality_warning_too_quiet() {
        let warning = QualityWarning::TooQuiet { rms_dbfs: -50.0 };

        assert_eq!(warning.warning_type(), "too_quiet");
        assert!(warning.message().contains("too quiet"));
        assert!(warning.suggested_action().contains("louder"));
    }

    #[test]
    fn test_quality_warning_clipping() {
        let warning = QualityWarning::Clipping { peak_dbfs: -0.5 };

        assert_eq!(warning.warning_type(), "clipping");
        assert!(warning.message().contains("clipping"));
        assert!(warning.suggested_action().contains("Reduce"));
    }

    #[test]
    fn test_quality_warning_off_axis() {
        let warning = QualityWarning::OffAxis {
            spectral_ratio: 0.2,
        };

        assert_eq!(warning.warning_type(), "off_axis");
        assert!(warning.message().contains("off-axis"));
        assert!(warning.suggested_action().contains("front"));
    }

    #[test]
    fn test_severity_levels() {
        let good = QualityStatus::Good {
            rms_dbfs: -20.0,
            peak_dbfs: -10.0,
        };
        let too_quiet = QualityStatus::Warning(QualityWarning::TooQuiet { rms_dbfs: -50.0 });
        let clipping = QualityStatus::Warning(QualityWarning::Clipping { peak_dbfs: -0.5 });

        assert_eq!(good.severity(), 0);
        assert_eq!(too_quiet.severity(), 1);
        assert_eq!(clipping.severity(), 2);
    }
}
