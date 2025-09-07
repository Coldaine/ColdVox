use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// Represents a running instance of a test application.
///
/// This struct manages the lifecycle of the test application process.
/// When it goes out of scope, its `Drop` implementation ensures the process
/// is terminated and any associated temporary files are cleaned up.
pub struct TestApp {
    /// The running child process.
    pub process: Child,
    /// The process ID.
    pub pid: u32,
    /// The path to the temporary output file the app writes to.
    pub output_file: PathBuf,
}

impl Drop for TestApp {
    fn drop(&mut self) {
        // Aggressively terminate the process.
        if let Err(e) = self.process.kill() {
            eprintln!(
                "Failed to kill test app process with PID {}: {}",
                self.pid, e
            );
        }
        // It's good practice to wait for the process to avoid zombies.
        if let Err(e) = self.process.wait() {
            eprintln!(
                "Failed to wait for test app process with PID {}: {}",
                self.pid, e
            );
        }

        // Clean up the temporary output file.
        if self.output_file.exists() {
            if let Err(e) = fs::remove_file(&self.output_file) {
                eprintln!("Failed to remove temp file {:?}: {}", self.output_file, e);
            }
        }
    }
}

/// A manager responsible for launching test applications.
///
/// This acts as a factory for creating `TestApp` instances.
pub struct TestAppManager;

impl TestAppManager {
    /// Launches the GTK test application.
    ///
    /// The application is expected to have been compiled by the `build.rs` script.
    pub fn launch_gtk_app() -> Result<TestApp, std::io::Error> {
        let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set, build script did not run?");
        let exe_path = Path::new(&out_dir).join("gtk_test_app");

        if !exe_path.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!(
                    "GTK test app executable not found at {:?}. Did build.rs fail to build it?",
                    exe_path
                ),
            ));
        }

        let process = Command::new(&exe_path)
            .stdout(Stdio::null()) // Prevent the app from polluting test output.
            .stderr(Stdio::null())
            .spawn()?;

        let pid = process.id();
        let output_file = PathBuf::from(format!("/tmp/coldvox_gtk_test_{}.txt", pid));

        Ok(TestApp {
            process,
            pid,
            output_file,
        })
    }

    /// Launches the terminal test application.
    ///
    /// The application is expected to have been compiled by the `build.rs` script.
    pub fn launch_terminal_app() -> Result<TestApp, std::io::Error> {
        let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set, build script did not run?");
        let exe_path = Path::new(&out_dir).join("terminal-test-app");

        if !exe_path.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Terminal test app executable not found at {:?}. Did build.rs fail to build it?", exe_path)
            ));
        }

        let process = Command::new(&exe_path)
            .stdin(Stdio::piped()) // We need to write to the app's stdin.
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        let pid = process.id();
        let output_file = PathBuf::from(format!("/tmp/coldvox_terminal_test_{}.txt", pid));

        Ok(TestApp {
            process,
            pid,
            output_file,
        })
    }
}

/// Helper function to verify text injection by polling a file.
pub fn verify_injection(output_file: &Path, expected_text: &str) -> Result<(), String> {
    let start = Instant::now();
    let timeout = Duration::from_millis(500);

    while start.elapsed() < timeout {
        if let Ok(content) = fs::read_to_string(output_file) {
            if content == expected_text {
                return Ok(());
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    let final_content = fs::read_to_string(output_file)
        .unwrap_or_else(|_| "<file not found or unreadable>".to_string());
    Err(format!(
        "Verification failed after {}ms. Expected: '{}', Found: '{}'",
        timeout.as_millis(),
        expected_text,
        final_content
    ))
}

/// Provides information about the current test environment to determine
/// if real injection tests are feasible to run.
pub struct TestEnvironment {
    pub has_display: bool,
    pub is_ci: bool,
}

impl TestEnvironment {
    /// Creates a new `TestEnvironment` by inspecting environment variables.
    pub fn current() -> Self {
        // A display server is required for any UI-based injection.
        let has_display = env::var("DISPLAY").is_ok() || env::var("WAYLAND_DISPLAY").is_ok();

        // The CI variable is a de-facto standard for detecting CI environments.
        let is_ci = env::var("CI").is_ok();

        Self { has_display, is_ci }
    }

    /// Determines if the environment is suitable for running real injection tests.
    ///
    /// For now, this is simply an alias for checking for a display, but could be
    /// expanded in the future.
    pub fn can_run_real_tests(&self) -> bool {
        self.has_display
    }
}
