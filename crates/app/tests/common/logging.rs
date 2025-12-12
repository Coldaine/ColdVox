//! Shared test logging infrastructure for ColdVox tests.
//!
//! This module provides standardized file-based logging for all tests,
//! ensuring we always have a persistent log trail for debugging failures.
//!
//! ## Usage
//!
//! ```rust,no_run
//! use crate::common::logging::init_test_logging;
//!
//! #[tokio::test]
//! async fn my_test() {
//!     let _guard = init_test_logging("my_test");
//!     // ... test code ...
//! }
//! ```
//!
//! Logs are written to `target/test-logs/<test_name>.log` with debug level by default.
//! Set `COLDVOX_TEST_LOG_LEVEL` to override (e.g., `trace`, `info`).

#![allow(dead_code)] // Utility functions may not be used in all test binaries

use std::path::PathBuf;
use std::sync::Once;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

static INIT: Once = Once::new();

/// Directory name for test logs (will be created under workspace root).
const TEST_LOG_DIR_NAME: &str = "test-logs";

/// Get the workspace root directory.
///
/// Uses CARGO_MANIFEST_DIR and walks up to find workspace root.
fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR points to the crate directory during tests
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Walk up from crates/app to workspace root
    manifest_dir
        .parent() // -> crates/
        .and_then(|p| p.parent()) // -> workspace root
        .map(|p| p.to_path_buf())
        .unwrap_or(manifest_dir)
}

/// Get the full path to the test logs directory.
pub fn test_log_dir() -> PathBuf {
    workspace_root().join("target").join(TEST_LOG_DIR_NAME)
}

/// Initialize test logging with file output.
///
/// Returns a guard that must be held for the duration of the test.
/// Logs are written to `target/test-logs/<test_name>.log`.
///
/// The log level defaults to `debug` but can be overridden via:
/// - `COLDVOX_TEST_LOG_LEVEL` environment variable
/// - `RUST_LOG` environment variable (if `COLDVOX_TEST_LOG_LEVEL` is not set)
///
/// # Arguments
/// * `test_name` - Name of the test, used for the log filename
///
/// # Returns
/// A `WorkerGuard` that ensures logs are flushed on drop. **You must keep this alive!**
pub fn init_test_logging(test_name: &str) -> WorkerGuard {
    // Get workspace-relative log directory
    let log_dir = test_log_dir();

    // Ensure log directory exists
    let _ = std::fs::create_dir_all(&log_dir);

    // Determine log level: COLDVOX_TEST_LOG_LEVEL > RUST_LOG > default (debug)
    let log_level = std::env::var("COLDVOX_TEST_LOG_LEVEL")
        .or_else(|_| std::env::var("RUST_LOG"))
        .unwrap_or_else(|_| "debug".to_string());

    // Build filter with sensible defaults for test debugging
    let filter = EnvFilter::try_new(&log_level).unwrap_or_else(|_| {
        EnvFilter::new("debug")
            .add_directive("coldvox_app=debug".parse().unwrap())
            .add_directive("coldvox_stt=debug".parse().unwrap())
            .add_directive("coldvox_audio=debug".parse().unwrap())
            .add_directive("coldvox_vad=debug".parse().unwrap())
            .add_directive("coldvox_text_injection=debug".parse().unwrap())
    });

    // Create file appender for this test
    let log_filename = format!("{}.log", test_name);
    let file_appender = tracing_appender::rolling::never(&log_dir, &log_filename);
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // Try to initialize the subscriber. If it's already initialized (by another test),
    // that's fine - we'll still get our file output.
    let _ = tracing_subscriber::registry()
        .with(filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_test_writer()
                .with_thread_ids(true),
        )
        .try_init();

    // Log test start marker
    tracing::info!("========================================");
    tracing::info!("TEST START: {}", test_name);
    tracing::info!("Log file: {}/{}", log_dir.display(), log_filename);
    tracing::info!("========================================");

    guard
}

/// Initialize test logging once for the entire test suite.
///
/// This is useful for integration tests that run multiple tests in sequence.
/// Unlike `init_test_logging`, this only initializes once per process.
pub fn init_test_logging_once(suite_name: &str) -> Option<WorkerGuard> {
    let mut guard: Option<WorkerGuard> = None;

    INIT.call_once(|| {
        guard = Some(init_test_logging(suite_name));
    });

    guard
}

/// Get the path to a test's log file.
pub fn test_log_path(test_name: &str) -> PathBuf {
    test_log_dir().join(format!("{}.log", test_name))
}

/// Append a marker to the current log indicating a test phase.
///
/// Useful for marking sections in long-running tests.
pub fn log_phase(phase: &str) {
    tracing::info!("--- PHASE: {} ---", phase);
}

/// Log test completion with result summary.
pub fn log_test_end(test_name: &str, success: bool) {
    tracing::info!("========================================");
    tracing::info!(
        "TEST END: {} - {}",
        test_name,
        if success { "PASSED" } else { "FAILED" }
    );
    tracing::info!("========================================");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_path_generation() {
        let path = test_log_path("my_test");
        assert!(path.to_string_lossy().contains("test-logs"));
        assert!(path.to_string_lossy().ends_with("my_test.log"));
    }
}
