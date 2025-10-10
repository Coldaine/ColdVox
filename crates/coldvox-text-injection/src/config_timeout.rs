//! # Timeout and Latency Configuration
//!
//! This module centralizes the parsing and validation of timeout and latency settings
//! for the text injection system. It provides a single source of truth for timing-related
//! configurations, ensuring that values are clamped to sensible ranges and are easily
//! accessible as `std::time::Duration` objects.
//!
//! This approach avoids scattering magic numbers and ad-hoc parsing logic across the
//! codebase, making the behavior more predictable and easier to test.

use crate::types::InjectionConfig;
use std::time::Duration;

/// A validated and clamped representation of all timing-related configurations.
#[derive(Debug, Clone, Copy)]
pub struct TimeoutConfig {
    /// Overall latency budget for a single injection call.
    max_total_latency: Duration,
    /// Timeout for individual injection method attempts.
    per_method_timeout: Duration,
    /// Timeout specifically for a paste action.
    paste_action_timeout: Duration,
    /// Initial cooldown period after a method fails.
    cooldown_initial: Duration,
    /// Backoff factor for exponential cooldown.
    cooldown_backoff_factor: f32,
    /// Maximum cooldown period.
    cooldown_max: Duration,
    /// Delay between paste chunks.
    chunk_delay: Duration,
    /// Delay for keystroke pacing.
    keystroke_delay: Duration,
    /// Cache duration for focus status.
    focus_cache_duration: Duration,
    /// Delay before restoring clipboard.
    clipboard_restore_delay: Duration,
    /// Timeout for window discovery operations.
    discovery_timeout: Duration,
}

impl TimeoutConfig {
    /// Minimum allowed timeout to prevent busy-loops or instant timeouts.
    const MIN_TIMEOUT_MS: u64 = 10;
    /// Maximum allowed timeout to prevent excessively long waits.
    const MAX_TIMEOUT_MS: u64 = 30_000; // 30 seconds
    /// Maximum cooldown to prevent a backend from being disabled for too long.
    const MAX_COOLDOWN_MS: u64 = 600_000; // 10 minutes

    /// Creates a new `TimeoutConfig` from the raw `InjectionConfig`,
    /// applying validation and clamping to all values.
    pub fn new(config: &InjectionConfig) -> Self {
        Self {
            max_total_latency: Self::clamp_duration(
                config.max_total_latency_ms,
                Self::MIN_TIMEOUT_MS,
                Self::MAX_TIMEOUT_MS,
            ),
            per_method_timeout: Self::clamp_duration(
                config.per_method_timeout_ms,
                Self::MIN_TIMEOUT_MS,
                Self::MAX_TIMEOUT_MS,
            ),
            paste_action_timeout: Self::clamp_duration(
                config.paste_action_timeout_ms,
                Self::MIN_TIMEOUT_MS,
                Self::MAX_TIMEOUT_MS,
            ),
            cooldown_initial: Self::clamp_duration(
                config.cooldown_initial_ms,
                Self::MIN_TIMEOUT_MS,
                Self::MAX_COOLDOWN_MS,
            ),
            cooldown_backoff_factor: config.cooldown_backoff_factor.max(1.0),
            cooldown_max: Self::clamp_duration(
                config.cooldown_max_ms,
                Self::MIN_TIMEOUT_MS,
                Self::MAX_COOLDOWN_MS,
            ),
            chunk_delay: Duration::from_millis(config.chunk_delay_ms),
            keystroke_delay: if config.keystroke_rate_cps > 0 {
                Duration::from_millis(1000 / config.keystroke_rate_cps as u64)
            } else {
                Duration::from_millis(50) // Default to 20 cps
            },
            focus_cache_duration: Duration::from_millis(config.focus_cache_duration_ms),
            clipboard_restore_delay: Duration::from_millis(
                config.clipboard_restore_delay_ms.unwrap_or(500),
            ),
            discovery_timeout: Self::clamp_duration(
                config.discovery_timeout_ms,
                Self::MIN_TIMEOUT_MS,
                Self::MAX_TIMEOUT_MS,
            ),
        }
    }

    /// Helper to clamp a millisecond value and convert it to a `Duration`.
    fn clamp_duration(ms: u64, min: u64, max: u64) -> Duration {
        Duration::from_millis(ms.clamp(min, max))
    }

    // --- Accessors ---

    pub fn max_total_latency(&self) -> Duration {
        self.max_total_latency
    }

    pub fn per_method_timeout(&self) -> Duration {
        self.per_method_timeout
    }

    pub fn paste_action_timeout(&self) -> Duration {
        self.paste_action_timeout
    }

    pub fn cooldown_initial(&self) -> Duration {
        self.cooldown_initial
    }

    pub fn cooldown_backoff_factor(&self) -> f32 {
        self.cooldown_backoff_factor
    }

    pub fn cooldown_max(&self) -> Duration {
        self.cooldown_max
    }

    pub fn chunk_delay(&self) -> Duration {
        self.chunk_delay
    }

    pub fn keystroke_delay(&self) -> Duration {
        self.keystroke_delay
    }

    pub fn focus_cache_duration(&self) -> Duration {
        self.focus_cache_duration
    }

    pub fn clipboard_restore_delay(&self) -> Duration {
        self.clipboard_restore_delay
    }

    pub fn discovery_timeout(&self) -> Duration {
        self.discovery_timeout
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::InjectionConfig;

    #[test]
    fn test_timeout_clamping() {
        let config = InjectionConfig {
            max_total_latency_ms: 5,      // Too low
            per_method_timeout_ms: 50000, // Too high
            cooldown_initial_ms: 100,     // Valid
            ..Default::default()
        };

        let timeout_config = TimeoutConfig::new(&config);

        assert_eq!(
            timeout_config.max_total_latency(),
            Duration::from_millis(TimeoutConfig::MIN_TIMEOUT_MS)
        );
        assert_eq!(
            timeout_config.per_method_timeout(),
            Duration::from_millis(TimeoutConfig::MAX_TIMEOUT_MS)
        );
        assert_eq!(
            timeout_config.cooldown_initial(),
            Duration::from_millis(100)
        );
    }

    #[test]
    fn test_backoff_factor_clamping() {
        let config = InjectionConfig {
            cooldown_backoff_factor: 0.5, // Too low
            ..Default::default()
        };

        let timeout_config = TimeoutConfig::new(&config);
        assert_eq!(timeout_config.cooldown_backoff_factor(), 1.0);

        let config = InjectionConfig {
            cooldown_backoff_factor: 2.5, // Valid
            ..Default::default()
        };
        let timeout_config = TimeoutConfig::new(&config);
        assert_eq!(timeout_config.cooldown_backoff_factor(), 2.5);
    }

    #[test]
    fn test_keystroke_delay_calculation() {
        let config = InjectionConfig {
            keystroke_rate_cps: 50, // 20ms delay
            ..Default::default()
        };
        let timeout_config = TimeoutConfig::new(&config);
        assert_eq!(timeout_config.keystroke_delay(), Duration::from_millis(20));

        // Test with zero rate to avoid division by zero
        let config = InjectionConfig {
            keystroke_rate_cps: 0,
            ..Default::default()
        };
        let timeout_config = TimeoutConfig::new(&config);
        assert_eq!(timeout_config.keystroke_delay(), Duration::from_millis(50));
    }
}
