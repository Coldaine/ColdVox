//! # Test Harness for Real Injection Tests
//!
//! This module provides utilities for launching test applications and verifying
//! that text injection was successful. It is designed to be robust and avoid
//! common pitfalls of UI testing, like race conditions and zombie processes.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use tokio::task::spawn_blocking;
use tokio::time::timeout;

/// Represents a running instance of a test application.
/// Its `Drop` implementation ensures the process is terminated.
pub struct TestApp {
    process: Child,
    pub pid: u32,
    pub output_file: PathBuf,
}

impl TestApp {
    /// Launches the GTK test application.
    pub fn launch() -> Result<Self, std::io::Error> {
        let out_dir = env::var("OUT_DIR").ok();
        // Fallback for when build script doesn't run (e.g. `cargo test --no-build`)
        let exe_path = out_dir.map_or_else(
            || PathBuf::from("target/debug/gtk_test_app"),
            |dir| Path::new(&dir).join("gtk_test_app"),
        );

        if !exe_path.exists() {
            // Try another common path before giving up
            let fallback_path = PathBuf::from("target/debug/gtk_test_app");
            if !fallback_path.exists() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("GTK test app not found at {:?} or {:?}", exe_path, fallback_path),
                ));
            }
        }

        let pid = std::process::id();
        let output_file = PathBuf::from(format!("/tmp/coldvox_gtk_test_{}.txt", pid));
        let _ = fs::remove_file(&output_file);

        let process = Command::new(&exe_path)
            .env("COLDVOX_TEST_OUTPUT_FILE", &output_file)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        let pid = process.id();

        Ok(TestApp {
            process,
            pid,
            output_file,
        })
    }

    /// Waits for the test application to be ready.
    pub async fn wait_ready(&self, max_wait_ms: u64) -> bool {
        let deadline = Instant::now() + Duration::from_millis(max_wait_ms);
        loop {
            if self.output_file.exists() {
                return true;
            }
            if Instant::now() > deadline {
                return false;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    /// Asynchronously verifies that the expected text was injected.
    pub async fn verify(&self, expected_text: &str) -> Result<(), String> {
        let verification_timeout = Duration::from_millis(1000);
        match timeout(verification_timeout, async {
            loop {
                if let Ok(content) = fs::read_to_string(&self.output_file) {
                    if content.trim() == expected_text.trim() {
                        return Ok::<(), String>(());
                    }
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        {
            Ok(Ok(_)) => Ok(()),
            _ => {
                let final_content = fs::read_to_string(&self.output_file)
                    .unwrap_or_else(|_| "<file not found or unreadable>".to_string());
                Err(format!(
                    "Verification failed. Expected: '{}', Found: '{}'",
                    expected_text, final_content
                ))
            }
        }
    }

    /// Ensures the test application process is terminated.
    pub async fn kill(mut self) {
        let _ = self.process.kill();
        // Use spawn_blocking to wait for the process without blocking the async runtime.
        let _ = spawn_blocking(move || self.process.wait()).await;
    }
}

impl Drop for TestApp {
    fn drop(&mut self) {
        // Best-effort attempt to kill the process.
        if let Err(e) = self.process.kill() {
            eprintln!("Failed to kill test app process {}: {}", self.pid, e);
        }
        // NOTE: We cannot call an async `wait` here. The `kill` signal should
        // be enough for the OS to clean up. This is a compromise for Drop.
        // The explicit `kill` method should be preferred for robust cleanup.
        let _ = self.process.wait();
        let _ = fs::remove_file(&self.output_file);
    }
}
