//! Test modules for coldvox-text-injection

// pub mod real_injection;
pub mod test_harness;
pub mod test_utils;
pub mod wl_copy_basic_test;
pub mod wl_copy_simple_test;
pub mod wl_copy_stdin_test;

#[cfg(all(test, feature = "atspi"))]
mod focus_backend_test;
