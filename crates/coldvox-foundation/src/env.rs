/// Comprehensive environment detection result
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentInfo {
    /// Detected display protocol (Wayland, X11, or Unknown)
    pub display_protocol: DisplayProtocol,
    /// Detected desktop environment
    pub desktop_environment: DesktopEnvironment,
    /// Detected environment type (CI, Development, or Production)
    pub environment: Environment,
    /// Whether a display server is available
    pub has_display: bool,
    /// Whether this is running on Wayland
    pub is_wayland: bool,
    /// Whether this is running on X11
    pub is_x11: bool,
    /// Whether this is running on XWayland (X11 on Wayland)
    pub is_xwayland: bool,
    /// Whether this is running in a CI environment
    pub is_ci: bool,
    /// Whether this is running in a development environment
    pub is_development: bool,
}

impl EnvironmentInfo {
    /// Create a new EnvironmentInfo with all detection results
    pub fn new(
        display_protocol: DisplayProtocol,
        desktop_environment: DesktopEnvironment,
        environment: Environment,
        has_display: bool,
    ) -> Self {
        Self {
            display_protocol,
            desktop_environment,
            environment,
            has_display,
            is_wayland: display_protocol.is_wayland(),
            is_x11: display_protocol.is_x11(),
            is_xwayland: display_protocol.is_xwayland(),
            is_ci: matches!(environment, Environment::CI),
            is_development: matches!(environment, Environment::Development),
        }
    }

    /// Check if this is a Wayland environment
    pub fn is_wayland_environment(&self) -> bool {
        self.is_wayland
    }

    /// Check if this is an X11 environment
    pub fn is_x11_environment(&self) -> bool {
        self.is_x11
    }

    /// Check if this is an XWayland environment
    pub fn is_xwayland_environment(&self) -> bool {
        self.is_xwayland
    }

    /// Check if this is a CI environment
    pub fn is_ci_environment(&self) -> bool {
        self.is_ci
    }

    /// Check if this is a development environment
    pub fn is_development_environment(&self) -> bool {
        self.is_development
    }

    /// Check if this is a production environment
    pub fn is_production_environment(&self) -> bool {
        matches!(self.environment, Environment::Production)
    }

    /// Get a human-readable summary of the environment
    pub fn summary(&self) -> String {
        format!(
            "Environment: {} | Display: {} | Desktop: {} | Has Display: {}",
            match self.environment {
                Environment::CI => "CI",
                Environment::Development => "Development",
                Environment::Production => "Production",
            },
            match self.display_protocol {
                DisplayProtocol::Wayland => "Wayland",
                DisplayProtocol::X11 =>
                    if self.is_xwayland {
                        "XWayland"
                    } else {
                        "X11"
                    },
                DisplayProtocol::Unknown => "Unknown",
            },
            self.desktop_environment,
            if self.has_display { "Yes" } else { "No" }
        )
    }
}

impl std::fmt::Display for EnvironmentInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.summary())
    }
}

/// Environment detection utilities for ColdVox
//
/// This module provides centralized environment detection functionality,
/// including display protocol detection (Wayland, X11), desktop environment
/// identification, and CI/development environment detection across different platforms.
//
/// # Quick Start
//
/// ```rust
/// use coldvox_foundation::env::{detect, EnvironmentInfo};
//
/// let env_info = detect();
/// println!("Environment: {}", env_info);
//
/// if env_info.is_wayland_environment() {
///     println!("Running on Wayland");
// }
//
/// if env_info.is_ci_environment() {
///     println!("Running in CI");
// }
/// ```
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

/// Desktop environment types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopEnvironment {
    /// KDE/KWin on Wayland
    KdeWayland,
    /// KDE/KWin on X11
    KdeX11,
    /// Hyprland (wlroots-based Wayland)
    Hyprland,
    /// GNOME on Wayland
    GnomeWayland,
    /// GNOME on X11
    GnomeX11,
    /// Other Wayland compositor
    OtherWayland,
    /// Other X11 desktop
    OtherX11,
    /// Windows
    Windows,
    /// macOS
    MacOS,
    /// Unknown environment
    Unknown,
}

impl std::fmt::Display for DesktopEnvironment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DesktopEnvironment::KdeWayland => write!(f, "KDE/Wayland"),
            DesktopEnvironment::KdeX11 => write!(f, "KDE/X11"),
            DesktopEnvironment::Hyprland => write!(f, "Hyprland"),
            DesktopEnvironment::GnomeWayland => write!(f, "GNOME/Wayland"),
            DesktopEnvironment::GnomeX11 => write!(f, "GNOME/X11"),
            DesktopEnvironment::OtherWayland => write!(f, "Other/Wayland"),
            DesktopEnvironment::OtherX11 => write!(f, "Other/X11"),
            DesktopEnvironment::Windows => write!(f, "Windows"),
            DesktopEnvironment::MacOS => write!(f, "macOS"),
            DesktopEnvironment::Unknown => write!(f, "Unknown"),
        }
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

/// Detect the current desktop environment
pub fn detect_desktop_environment() -> DesktopEnvironment {
    // Check for Windows
    if cfg!(target_os = "windows") {
        return DesktopEnvironment::Windows;
    }

    // Check for macOS
    if cfg!(target_os = "macos") {
        return DesktopEnvironment::MacOS;
    }

    // Check for Wayland vs X11
    let is_wayland = env::var("XDG_SESSION_TYPE")
        .map(|s| s == "wayland")
        .unwrap_or(false)
        || env::var("WAYLAND_DISPLAY").is_ok();

    let is_x11 = env::var("XDG_SESSION_TYPE")
        .map(|s| s == "x11")
        .unwrap_or(false)
        || env::var("DISPLAY").is_ok();

    // Check for specific desktop environments
    let desktop = env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .to_lowercase();

    let kde = desktop.contains("kde") || env::var("KDE_SESSION_VERSION").is_ok();
    let gnome = desktop.contains("gnome") || desktop.contains("ubuntu");
    let hyprland = env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok();

    if is_wayland {
        if kde {
            DesktopEnvironment::KdeWayland
        } else if hyprland {
            DesktopEnvironment::Hyprland
        } else if gnome {
            DesktopEnvironment::GnomeWayland
        } else {
            DesktopEnvironment::OtherWayland
        }
    } else if is_x11 {
        if kde {
            DesktopEnvironment::KdeX11
        } else if gnome {
            DesktopEnvironment::GnomeX11
        } else {
            DesktopEnvironment::OtherX11
        }
    } else {
        DesktopEnvironment::Unknown
    }
}

/// Check if running on Wayland
pub fn is_wayland_environment() -> bool {
    detect_display_protocol().is_wayland()
}

/// Check if running on X11
pub fn is_x11_environment() -> bool {
    detect_display_protocol().is_x11()
}

/// Check if running on XWayland
pub fn is_xwayland_environment() -> bool {
    detect_display_protocol().is_xwayland()
}

/// Check if a display server is available
pub fn has_display() -> bool {
    env::var("DISPLAY").is_ok() || env::var("WAYLAND_DISPLAY").is_ok()
}

/// Comprehensive environment detection function
///
/// This function provides a single entry point for detecting all aspects of the
/// current environment, consolidating display protocol detection, desktop environment
/// identification, and environment type detection (CI/Development/Production).
///
/// # Detection Strategy
///
/// The function performs detection in the following order:
/// 1. **Display Protocol**: Uses XDG_SESSION_TYPE, WAYLAND_DISPLAY, and DISPLAY variables
/// 2. **Desktop Environment**: Checks XDG_CURRENT_DESKTOP and platform-specific indicators
/// 3. **Environment Type**: Checks for CI indicators first, then development indicators
/// 4. **Display Availability**: Confirms if a display server is accessible
///
/// # Returns
///
/// Returns an `EnvironmentInfo` struct containing comprehensive environment details
/// that can be used to make informed decisions about application behavior.
///
/// # Examples
///
/// ```rust
/// use coldvox_foundation::env::detect;
///
/// let env_info = detect();
///
/// // Check display protocol
/// if env_info.is_wayland_environment() {
///     println!("Running on Wayland");
/// }
///
/// // Check environment type
/// if env_info.is_ci_environment() {
///     println!("Running in CI - using longer timeouts");
/// }
///
/// // Get full summary
/// println!("Environment: {}", env_info);
/// ```
///
/// # Performance
///
/// This function performs multiple environment variable lookups and file system checks.
/// For performance-critical code that only needs specific information, consider using
/// the individual detection functions like `detect_display_protocol()` or `detect_environment()`.
///
/// # Thread Safety
///
/// This function is safe to call from multiple threads as it only reads environment
/// variables and performs read-only operations.
pub fn detect() -> EnvironmentInfo {
    debug!("Starting comprehensive environment detection");

    // Detect display protocol
    let display_protocol = detect_display_protocol();
    debug!("Detected display protocol: {:?}", display_protocol);

    // Detect desktop environment
    let desktop_environment = detect_desktop_environment();
    debug!("Detected desktop environment: {:?}", desktop_environment);

    // Detect environment type
    let environment = detect_environment();
    debug!("Detected environment type: {:?}", environment);

    // Check display availability
    let has_display = has_display();
    debug!("Display available: {}", has_display);

    let env_info = EnvironmentInfo::new(
        display_protocol,
        desktop_environment,
        environment,
        has_display,
    );

    debug!("Environment detection complete: {}", env_info);
    env_info
}

/// Environment types for application behavior configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    /// Continuous Integration environment
    CI,
    /// Development environment
    Development,
    /// Production environment
    Production,
}

/// Check if running in CI environment
///
/// This function checks for common CI environment variables including:
/// - CI (de-facto standard)
/// - CONTINUOUS_INTEGRATION
/// - GITHUB_ACTIONS
/// - GITLAB_CI
/// - TRAVIS
/// - CIRCLECI
/// - JENKINS_URL
/// - BUILDKITE
pub fn is_ci_environment() -> bool {
    env::var("CI").is_ok()
        || env::var("CONTINUOUS_INTEGRATION").is_ok()
        || env::var("GITHUB_ACTIONS").is_ok()
        || env::var("GITLAB_CI").is_ok()
        || env::var("TRAVIS").is_ok()
        || env::var("CIRCLECI").is_ok()
        || env::var("JENKINS_URL").is_ok()
        || env::var("BUILDKITE").is_ok()
}

/// Check if running in development environment
///
/// This function checks for development environment indicators including:
/// - RUST_BACKTRACE (debugging enabled)
/// - DEBUG (debug mode)
/// - DEV (development flag)
/// - .git directory presence (running from git repository)
pub fn is_development_environment() -> bool {
    env::var("RUST_BACKTRACE").is_ok()
        || env::var("DEBUG").is_ok()
        || env::var("DEV").is_ok()
        // Check if running from a git repository
        || std::path::PathBuf::from(".git").exists()
}

/// Detect the current environment
///
/// Detection hierarchy:
/// 1. CI environment (highest priority)
/// 2. Development environment
/// 3. Production environment (default)
///
/// Returns the detected environment type.
pub fn detect_environment() -> Environment {
    // Check for CI environment variables first
    if is_ci_environment() {
        return Environment::CI;
    }

    // Check for development environment indicators
    if is_development_environment() {
        return Environment::Development;
    }

    // Default to production
    Environment::Production
}

/// Get appropriate timeout duration based on environment
///
/// In CI environments, returns a longer timeout to account for potential
/// resource contention. In development, returns a shorter timeout for
/// faster feedback.
///
/// # Arguments
///
/// * `ci_timeout` - Timeout duration for CI environments
/// * `dev_timeout` - Timeout duration for development environments
/// * `prod_timeout` - Timeout duration for production environments
///
/// # Returns
///
/// The appropriate timeout duration based on the current environment.
pub fn get_environment_timeout(
    ci_timeout: std::time::Duration,
    dev_timeout: std::time::Duration,
    prod_timeout: std::time::Duration,
) -> std::time::Duration {
    match detect_environment() {
        Environment::CI => ci_timeout,
        Environment::Development => dev_timeout,
        Environment::Production => prod_timeout,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

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
    #[serial]
    fn test_detect_display_protocol_unknown() {
        // Clear relevant environment variables
        env::remove_var("XDG_SESSION_TYPE");
        env::remove_var("WAYLAND_DISPLAY");
        env::remove_var("DISPLAY");

        let result = detect_display_protocol();
        assert_eq!(result, DisplayProtocol::Unknown);
    }

    #[test]
    #[serial]
    fn test_detect_display_protocol_xdg_session_type() {
        // Clear all display-related environment variables first
        env::remove_var("XDG_SESSION_TYPE");
        env::remove_var("WAYLAND_DISPLAY");
        env::remove_var("DISPLAY");

        // Test Wayland detection via XDG_SESSION_TYPE
        env::set_var("XDG_SESSION_TYPE", "wayland");
        assert_eq!(detect_display_protocol(), DisplayProtocol::Wayland);

        // Clean up before next test
        env::remove_var("XDG_SESSION_TYPE");
        env::remove_var("WAYLAND_DISPLAY");
        env::remove_var("DISPLAY");

        // Test X11 detection via XDG_SESSION_TYPE
        env::set_var("XDG_SESSION_TYPE", "x11");
        assert_eq!(detect_display_protocol(), DisplayProtocol::X11);

        // Clean up before next test
        env::remove_var("XDG_SESSION_TYPE");
        env::remove_var("WAYLAND_DISPLAY");
        env::remove_var("DISPLAY");

        // Test unknown session type
        env::set_var("XDG_SESSION_TYPE", "unknown");
        assert_eq!(detect_display_protocol(), DisplayProtocol::Unknown);

        // Final cleanup
        env::remove_var("XDG_SESSION_TYPE");
        env::remove_var("WAYLAND_DISPLAY");
        env::remove_var("DISPLAY");
    }

    #[test]
    #[serial]
    fn test_detect_display_protocol_wayland_display() {
        // Clear all display-related environment variables first
        env::remove_var("XDG_SESSION_TYPE");
        env::remove_var("WAYLAND_DISPLAY");
        env::remove_var("DISPLAY");

        // Test Wayland detection via WAYLAND_DISPLAY
        env::set_var("WAYLAND_DISPLAY", "wayland-0");
        assert_eq!(detect_display_protocol(), DisplayProtocol::Wayland);

        // Clean up
        env::remove_var("XDG_SESSION_TYPE");
        env::remove_var("WAYLAND_DISPLAY");
        env::remove_var("DISPLAY");
    }

    #[test]
    #[serial]
    fn test_detect_display_protocol_display() {
        // Clear all display-related environment variables first
        env::remove_var("XDG_SESSION_TYPE");
        env::remove_var("WAYLAND_DISPLAY");
        env::remove_var("DISPLAY");

        // Test X11 detection via DISPLAY
        env::set_var("DISPLAY", ":0");
        assert_eq!(detect_display_protocol(), DisplayProtocol::X11);

        // Clean up
        env::remove_var("XDG_SESSION_TYPE");
        env::remove_var("WAYLAND_DISPLAY");
        env::remove_var("DISPLAY");
    }

    #[test]
    fn test_desktop_environment_display() {
        let env = detect_desktop_environment();
        // Just ensure it doesn't panic
        println!("Detected environment: {}", env);
    }

    #[test]
    #[serial]
    fn test_has_display() {
        // Test that the function doesn't panic
        let _ = has_display();
    }

    #[test]
    #[serial]
    fn test_environment_helpers() {
        let _ = is_wayland_environment();
        let _ = is_x11_environment();
        let _ = is_xwayland_environment();
    }

    #[test]
    #[serial]
    fn test_is_ci_environment() {
        // Clear CI-related environment variables first
        for var in [
            "CI",
            "CONTINUOUS_INTEGRATION",
            "GITHUB_ACTIONS",
            "GITLAB_CI",
            "TRAVIS",
            "CIRCLECI",
            "JENKINS_URL",
            "BUILDKITE",
        ] {
            env::remove_var(var);
        }

        // Test CI detection
        env::set_var("CI", "true");
        assert!(is_ci_environment());
        env::remove_var("CI");

        // Test GitHub Actions detection
        env::set_var("GITHUB_ACTIONS", "true");
        assert!(is_ci_environment());
        env::remove_var("GITHUB_ACTIONS");

        // Test no CI indicators
        assert!(!is_ci_environment());
    }

    #[test]
    #[serial]
    fn test_is_development_environment() {
        // Clear development-related environment variables first
        for var in ["RUST_BACKTRACE", "DEBUG", "DEV"] {
            env::remove_var(var);
        }

        // Test DEBUG detection
        env::set_var("DEBUG", "1");
        assert!(is_development_environment());
        env::remove_var("DEBUG");

        // Test RUST_BACKTRACE detection
        env::set_var("RUST_BACKTRACE", "1");
        assert!(is_development_environment());
        env::remove_var("RUST_BACKTRACE");

        // Test no development indicators
        assert!(!is_development_environment());
    }

    #[test]
    #[serial]
    fn test_detect_environment() {
        // Clear all environment variables first
        for var in [
            "CI",
            "CONTINUOUS_INTEGRATION",
            "GITHUB_ACTIONS",
            "GITLAB_CI",
            "TRAVIS",
            "CIRCLECI",
            "JENKINS_URL",
            "BUILDKITE",
            "RUST_BACKTRACE",
            "DEBUG",
            "DEV",
        ] {
            env::remove_var(var);
        }

        // Test CI detection
        env::set_var("CI", "true");
        assert_eq!(detect_environment(), Environment::CI);
        env::remove_var("CI");

        // Test development detection
        env::set_var("DEBUG", "1");
        assert_eq!(detect_environment(), Environment::Development);
        env::remove_var("DEBUG");

        // Test default to production
        assert_eq!(detect_environment(), Environment::Production);
    }

    #[test]
    #[serial]
    fn test_get_environment_timeout() {
        use std::time::Duration;

        let ci_timeout = Duration::from_millis(2000);
        let dev_timeout = Duration::from_millis(500);
        let prod_timeout = Duration::from_millis(1000);

        // Test CI timeout
        env::set_var("CI", "true");
        assert_eq!(
            get_environment_timeout(ci_timeout, dev_timeout, prod_timeout),
            ci_timeout
        );
        env::remove_var("CI");

        // Test development timeout
        env::set_var("DEBUG", "1");
        assert_eq!(
            get_environment_timeout(ci_timeout, dev_timeout, prod_timeout),
            dev_timeout
        );
        env::remove_var("DEBUG");

        // Test production timeout
        assert_eq!(
            get_environment_timeout(ci_timeout, dev_timeout, prod_timeout),
            prod_timeout
        );
    }

    // Tests for EnvironmentInfo struct
    #[test]
    fn test_environment_info_new() {
        let env_info = EnvironmentInfo::new(
            DisplayProtocol::Wayland,
            DesktopEnvironment::GnomeWayland,
            Environment::Development,
            true,
        );

        assert_eq!(env_info.display_protocol, DisplayProtocol::Wayland);
        assert_eq!(
            env_info.desktop_environment,
            DesktopEnvironment::GnomeWayland
        );
        assert_eq!(env_info.environment, Environment::Development);
        assert!(env_info.has_display);
        assert!(env_info.is_wayland);
        assert!(!env_info.is_x11);
        assert!(!env_info.is_xwayland);
        assert!(!env_info.is_ci);
        assert!(env_info.is_development);
    }

    #[test]
    fn test_environment_info_helper_methods() {
        let env_info = EnvironmentInfo::new(
            DisplayProtocol::X11,
            DesktopEnvironment::KdeX11,
            Environment::CI,
            true,
        );

        assert!(env_info.is_x11_environment());
        assert!(!env_info.is_wayland_environment());
        assert!(env_info.is_ci_environment());
        assert!(!env_info.is_development_environment());
        assert!(!env_info.is_production_environment());
    }

    #[test]
    fn test_environment_info_xwayland_detection() {
        // Create X11 environment with Wayland indicators (XWayland)
        let env_info = EnvironmentInfo::new(
            DisplayProtocol::X11,
            DesktopEnvironment::OtherX11,
            Environment::Production,
            true,
        );

        // Manually set the is_xwayland flag for testing
        let env_info = EnvironmentInfo {
            is_xwayland: true,
            ..env_info
        };

        assert!(env_info.is_xwayland_environment());
        assert!(env_info.is_x11_environment());
        assert!(!env_info.is_wayland_environment());
    }

    #[test]
    fn test_environment_info_summary() {
        let env_info = EnvironmentInfo::new(
            DisplayProtocol::Wayland,
            DesktopEnvironment::GnomeWayland,
            Environment::Development,
            true,
        );

        let summary = env_info.summary();
        assert!(summary.contains("Development"));
        assert!(summary.contains("Wayland"));
        assert!(summary.contains("GNOME"));
        assert!(summary.contains("Yes"));
    }

    #[test]
    fn test_environment_info_display() {
        let env_info = EnvironmentInfo::new(
            DisplayProtocol::X11,
            DesktopEnvironment::KdeX11,
            Environment::Production,
            false,
        );

        let display_string = format!("{}", env_info);
        assert!(display_string.contains("Production"));
        assert!(display_string.contains("X11"));
        assert!(display_string.contains("KDE"));
        assert!(display_string.contains("No"));
    }

    // Tests for detect() function
    #[test]
    #[serial]
    fn test_detect_function_basic() {
        // Clear all environment variables
        for var in [
            "XDG_SESSION_TYPE",
            "WAYLAND_DISPLAY",
            "DISPLAY",
            "XDG_CURRENT_DESKTOP",
            "CI",
            "CONTINUOUS_INTEGRATION",
            "GITHUB_ACTIONS",
            "GITLAB_CI",
            "TRAVIS",
            "CIRCLECI",
            "JENKINS_URL",
            "BUILDKITE",
            "RUST_BACKTRACE",
            "DEBUG",
            "DEV",
        ] {
            env::remove_var(var);
        }

        let env_info = detect();

        // Should detect some environment (even if unknown)
        assert!(
            env_info.display_protocol == DisplayProtocol::Unknown
                || env_info.display_protocol == DisplayProtocol::Wayland
                || env_info.display_protocol == DisplayProtocol::X11
        );
        // Desktop environment can be unknown when no display is available
        assert!(
            env_info.desktop_environment == DesktopEnvironment::Unknown || env_info.has_display
        );
        assert!(env_info.environment == Environment::Production); // Default when no indicators
    }

    #[test]
    #[serial]
    fn test_detect_wayland_environment() {
        // Clear environment variables
        for var in [
            "XDG_SESSION_TYPE",
            "WAYLAND_DISPLAY",
            "DISPLAY",
            "CI",
            "DEBUG",
        ] {
            env::remove_var(var);
        }

        // Set Wayland environment
        env::set_var("XDG_SESSION_TYPE", "wayland");
        env::set_var("WAYLAND_DISPLAY", "wayland-0");
        env::set_var("XDG_CURRENT_DESKTOP", "GNOME");

        let env_info = detect();

        assert_eq!(env_info.display_protocol, DisplayProtocol::Wayland);
        assert!(env_info.is_wayland_environment());
        assert!(!env_info.is_x11_environment());
        assert!(env_info.has_display);

        // Cleanup
        env::remove_var("XDG_SESSION_TYPE");
        env::remove_var("WAYLAND_DISPLAY");
        env::remove_var("XDG_CURRENT_DESKTOP");
    }

    #[test]
    #[serial]
    fn test_detect_x11_environment() {
        // Clear environment variables
        for var in [
            "XDG_SESSION_TYPE",
            "WAYLAND_DISPLAY",
            "DISPLAY",
            "CI",
            "DEBUG",
        ] {
            env::remove_var(var);
        }

        // Set X11 environment
        env::set_var("XDG_SESSION_TYPE", "x11");
        env::set_var("DISPLAY", ":0");
        env::set_var("XDG_CURRENT_DESKTOP", "KDE");

        let env_info = detect();

        assert_eq!(env_info.display_protocol, DisplayProtocol::X11);
        assert!(env_info.is_x11_environment());
        assert!(!env_info.is_wayland_environment());
        assert!(env_info.has_display);

        // Cleanup
        env::remove_var("XDG_SESSION_TYPE");
        env::remove_var("DISPLAY");
        env::remove_var("XDG_CURRENT_DESKTOP");
    }

    #[test]
    #[serial]
    fn test_detect_ci_environment() {
        // Clear environment variables
        for var in ["CI", "GITHUB_ACTIONS", "DEBUG", "XDG_SESSION_TYPE"] {
            env::remove_var(var);
        }

        // Set CI environment
        env::set_var("CI", "true");
        env::set_var("GITHUB_ACTIONS", "true");

        let env_info = detect();

        assert_eq!(env_info.environment, Environment::CI);
        assert!(env_info.is_ci_environment());
        assert!(!env_info.is_development_environment());
        assert!(!env_info.is_production_environment());

        // Cleanup
        env::remove_var("CI");
        env::remove_var("GITHUB_ACTIONS");
    }

    #[test]
    #[serial]
    fn test_detect_development_environment() {
        // Clear environment variables
        for var in ["CI", "DEBUG", "RUST_BACKTRACE", "XDG_SESSION_TYPE"] {
            env::remove_var(var);
        }

        // Set development environment
        env::set_var("DEBUG", "1");
        env::set_var("RUST_BACKTRACE", "1");

        let env_info = detect();

        assert_eq!(env_info.environment, Environment::Development);
        assert!(env_info.is_development_environment());
        assert!(!env_info.is_ci_environment());
        assert!(!env_info.is_production_environment());

        // Cleanup
        env::remove_var("DEBUG");
        env::remove_var("RUST_BACKTRACE");
    }

    #[test]
    #[serial]
    fn test_detect_priority_ci_over_development() {
        // Clear environment variables
        for var in ["CI", "DEBUG", "XDG_SESSION_TYPE"] {
            env::remove_var(var);
        }

        // Set both CI and development indicators
        env::set_var("CI", "true");
        env::set_var("DEBUG", "1");

        let env_info = detect();

        // CI should take priority
        assert_eq!(env_info.environment, Environment::CI);
        assert!(env_info.is_ci_environment());
        assert!(!env_info.is_development_environment());

        // Cleanup
        env::remove_var("CI");
        env::remove_var("DEBUG");
    }

    #[test]
    #[serial]
    fn test_detect_no_display_environment() {
        // Clear all display-related environment variables
        for var in [
            "XDG_SESSION_TYPE",
            "WAYLAND_DISPLAY",
            "DISPLAY",
            "CI",
            "DEBUG",
        ] {
            env::remove_var(var);
        }

        let env_info = detect();

        assert!(!env_info.has_display);
        assert_eq!(env_info.display_protocol, DisplayProtocol::Unknown);
    }

    #[test]
    #[serial]
    fn test_detect_consistency_with_individual_functions() {
        // Set up a known environment
        for var in [
            "XDG_SESSION_TYPE",
            "WAYLAND_DISPLAY",
            "DISPLAY",
            "CI",
            "DEBUG",
        ] {
            env::remove_var(var);
        }

        env::set_var("XDG_SESSION_TYPE", "wayland");
        env::set_var("WAYLAND_DISPLAY", "wayland-0");
        env::set_var("CI", "true");

        let env_info = detect();

        // Verify consistency with individual detection functions
        assert_eq!(env_info.display_protocol, detect_display_protocol());
        assert_eq!(env_info.desktop_environment, detect_desktop_environment());
        assert_eq!(env_info.environment, detect_environment());
        assert_eq!(env_info.has_display, has_display());

        // Verify helper methods match the detected values
        assert_eq!(env_info.is_wayland_environment(), is_wayland_environment());
        assert_eq!(env_info.is_x11_environment(), is_x11_environment());
        assert_eq!(env_info.is_ci_environment(), is_ci_environment());

        // Cleanup
        env::remove_var("XDG_SESSION_TYPE");
        env::remove_var("WAYLAND_DISPLAY");
        env::remove_var("CI");
    }
}
