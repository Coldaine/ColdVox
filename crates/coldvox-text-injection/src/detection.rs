//! Unified display protocol detection module
//!
//! This module provides a centralized way to detect the display protocol
//! (Wayland, X11, or XWayland) that the system is using. It prioritizes
//! XDG_SESSION_TYPE over individual display variables for accurate detection.

use std::env;
use tracing::{debug, warn};

/// Display protocol types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayProtocol {
    /// Native Wayland session
    Wayland,
    /// X11 session (including XWayland)
    X11,
    /// Unknown or unsupported protocol
    Unknown,
}

impl DisplayProtocol {
    /// Check if this is a Wayland protocol
    pub fn is_wayland(&self) -> bool {
        matches!(self, DisplayProtocol::Wayland)
    }

    /// Check if this is an X11 protocol
    pub fn is_x11(&self) -> bool {
        matches!(self, DisplayProtocol::X11)
    }

    /// Check if this is XWayland (X11 running on Wayland)
    pub fn is_xwayland(&self) -> bool {
        if !self.is_x11() {
            return false;
        }

        // Check for XWayland indicators
        env::var("WAYLAND_DISPLAY").is_ok()
            || env::var("XDG_SESSION_TYPE").as_deref() == Ok("wayland")
    }
}

/// Detect the current display protocol
///
/// Detection hierarchy:
/// 1. XDG_SESSION_TYPE environment variable (most reliable)
/// 2. WAYLAND_DISPLAY environment variable
/// 3. DISPLAY environment variable with XWayland checks
///
/// Returns DisplayProtocol::Unknown if no protocol can be determined.
pub fn detect_display_protocol() -> DisplayProtocol {
    // 1. Check XDG_SESSION_TYPE first (most authoritative)
    if let Ok(session_type) = env::var("XDG_SESSION_TYPE") {
        match session_type.to_lowercase().as_str() {
            "wayland" => {
                debug!("Detected Wayland via XDG_SESSION_TYPE");
                return DisplayProtocol::Wayland;
            }
            "x11" => {
                debug!("Detected X11 via XDG_SESSION_TYPE");
                return DisplayProtocol::X11;
            }
            _ => {
                warn!("Unknown XDG_SESSION_TYPE: {}", session_type);
            }
        }
    }

    // 2. Check WAYLAND_DISPLAY
    if env::var("WAYLAND_DISPLAY").is_ok() {
        debug!("Detected Wayland via WAYLAND_DISPLAY");
        return DisplayProtocol::Wayland;
    }

    // 3. Check DISPLAY (X11 or XWayland)
    if env::var("DISPLAY").is_ok() {
        debug!("Detected X11 via DISPLAY");
        return DisplayProtocol::X11;
    }

    warn!("Could not detect display protocol from environment variables");
    DisplayProtocol::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_protocol_is_wayland() {
        assert!(DisplayProtocol::Wayland.is_wayland());
        assert!(!DisplayProtocol::X11.is_wayland());
        assert!(!DisplayProtocol::Unknown.is_wayland());
    }

    #[test]
    fn test_display_protocol_is_x11() {
        assert!(!DisplayProtocol::Wayland.is_x11());
        assert!(DisplayProtocol::X11.is_x11());
        assert!(!DisplayProtocol::Unknown.is_x11());
    }

    #[test]
    fn test_display_protocol_is_xwayland() {
        // Test without environment variables
        assert!(!DisplayProtocol::Wayland.is_xwayland());
        assert!(!DisplayProtocol::X11.is_xwayland()); // No Wayland indicators
        assert!(!DisplayProtocol::Unknown.is_xwayland());
    }

    #[test]
    fn test_detect_display_protocol_unknown() {
        // Clear relevant environment variables
        env::remove_var("XDG_SESSION_TYPE");
        env::remove_var("WAYLAND_DISPLAY");
        env::remove_var("DISPLAY");

        assert_eq!(detect_display_protocol(), DisplayProtocol::Unknown);
    }

    #[test]
    fn test_detect_display_protocol_xdg_session_type() {
        env::set_var("XDG_SESSION_TYPE", "wayland");
        env::remove_var("WAYLAND_DISPLAY");
        env::remove_var("DISPLAY");

        assert_eq!(detect_display_protocol(), DisplayProtocol::Wayland);

        env::set_var("XDG_SESSION_TYPE", "x11");
        assert_eq!(detect_display_protocol(), DisplayProtocol::X11);

        env::set_var("XDG_SESSION_TYPE", "unknown");
        assert_eq!(detect_display_protocol(), DisplayProtocol::Unknown);

        env::remove_var("XDG_SESSION_TYPE");
    }

    #[test]
    fn test_detect_display_protocol_wayland_display() {
        env::remove_var("XDG_SESSION_TYPE");
        env::set_var("WAYLAND_DISPLAY", "wayland-0");
        env::remove_var("DISPLAY");

        assert_eq!(detect_display_protocol(), DisplayProtocol::Wayland);

        env::remove_var("WAYLAND_DISPLAY");
    }

    #[test]
    fn test_detect_display_protocol_display() {
        env::remove_var("XDG_SESSION_TYPE");
        env::remove_var("WAYLAND_DISPLAY");
        env::set_var("DISPLAY", ":0");

        assert_eq!(detect_display_protocol(), DisplayProtocol::X11);

        env::remove_var("DISPLAY");
    }
}
