//! Environment detection utilities for the ColdVox project.
//!
//! This module centralizes logic for detecting the operating environment,
//! such as the display server protocol (Wayland/X11) and whether the
//! application is running in a CI or development context.

use std::env;
use tracing::debug;

/// Represents the display server protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayProtocol {
    Wayland,
    X11,
    Unknown,
}

/// Detects the current display server protocol.
///
/// It checks for Wayland first, then X11, falling back to Unknown.
pub fn detect_display_protocol() -> DisplayProtocol {
    if is_wayland() {
        DisplayProtocol::Wayland
    } else if is_x11() {
        DisplayProtocol::X11
    } else {
        DisplayProtocol::Unknown
    }
}

/// Checks if the current environment is a Wayland session.
pub fn is_wayland() -> bool {
    env::var("WAYLAND_DISPLAY").is_ok() || env::var("XDG_SESSION_TYPE").map_or(false, |s| s == "wayland")
}

/// Checks if the current environment is an X11 session.
pub fn is_x11() -> bool {
    env::var("DISPLAY").is_ok() || env::var("XDG_SESSION_TYPE").map_or(false, |s| s == "x11")
}

/// Checks if the application is running in a CI (Continuous Integration) environment.
pub fn is_ci() -> bool {
    env::var("CI").is_ok()
        || env::var("CONTINUOUS_INTEGRATION").is_ok()
        || env::var("GITHUB_ACTIONS").is_ok()
        || env::var("GITLAB_CI").is_ok()
        || env::var("TRAVIS").is_ok()
        || env::var("CIRCLECI").is_ok()
        || env::var("JENKINS_URL").is_ok()
        || env::var("BUILDKITE").is_ok()
}

/// Checks if the application is running in a development environment.
pub fn is_dev() -> bool {
    // This is a simple heuristic. In a real application, this might be more complex.
    cfg!(debug_assertions)
}
