//! # Clock Abstraction for Test Determinism
//!
//! This module provides a Clock trait that can be implemented for both real-time
//! and virtual-time execution, enabling deterministic testing of time-dependent code.

use std::time::{Duration, Instant};

/// Clock trait for time abstraction
pub trait Clock: Send + Sync {
    /// Get the current time
    fn now(&self) -> Instant;

    /// Sleep for the specified duration
    fn sleep(&self, duration: Duration);
}

/// Real-time clock implementation
pub struct RealClock;

impl Default for RealClock {
    fn default() -> Self {
        Self::new()
    }
}

impl RealClock {
    pub fn new() -> Self {
        Self
    }
}

impl Clock for RealClock {
    fn now(&self) -> Instant {
        Instant::now()
    }

    fn sleep(&self, duration: Duration) {
        std::thread::sleep(duration);
    }
}

/// Virtual clock for deterministic testing
pub struct TestClock {
    current_time: std::sync::Mutex<Instant>,
}

impl Default for TestClock {
    fn default() -> Self {
        Self::new()
    }
}

impl TestClock {
    pub fn new() -> Self {
        Self {
            current_time: std::sync::Mutex::new(Instant::now()),
        }
    }

    pub fn new_with_start_time(start_time: Instant) -> Self {
        Self {
            current_time: std::sync::Mutex::new(start_time),
        }
    }

    /// Advance the virtual clock by the specified duration
    pub fn advance(&self, duration: Duration) {
        let mut time = self.current_time.lock().unwrap();
        *time += duration;
    }

    /// Set the virtual clock to a specific time
    pub fn set_time(&self, time: Instant) {
        let mut current = self.current_time.lock().unwrap();
        *current = time;
    }
}

impl Clock for TestClock {
    fn now(&self) -> Instant {
        *self.current_time.lock().unwrap()
    }

    fn sleep(&self, duration: Duration) {
        // In virtual time, sleep just advances the clock
        self.advance(duration);
        // Yield to allow other tasks to run (though this is synchronous)
        std::thread::yield_now();
    }
}

/// Thread-safe clock that can be shared across threads
pub type SharedClock = std::sync::Arc<dyn Clock + Send + Sync>;

/// Create a real-time clock
pub fn real_clock() -> SharedClock {
    std::sync::Arc::new(RealClock::new())
}

/// Create a test clock
pub fn test_clock() -> SharedClock {
    std::sync::Arc::new(TestClock::new())
}

/// Create a test clock with specific start time
pub fn test_clock_with_start(start_time: Instant) -> SharedClock {
    std::sync::Arc::new(TestClock::new_with_start_time(start_time))
}
