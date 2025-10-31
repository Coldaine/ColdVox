//! # Logging Module
//!
//! This module provides tracing setup and event schema for the ColdVox text injection system.

use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, info, warn, Level};

/// Initialize tracing with appropriate configuration for the text injection system
pub fn init_tracing() {
    // Note: tracing_subscriber is a dev dependency, so this function is only available
    // when the feature is enabled. In production, use the application's tracing setup.
    #[cfg(feature = "all-backends")]
    {
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(Level::INFO)
            .with_target(false)
            .with_thread_ids(true)
            .with_file(true)
            .with_line_number(true)
            .compact()
            .finish();

        tracing::subscriber::set_global_default(subscriber)
            .expect("Failed to set tracing subscriber");
    }

    #[cfg(not(feature = "all-backends"))]
    {
        // No-op when tracing-subscriber is not available
        tracing::info!("Tracing initialization skipped (tracing-subscriber feature not enabled)");
    }
}

/// Initialize tracing with custom configuration
pub fn init_tracing_with_config(config: LoggingConfig) {
    #[cfg(feature = "all-backends")]
    {
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(config.tracing_level())
            .with_target(config.include_target)
            .with_thread_ids(config.include_thread_id)
            .with_file(config.include_file)
            .with_line_number(config.include_line_number)
            .compact()
            .finish();

        tracing::subscriber::set_global_default(subscriber)
            .expect("Failed to set tracing subscriber");
    }

    #[cfg(not(feature = "all-backends"))]
    {
        // Use the config in a simple message so it's not unused in builds without the feature
        tracing::info!(
            "Tracing initialization skipped (tracing subscriber not enabled). config={:?}",
            config
        );
    }
}

/// Configuration for logging behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Maximum log level to emit (string form, e.g. "INFO")
    pub level: String,
    /// Whether to include the target module in logs
    pub include_target: bool,
    /// Whether to include thread IDs in logs
    pub include_thread_id: bool,
    /// Whether to include file names in logs
    pub include_file: bool,
    /// Whether to include line numbers in logs
    pub include_line_number: bool,
    /// Whether to redact sensitive text content
    pub redact_text: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "INFO".to_string(),
            include_target: false,
            include_thread_id: true,
            include_file: true,
            include_line_number: true,
            redact_text: true,
        }
    }
}

impl LoggingConfig {
    /// Parse configured level into a tracing::Level, defaulting to INFO on parse errors
    pub fn tracing_level(&self) -> Level {
        self.level.parse().unwrap_or(Level::INFO)
    }
}

/// Event types for injection operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InjectionEvent {
    /// Injection attempt started
    InjectionStarted {
        method: String,
        text_length: usize,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// Injection completed successfully
    InjectionCompleted {
        method: String,
        text_length: usize,
        duration_ms: u64,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// Injection failed
    InjectionFailed {
        method: String,
        error: String,
        duration_ms: u64,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// Method availability check
    MethodAvailabilityCheck {
        method: String,
        available: bool,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// Focus status check
    FocusStatusCheck {
        has_focus: bool,
        is_editable: bool,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// Window information updated
    WindowInfoUpdated {
        window_id: String,
        application: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// Backend configuration changed
    BackendConfigChanged {
        backend: String,
        config_key: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// Performance metrics
    PerformanceMetrics {
        method: String,
        avg_duration_ms: f64,
        success_rate: f64,
        total_attempts: u64,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
}

impl InjectionEvent {
    /// Log this event with appropriate tracing level
    pub fn log(&self) {
        match self {
            InjectionEvent::InjectionStarted {
                method,
                text_length,
                timestamp,
            } => {
                info!(
                    method = %method,
                    text_length = %text_length,
                    timestamp = %timestamp,
                    "Injection started"
                );
            }
            InjectionEvent::InjectionCompleted {
                method,
                text_length,
                duration_ms,
                timestamp,
            } => {
                info!(
                    method = %method,
                    text_length = %text_length,
                    duration_ms = %duration_ms,
                    timestamp = %timestamp,
                    "Injection completed successfully"
                );
            }
            InjectionEvent::InjectionFailed {
                method,
                error,
                duration_ms,
                timestamp,
            } => {
                warn!(
                    method = %method,
                    error = %error,
                    duration_ms = %duration_ms,
                    timestamp = %timestamp,
                    "Injection failed"
                );
            }
            InjectionEvent::MethodAvailabilityCheck {
                method,
                available,
                timestamp,
            } => {
                debug!(
                    method = %method,
                    available = %available,
                    timestamp = %timestamp,
                    "Method availability check"
                );
            }
            InjectionEvent::FocusStatusCheck {
                has_focus,
                is_editable,
                timestamp,
            } => {
                debug!(
                    has_focus = %has_focus,
                    is_editable = %is_editable,
                    timestamp = %timestamp,
                    "Focus status check"
                );
            }
            InjectionEvent::WindowInfoUpdated {
                window_id,
                application,
                timestamp,
            } => {
                debug!(
                    window_id = %window_id,
                    application = %application,
                    timestamp = %timestamp,
                    "Window information updated"
                );
            }
            InjectionEvent::BackendConfigChanged {
                backend,
                config_key,
                timestamp,
            } => {
                info!(
                    backend = %backend,
                    config_key = %config_key,
                    timestamp = %timestamp,
                    "Backend configuration changed"
                );
            }
            InjectionEvent::PerformanceMetrics {
                method,
                avg_duration_ms,
                success_rate,
                total_attempts,
                timestamp,
            } => {
                info!(
                    method = %method,
                    avg_duration_ms = %avg_duration_ms,
                    success_rate = %success_rate,
                    total_attempts = %total_attempts,
                    timestamp = %timestamp,
                    "Performance metrics"
                );
            }
        }
    }
}

/// Utility functions for logging injection operations
pub mod utils {
    use super::*;
    use crate::types::InjectionMethod;

    /// Log injection attempt with redaction if needed
    pub fn log_injection_attempt(method: InjectionMethod, text: &str, redact: bool) {
        let display_text = if redact {
            // Do not leak any portion of text when redaction is enabled
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            text.hash(&mut hasher);
            let hash = hasher.finish() & 0xFFFFFFFF;
            format!("[REDACTED] len={} hash={:08x}", text.len(), hash)
        } else {
            text.to_string()
        };

        info!(
            method = ?method,
            text_length = %text.len(),
            text_preview = %display_text,
            "Attempting text injection"
        );
    }

    /// Log injection success with timing
    pub fn log_injection_success(
        method: InjectionMethod,
        text: &str,
        duration: Duration,
        redact: bool,
    ) {
        let display_text = if redact {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            text.hash(&mut hasher);
            let hash = hasher.finish() & 0xFFFFFFFF;
            format!("[REDACTED] len={} hash={:08x}", text.len(), hash)
        } else {
            text.to_string()
        };

        info!(
            method = ?method,
            text_length = %text.len(),
            duration_ms = %duration.as_millis(),
            text_preview = %display_text,
            "Text injection completed successfully"
        );
    }

    /// Log injection failure with error details
    pub fn log_injection_failure(
        method: InjectionMethod,
        text: &str,
        error: &str,
        duration: Duration,
        redact: bool,
    ) {
        let display_text = if redact {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            text.hash(&mut hasher);
            let hash = hasher.finish() & 0xFFFFFFFF;
            format!("[REDACTED] len={} hash={:08x}", text.len(), hash)
        } else {
            text.to_string()
        };

        warn!(
            method = ?method,
            text_length = %text.len(),
            duration_ms = %duration.as_millis(),
            error = %error,
            text_preview = %display_text,
            "Text injection failed"
        );
    }

    /// Log method availability check
    pub fn log_method_availability(method: InjectionMethod, available: bool) {
        debug!(
            method = ?method,
            available = %available,
            "Method availability check"
        );
    }

    /// Log performance metrics
    pub fn log_performance_metrics(
        method: InjectionMethod,
        avg_duration: Duration,
        success_rate: f64,
        attempts: u64,
    ) {
        info!(
            method = ?method,
            avg_duration_ms = %avg_duration.as_millis(),
            success_rate = %success_rate,
            attempts = %attempts,
            "Performance metrics updated"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logging_config_default() {
        let config = LoggingConfig::default();
        assert_eq!(config.level, "INFO".to_string());
        assert!(!config.include_target);
        assert!(config.include_thread_id);
        assert!(config.include_file);
        assert!(config.include_line_number);
        assert!(config.redact_text);
    }

    #[test]
    fn test_injection_event_logging() {
        let event = InjectionEvent::InjectionStarted {
            method: "test".to_string(),
            text_length: 10,
            timestamp: chrono::Utc::now(),
        };

        // This should not panic
        event.log();
    }

    #[test]
    fn test_log_injection_attempt() {
        utils::log_injection_attempt(crate::types::InjectionMethod::NoOp, "test text", false);
        utils::log_injection_attempt(
            crate::types::InjectionMethod::NoOp,
            "very long test text that should be redacted",
            true,
        );
    }
}
