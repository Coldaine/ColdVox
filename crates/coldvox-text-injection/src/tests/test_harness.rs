use coldvox_foundation::test_env::TestEnvironment as FoundationTestEnv;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// Represents a running instance of a test application.
///
/// This struct manages lifecycle of test application process.
/// When it goes out of scope, its `Drop` implementation ensures that process
/// is terminated and any associated temporary files are cleaned up.
pub struct TestApp {
    /// The running child process.
    pub process: Child,
    /// The process ID.
    pub pid: u32,
    /// The path to temporary output file that app writes to.
    pub output_file: PathBuf,
}

impl Drop for TestApp {
    fn drop(&mut self) {
        // First try to terminate gracefully with SIGTERM (if on Unix)
        #[cfg(unix)]
        {
            use std::process::Command;
            let _ = Command::new("kill")
                .arg("-TERM")
                .arg(self.pid.to_string())
                .output();

            // Give process a moment to exit gracefully
            std::thread::sleep(Duration::from_millis(100));
        }

        // Check if process is still running before trying to kill it
        match self.process.try_wait() {
            Ok(Some(_exit_status)) => {
                // Process has already exited, no need to kill
            }
            Ok(None) => {
                // Process is still running, try to kill it
                if let Err(e) = self.process.kill() {
                    // Handle common error cases
                    if e.kind() == std::io::ErrorKind::InvalidInput {
                        // Process may have already exited
                        eprintln!("Process PID {} already exited during cleanup", self.pid);
                    } else {
                        eprintln!(
                            "Failed to kill test app process with PID {}: {}",
                            self.pid, e
                        );
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to check process status for PID {}: {}", self.pid, e);
                // Try to kill anyway as a fallback
                let _ = self.process.kill();
            }
        }

        // Wait for process to avoid zombies with a timeout
        let start = Instant::now();
        let wait_timeout = Duration::from_secs(5);

        while start.elapsed() < wait_timeout {
            match self.process.try_wait() {
                Ok(Some(_exit_status)) => {
                    // Process has exited
                    break;
                }
                Ok(None) => {
                    // Still running, wait a bit more
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(e) => {
                    eprintln!("Error waiting for process PID {}: {}", self.pid, e);
                    break;
                }
            }
        }

        // Final attempt to wait (non-blocking)
        if let Err(e) = self.process.try_wait() {
            eprintln!(
                "Final wait failed for test app process with PID {}: {}",
                self.pid, e
            );
        }

        // Clean up any remaining child processes (Unix only)
        #[cfg(unix)]
        {
            use std::process::Command;
            // Kill any child processes in process group
            let _ = Command::new("pkill")
                .arg("-P") // Parent PID
                .arg(self.pid.to_string())
                .output();
        }

        // Clean up temporary output file.
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
    /// Launches GTK test application.
    ///
    /// The application is expected to have been compiled by `build.rs` script.
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
            .stdout(Stdio::null()) // Prevent app from polluting test output.
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

    /// Launches terminal test application.
    ///
    /// The application is expected to have been compiled by `build.rs` script.
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
pub async fn verify_injection(output_file: &Path, expected_text: &str) -> Result<(), String> {
    verify_injection_with_timeout(output_file, expected_text, None).await
}

/// Helper function to verify text injection by polling a file with configurable timeout.
///
/// Uses a longer timeout in CI environments where systems may be under higher load.
pub async fn verify_injection_with_timeout(
    output_file: &Path,
    expected_text: &str,
    custom_timeout: Option<Duration>,
) -> Result<(), String> {
    let start = Instant::now();

    // Use custom timeout or determine based on environment
    let env = FoundationTestEnv::detect();
    let timeout =
        custom_timeout.unwrap_or_else(|| env.get_test_timeout(Duration::from_millis(500)));

    while start.elapsed() < timeout {
        if let Ok(content) = fs::read_to_string(output_file) {
            if content.trim() == expected_text.trim() {
                return Ok(());
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    let final_content = fs::read_to_string(output_file)
        .unwrap_or_else(|_| "<file not found or unreadable>".to_string());
    Err(format!(
        "Verification failed after {}ms. Expected: '{}', Found: '{}'",
        timeout.as_millis(),
        expected_text,
        final_content.trim()
    ))
}

/// Provides information about current test environment to determine
/// if real injection tests are feasible to run.
///
/// This is now a wrapper around foundation's TestEnvironment
/// for backward compatibility.
pub struct TestEnvironment {
    pub has_display: bool,
    pub is_ci: bool,
    inner: FoundationTestEnv,
}

impl TestEnvironment {
    /// Creates a new `TestEnvironment` by inspecting environment variables.
    pub fn current() -> Self {
        let inner = FoundationTestEnv::detect();

        Self {
            has_display: inner.has_display,
            is_ci: inner.is_ci,
            inner,
        }
    }

    /// Determines if environment is suitable for running real injection tests.
    pub fn can_run_real_tests(&self) -> bool {
        self.inner.can_run_gui_tests()
    }

    /// Check if Wayland-specific tests can run
    pub fn can_run_wayland_tests(&self) -> bool {
        self.inner.can_run_wayland_tests()
    }

    /// Check if X11-specific tests can run
    pub fn can_run_x11_tests(&self) -> bool {
        self.inner.can_run_x11_tests()
    }

    /// Check if daemon-dependent tests can run
    pub fn can_run_daemon_tests(&self) -> bool {
        self.inner.can_run_daemon_tests()
    }
}
