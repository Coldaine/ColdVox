use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};
use tracing::{trace, warn};

/// Throttles repeated log messages to reduce noise
/// Thread-safe throttling with configurable intervals per message type
pub struct LogThrottle {
    last_logged: HashMap<String, Instant>,
    throttle_duration: Duration,
}

impl LogThrottle {
    pub fn new() -> Self {
        Self {
            last_logged: HashMap::new(),
            throttle_duration: Duration::from_secs(30),
        }
    }

    pub fn with_duration(duration: Duration) -> Self {
        Self {
            last_logged: HashMap::new(),
            throttle_duration: duration,
        }
    }

    /// Check if a log message should be emitted based on throttling rules
    /// Returns true if message should be logged, false if it should be suppressed
    pub fn should_log(&mut self, key: &str) -> bool {
        let now = Instant::now();
        match self.last_logged.get(key) {
            Some(last_time) => {
                if now.duration_since(*last_time) > self.throttle_duration {
                    self.last_logged.insert(key.to_string(), now);
                    true
                } else {
                    false
                }
            }
            None => {
                self.last_logged.insert(key.to_string(), now);
                true
            }
        }
    }

    /// Clean up old entries to prevent memory growth
    /// Should be called periodically (e.g., every few minutes)
    pub fn cleanup_old_entries(&mut self) {
        let now = Instant::now();
        let cleanup_threshold = self.throttle_duration * 2;

        self.last_logged
            .retain(|_, &mut last_time| now.duration_since(last_time) <= cleanup_threshold);
    }
}

impl Default for LogThrottle {
    fn default() -> Self {
        Self::new()
    }
}

/// Global counter for AT-SPI UnknownMethod warnings
/// Uses atomic operations for thread safety
static ATSPI_UNKNOWN_METHOD_WARN_COUNT: AtomicU32 = AtomicU32::new(0);

/// Log AT-SPI UnknownMethod error with suppression after first occurrence
/// First occurrence logs at WARN level, subsequent occurrences at TRACE level
pub fn log_atspi_unknown_method(error: &str) {
    let count = ATSPI_UNKNOWN_METHOD_WARN_COUNT.fetch_add(1, Ordering::Relaxed);

    if count == 0 {
        warn!("AT-SPI UnknownMethod error (first occurrence): {}", error);
    } else {
        trace!(
            "AT-SPI UnknownMethod error (suppressed, count {}): {}",
            count + 1,
            error
        );
    }
}

/// Global counter for AT-SPI connection failure warnings
static ATSPI_CONNECTION_WARN_COUNT: AtomicU32 = AtomicU32::new(0);

/// Log AT-SPI connection failures with suppression after first occurrence
pub fn log_atspi_connection_failure(error: &str) {
    let count = ATSPI_CONNECTION_WARN_COUNT.fetch_add(1, Ordering::Relaxed);

    if count == 0 {
        warn!("AT-SPI connection failed (first occurrence): {}", error);
    } else {
        trace!(
            "AT-SPI connection failed (suppressed, count {}): {}",
            count + 1,
            error
        );
    }
}

/// Reset suppression counters (useful for tests or long-running applications)
pub fn reset_suppression_counters() {
    ATSPI_UNKNOWN_METHOD_WARN_COUNT.store(0, Ordering::Relaxed);
    ATSPI_CONNECTION_WARN_COUNT.store(0, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_log_throttle_allows_first_message() {
        let mut throttle = LogThrottle::new();

        assert!(throttle.should_log("test_key"));
        assert!(!throttle.should_log("test_key"));
    }

    #[test]
    fn test_log_throttle_allows_after_duration() {
        let mut throttle = LogThrottle::with_duration(Duration::from_millis(10));

        assert!(throttle.should_log("test_key"));
        assert!(!throttle.should_log("test_key"));

        // Wait for duration to pass
        std::thread::sleep(Duration::from_millis(15));

        assert!(throttle.should_log("test_key"));
    }

    #[test]
    fn test_log_throttle_different_keys() {
        let mut throttle = LogThrottle::new();

        assert!(throttle.should_log("key1"));
        assert!(throttle.should_log("key2"));
        assert!(!throttle.should_log("key1"));
        assert!(!throttle.should_log("key2"));
    }

    #[test]
    fn test_cleanup_old_entries() {
        let mut throttle = LogThrottle::with_duration(Duration::from_millis(10));

        // Add entries
        throttle.should_log("key1");
        throttle.should_log("key2");

        assert_eq!(throttle.last_logged.len(), 2);

        // Wait and cleanup
        std::thread::sleep(Duration::from_millis(25));
        throttle.cleanup_old_entries();

        // Entries should be cleaned up
        assert_eq!(throttle.last_logged.len(), 0);
    }

    #[test]
    fn test_atspi_unknown_method_suppression() {
        reset_suppression_counters();

        // First call should go to warn level (we can't test the actual log output easily,
        // but we can test the counter behavior)
        log_atspi_unknown_method("test error 1");
        log_atspi_unknown_method("test error 2");

        let count = ATSPI_UNKNOWN_METHOD_WARN_COUNT.load(Ordering::Relaxed);
        assert_eq!(count, 2);
    }
}
