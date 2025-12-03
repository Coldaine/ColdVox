// The `real-injection-tests` feature enables tests that interact with a live desktop environment.
// These tests are disabled by default as they require a graphical session (X11 or Wayland).
// To run these tests: `cargo test -p coldvox-text-injection --features real-injection-tests`
#[cfg(feature = "real-injection-tests")]
pub mod real_injection;

// Shared test utilities for both unit and real injection tests.
pub mod test_harness;
pub mod test_utils;

// Unit tests for wl-clipboard-rs integration
#[cfg(feature = "wl_clipboard")]
mod wl_copy_basic_test;
#[cfg(feature = "wl_clipboard")]
mod wl_copy_simple_test;
#[cfg(feature = "wl_clipboard")]
mod wl_copy_stdin_test;
