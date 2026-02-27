#![allow(clippy::test_attr_in_doctest)]
//! Comprehensive test environment detection for ColdVox
//!
//! This module provides sophisticated environment detection capabilities for tests,
//! enabling automatic determination of when tests should run based on:
//! - Display server availability (X11/Wayland)
//! - CI environment detection
//! - Required tool availability
//! - Resource constraints
//! - Backend-specific requirements
//!
//! # Examples
//!
//! ```ignore
//! use coldvox_foundation::test_env::{TestEnvironment, TestRequirements};
//!
//! // Check if current environment supports GUI tests
//! let env = TestEnvironment::detect();
//! if env.can_run_gui_tests() {
//!     // Run GUI-dependent tests
//! }
//!
//! // Check for specific backend availability
//! let requirements = TestRequirements::new()
//!     .requires_wayland()
//!     .requires_command("wl-copy");
//!
//! let result = env.meets_requirements(&requirements);
//! if result.can_run {
//!     // Run Wayland clipboard tests
//! }
//! ```

use std::env;
use std::path::Path;
use std::process::Command;
use std::time::Duration;
use tracing::{debug, info};

use super::env::{detect_display_protocol, detect_environment, DisplayProtocol, Environment};

/// Comprehensive test environment information
#[derive(Debug, Clone)]
pub struct TestEnvironment {
    /// Display protocol detection
    pub display_protocol: DisplayProtocol,
    /// Environment type (CI/Development/Production)
    pub environment_type: Environment,
    /// Whether a display server is available
    pub has_display: bool,
    /// Available commands on the system
    pub available_commands: std::collections::HashSet<String>,
    /// Whether running in CI
    pub is_ci: bool,
    /// System resource constraints
    pub resources: ResourceConstraints,
}

/// System resource constraints that affect test execution
#[derive(Debug, Clone)]
pub struct ResourceConstraints {
    /// Whether the system has limited memory (affects test timeouts)
    pub limited_memory: bool,
    /// Whether the system has limited CPU (affects test parallelism)
    pub limited_cpu: bool,
    /// Whether running in a containerized environment
    pub is_containerized: bool,
}

/// Requirements for running specific tests
#[derive(Debug, Clone, Default)]
pub struct TestRequirements {
    /// Requires a display server (X11 or Wayland)
    pub requires_display: bool,
    /// Specifically requires Wayland
    pub requires_wayland: bool,
    /// Specifically requires X11
    pub requires_x11: bool,
    /// Required commands that must be available
    pub required_commands: Vec<String>,
    /// Required environment variables
    pub required_env_vars: Vec<String>,
    /// Minimum memory requirement in MB
    pub min_memory_mb: Option<u64>,
    /// Whether test requires GUI interaction
    pub requires_gui: bool,
    /// Whether test is timing-sensitive
    pub timing_sensitive: bool,
    /// Whether test requires daemon processes
    pub requires_daemons: bool,
}

impl TestRequirements {
    /// Create new empty requirements
    pub fn new() -> Self {
        Self::default()
    }

    /// Require a display server
    pub fn requires_display(mut self) -> Self {
        self.requires_display = true;
        self
    }

    /// Require Wayland specifically
    pub fn requires_wayland(mut self) -> Self {
        self.requires_wayland = true;
        self.requires_display = true;
        self
    }

    /// Require X11 specifically
    pub fn requires_x11(mut self) -> Self {
        self.requires_x11 = true;
        self.requires_display = true;
        self
    }

    /// Require a specific command to be available
    pub fn requires_command<S: Into<String>>(mut self, command: S) -> Self {
        self.required_commands.push(command.into());
        self
    }

    /// Require multiple commands
    pub fn requires_commands<I, S>(mut self, commands: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for cmd in commands {
            self.required_commands.push(cmd.into());
        }
        self
    }

    /// Require an environment variable to be set
    pub fn requires_env_var<S: Into<String>>(mut self, var: S) -> Self {
        self.required_env_vars.push(var.into());
        self
    }

    /// Require minimum memory in MB
    pub fn min_memory_mb(mut self, mb: u64) -> Self {
        self.min_memory_mb = Some(mb);
        self
    }

    /// Require GUI interaction
    pub fn requires_gui(mut self) -> Self {
        self.requires_gui = true;
        self.requires_display = true;
        self
    }

    /// Mark test as timing-sensitive
    pub fn timing_sensitive(mut self) -> Self {
        self.timing_sensitive = true;
        self
    }

    /// Require daemon processes
    pub fn requires_daemons(mut self) -> Self {
        self.requires_daemons = true;
        self
    }

    /// Check if requirements are met by the current environment
    pub fn check(&self, env: &TestEnvironment) -> TestResult {
        let mut reasons = Vec::new();
        let mut can_run = true;

        // Check display requirements
        if self.requires_display && !env.has_display {
            can_run = false;
            reasons.push("No display server available".to_string());
        }

        if self.requires_wayland && !env.display_protocol.is_wayland() {
            can_run = false;
            reasons.push("Wayland not available".to_string());
        }

        if self.requires_x11 && !env.display_protocol.is_x11() {
            can_run = false;
            reasons.push("X11 not available".to_string());
        }

        // Check command requirements
        for cmd in &self.required_commands {
            if !env.available_commands.contains(cmd) {
                can_run = false;
                reasons.push(format!("Command '{}' not available", cmd));
            }
        }

        // Check environment variable requirements
        for var in &self.required_env_vars {
            if env::var(var).is_err() {
                can_run = false;
                reasons.push(format!("Environment variable '{}' not set", var));
            }
        }

        // Check memory requirements
        if let Some(min_memory) = self.min_memory_mb {
            if env.resources.limited_memory {
                // In CI, we assume limited memory and skip memory-intensive tests
                can_run = false;
                reasons.push(format!("Limited memory (requires {}MB)", min_memory));
            }
        }

        // Check GUI requirements
        if self.requires_gui && env.is_ci {
            // GUI tests in CI need special handling
            if !env.has_display {
                can_run = false;
                reasons.push("GUI test in CI without display".to_string());
            }
        }

        // Check timing sensitivity
        if self.timing_sensitive && env.is_ci {
            // Timing-sensitive tests may need longer timeouts in CI
            info!("Timing-sensitive test detected in CI - will use extended timeouts");
        }

        // Check daemon requirements
        if self.requires_daemons && env.is_ci {
            // Daemon-dependent tests may not work in containerized CI
            if env.resources.is_containerized {
                can_run = false;
                reasons.push("Daemon-dependent test in containerized CI".to_string());
            }
        }

        TestResult {
            can_run,
            reasons,
            suggested_timeout: self.get_suggested_timeout(env),
        }
    }

    /// Get suggested timeout based on requirements and environment
    fn get_suggested_timeout(&self, env: &TestEnvironment) -> Duration {
        let base_timeout = Duration::from_millis(500);

        let mut multiplier = 1.0;

        // Increase timeout for CI
        if env.is_ci {
            multiplier *= 4.0; // 4x longer in CI
        }

        // Increase for timing-sensitive tests
        if self.timing_sensitive {
            multiplier *= 2.0;
        }

        // Increase for GUI tests
        if self.requires_gui {
            multiplier *= 1.5;
        }

        // Increase for daemon-dependent tests
        if self.requires_daemons {
            multiplier *= 2.0;
        }

        Duration::from_millis((base_timeout.as_millis() as f64 * multiplier) as u64)
    }
}

/// Result of checking test requirements
#[derive(Debug, Clone)]
pub struct TestResult {
    /// Whether the test can run
    pub can_run: bool,
    /// Reasons why the test cannot run (if any)
    pub reasons: Vec<String>,
    /// Suggested timeout for the test
    pub suggested_timeout: Duration,
}

impl TestEnvironment {
    /// Detect the current test environment
    pub fn detect() -> Self {
        info!("Detecting test environment...");

        let display_protocol = detect_display_protocol();
        let environment_type = detect_environment();
        let has_display = Self::has_display_server();
        let available_commands = Self::detect_available_commands();
        let is_ci = matches!(environment_type, Environment::CI);
        let resources = Self::detect_resource_constraints();

        let env = Self {
            display_protocol,
            environment_type,
            has_display,
            available_commands,
            is_ci,
            resources,
        };

        info!(
            "Test environment detected: {:?} | Display: {:?} | Has Display: {} | CI: {} | Containerized: {}",
            &env.environment_type,
            &env.display_protocol,
            env.has_display,
            env.is_ci,
            env.resources.is_containerized
        );

        env
    }

    /// Check if GUI tests can run
    pub fn can_run_gui_tests(&self) -> bool {
        self.has_display && (!self.is_ci || !self.resources.is_containerized)
    }

    /// Check if Wayland-specific tests can run
    pub fn can_run_wayland_tests(&self) -> bool {
        self.display_protocol.is_wayland() && self.available_commands.contains("wl-copy")
    }

    /// Check if X11-specific tests can run
    pub fn can_run_x11_tests(&self) -> bool {
        self.display_protocol.is_x11() && self.available_commands.contains("xclip")
    }

    /// Check if daemon-dependent tests can run
    pub fn can_run_daemon_tests(&self) -> bool {
        !self.is_ci || !self.resources.is_containerized
    }

    /// Check if timing-sensitive tests can run
    pub fn can_run_timing_sensitive_tests(&self) -> bool {
        // In CI, timing-sensitive tests may need special handling
        !self.is_ci || !self.resources.limited_cpu
    }

    /// Check if current environment meets the given requirements
    pub fn meets_requirements(&self, requirements: &TestRequirements) -> TestResult {
        requirements.check(self)
    }

    /// Get appropriate timeout for tests in this environment
    pub fn get_test_timeout(&self, base_timeout: Duration) -> Duration {
        let mut multiplier = 1.0;

        if self.is_ci {
            multiplier *= 4.0; // 4x longer in CI
        }

        if self.resources.limited_memory {
            multiplier *= 1.5;
        }

        if self.resources.limited_cpu {
            multiplier *= 2.0;
        }

        Duration::from_millis((base_timeout.as_millis() as f64 * multiplier) as u64)
    }

    /// Detect if a display server is available
    fn has_display_server() -> bool {
        env::var("DISPLAY").is_ok() || env::var("WAYLAND_DISPLAY").is_ok()
    }

    /// Detect available commands on the system
    fn detect_available_commands() -> std::collections::HashSet<String> {
        let mut commands = std::collections::HashSet::new();

        // Common commands used in tests
        let test_commands = [
            "wl-copy",
            "wl-paste",
            "xclip",
            "xsel",
            "xdotool",
            "ydotool",
            "enigo",
            "dbus-send",
            "at-spi-bus-launcher",
            "xvfb-run",
            "Xvfb",
            "fluxbox",
            "openbox",
        ];

        for cmd in &test_commands {
            if Self::command_exists(cmd) {
                commands.insert(cmd.to_string());
                debug!("Found command: {}", cmd);
            }
        }

        commands
    }

    /// Check if a command exists on the system
    fn command_exists(cmd: &str) -> bool {
        Command::new("which")
            .arg(cmd)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Detect resource constraints
    fn detect_resource_constraints() -> ResourceConstraints {
        let limited_memory = Self::detect_memory_constraint();
        let limited_cpu = Self::detect_cpu_constraint();
        let is_containerized = Self::detect_containerization();

        ResourceConstraints {
            limited_memory,
            limited_cpu,
            is_containerized,
        }
    }

    /// Detect if memory is limited
    fn detect_memory_constraint() -> bool {
        // Check for common indicators of limited memory
        if let Some(memory_kb) = Self::read_file("/proc/meminfo")
            .ok()
            .and_then(|content| Self::parse_memory_kb(&content).ok())
        {
            // Consider limited if less than 2GB available
            memory_kb < 2_048_000
        } else {
            // Assume limited in CI if we can't determine
            env::var("CI").is_ok()
        }
    }

    /// Parse memory from /proc/meminfo
    fn parse_memory_kb(content: &str) -> Result<u64, ()> {
        for line in content.lines() {
            if line.starts_with("MemAvailable:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    return parts[1].parse::<u64>().map_err(|_| ());
                }
            }
        }
        Err(())
    }

    /// Detect if CPU is limited
    fn detect_cpu_constraint() -> bool {
        // Check for limited CPU cores (common in CI)
        if let Ok(cpu_info) = Self::read_file("/proc/cpuinfo") {
            let core_count = cpu_info
                .lines()
                .filter(|line| line.starts_with("processor"))
                .count();
            core_count <= 2
        } else {
            // Assume limited in CI if we can't determine
            env::var("CI").is_ok()
        }
    }

    /// Detect if running in a container
    fn detect_containerization() -> bool {
        // Check for common container indicators
        Path::new("/.dockerenv").exists()
            || Self::read_file("/proc/1/cgroup")
                .map(|content| content.contains("docker") || content.contains("container"))
                .unwrap_or(false)
            || env::var("CI").is_ok() // Assume containerized in CI
    }

    /// Read file contents safely
    fn read_file(path: &str) -> Result<String, std::io::Error> {
        std::fs::read_to_string(path)
    }
}

/// Macro to skip tests based on environment requirements
///
/// This macro replaces manual `#[ignore]` attributes with intelligent
/// environment detection and provides clear reasons for skipping.
///
/// # Examples
///
/// ```rust
/// use coldvox_foundation::test_env::*;
///
/// #[test]
/// fn my_gui_test() {
///     skip_test_unless!(
///         TestRequirements::new()
///             .requires_gui()
///             .requires_command("xclip")
///     );
///
///     // Test code here...
/// }
/// ```
#[macro_export]
macro_rules! skip_test_unless {
    ($requirements:expr) => {{
        use $crate::test_env::{TestEnvironment, TestRequirements};

        let env = TestEnvironment::detect();
        let result = env.meets_requirements(&$requirements);

        if !result.can_run {
            eprintln!("⏭️  Skipping test: {}", result.reasons.join(", "));
            return;
        }

        // Set timeout if timing-sensitive
        if $requirements.timing_sensitive {
            std::env::set_var(
                "RUST_TEST_TIMEOUT",
                format!("{}", result.suggested_timeout.as_millis()),
            );
        }
    }};
}

/// Macro for conditional test execution with detailed logging
#[macro_export]
macro_rules! run_test_if {
    ($requirements:expr, $test_code:block) => {{
        use $crate::test_env::{TestEnvironment, TestRequirements};

        let env = TestEnvironment::detect();
        let result = env.meets_requirements(&$requirements);

        if !result.can_run {
            eprintln!("⏭️  Skipping test: {}", result.reasons.join(", "));
            return;
        }

        info!(
            "✅ Running test with timeout: {:?}",
            result.suggested_timeout
        );

        // Set timeout for the test
        std::env::set_var(
            "RUST_TEST_TIMEOUT",
            format!("{}", result.suggested_timeout.as_millis()),
        );

        $test_code
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_detection() {
        let env = TestEnvironment::detect();

        // Should not panic and should provide some information
        assert!(env.available_commands.iter().all(|cmd| {
            matches!(
                cmd.as_str(),
                "wl-copy"
                    | "wl-paste"
                    | "xclip"
                    | "xsel"
                    | "xdotool"
                    | "ydotool"
                    | "enigo"
                    | "dbus-send"
                    | "at-spi-bus-launcher"
                    | "xvfb-run"
                    | "Xvfb"
                    | "fluxbox"
                    | "openbox"
            )
        }));

        // Check that display detection works
        let has_display = env.has_display;
        println!("Has display: {}", has_display);
    }

    #[test]
    fn test_requirements_builder() {
        let requirements = TestRequirements::new()
            .requires_gui()
            .requires_command("xclip")
            .requires_wayland()
            .timing_sensitive();

        assert!(requirements.requires_display);
        assert!(requirements.requires_gui);
        assert!(requirements.requires_wayland);
        assert!(requirements.timing_sensitive);
        assert!(requirements
            .required_commands
            .contains(&"xclip".to_string()));
    }

    #[test]
    fn test_requirements_check() {
        let env = TestEnvironment::detect();
        let requirements = TestRequirements::new().requires_command("nonexistent-command");

        let result = env.meets_requirements(&requirements);

        // Should fail because command doesn't exist
        assert!(!result.can_run);
        assert!(result
            .reasons
            .iter()
            .any(|r| r.contains("nonexistent-command")));
    }
}
