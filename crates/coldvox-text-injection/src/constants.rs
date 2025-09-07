//! # Timeout and Configuration Constants
//!
//! This module centralizes all configurable constants, especially timeouts,
//! for the text injection system. This makes it easy to tune performance
//! and behavior from a single location.

use std::time::Duration;

// --- Global Timeouts ---
/// The total budget for a single `inject_with_fail_fast` call.
pub const GLOBAL_INJECTION_BUDGET_MS: u64 = 1200;

/// The maximum time to wait for a test body to complete before failing.
pub const TEST_WATCHDOG_TIMEOUT_SECS: u64 = 10;

// --- Per-Backend Timeouts ---
/// The soft timeout for a single backend attempt, including retries.
pub const PER_BACKEND_SOFT_TIMEOUT_MS: u64 = 600;

// --- Phase Timeouts ---
/// Timeout for acquiring focus on a target window.
pub const FOCUS_ACQUISITION_TIMEOUT_MS: u64 = 250;

/// Timeout for a single AT-SPI method call.
pub const ATSPI_METHOD_TIMEOUT_MS: u64 = 500;

/// Timeout for a roundtrip clipboard operation (get or set).
pub const CLIPBOARD_ROUNDTRIP_TIMEOUT_MS: u64 = 400;

// --- Subprocess Timeouts ---
/// Timeout for quick-probing a subprocess for availability (e.g., `wl-paste --version`).
/// This must be short to ensure the environment probe is fast.
pub const SUBPROCESS_PROBE_TIMEOUT_MS: u64 = 150;

/// Timeout for running a clipboard tool like `wl-paste` or `xclip`.
/// This needs to be strict to avoid hangs.
pub const CLIPBOARD_TOOL_TIMEOUT_MS: u64 = 180;

// --- Polling Intervals ---
/// The interval for polling for readiness (e.g., waiting for a window to get focus).
pub const READINESS_POLL_INTERVAL_MS: u64 = 30;

// --- Convenience Functions ---
pub fn global_injection_budget() -> Duration {
    Duration::from_millis(GLOBAL_INJECTION_BUDGET_MS)
}

pub fn test_watchdog_timeout() -> Duration {
    Duration::from_secs(TEST_WATCHDOG_TIMEOUT_SECS)
}

pub fn per_backend_soft_timeout() -> Duration {
    Duration::from_millis(PER_BACKEND_SOFT_TIMEOUT_MS)
}
