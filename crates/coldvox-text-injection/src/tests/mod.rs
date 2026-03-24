//! Test modules for coldvox-text-injection

pub mod real_injection;
#[cfg(feature = "real-injection-tests")]
pub mod test_harness;
pub mod test_utils;
#[cfg(all(unix, feature = "ydotool"))]
pub mod test_ydotool_injector;
pub mod wl_copy_basic_test;
pub mod wl_copy_simple_test;
pub mod wl_copy_stdin_test;
