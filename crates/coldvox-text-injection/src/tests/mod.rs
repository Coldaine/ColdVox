//! Test modules for coldvox-text-injection

// Integration tests requiring a live display server and user interaction.
// Gated by the `real-injection-tests` feature.
#[cfg(feature = "real-injection-tests")]
pub mod real_injection;

// Mock harness for headless unit testing of injection logic.
pub mod mock_harness;

// Unit tests using the mock harness.
pub mod mock_injection;

// Shared test utilities and application runners.
pub mod test_harness;
pub mod test_utils;

// Specific backend tests (wl-copy, etc.)
pub mod wl_copy_basic_test;
pub mod wl_copy_simple_test;
pub mod wl_copy_stdin_test;
