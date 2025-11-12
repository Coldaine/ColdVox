//! Test modules for coldvox-text-injection

// pub mod real_injection;
pub mod test_utils;
pub mod wl_copy_basic_test;
pub mod wl_copy_simple_test;
pub mod wl_copy_stdin_test;
#[cfg(all(unix, not(target_os = "macos")))]
pub mod xclip_roundtrip_test;
