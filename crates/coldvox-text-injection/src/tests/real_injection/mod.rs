//! # Real Injection Tests
//!
//! This module contains tests that perform real text injection into lightweight
//! test applications. These tests require a graphical environment (X11 or Wayland)
//! and are therefore ignored by default.
//!
//! To run these tests, use the following command:
//! `cargo test -p coldvox-text-injection --features real-injection-tests`

use crate::tests::test_harness::{verify_injection, TestAppManager, TestEnvironment};
use crate::TextInjector;
use std::time::Duration;

// --- Test Modules ---
#[cfg(feature = "atspi")]
mod atspi;
#[cfg(all(feature = "wl_clipboard", feature = "enigo"))]
mod clipboard;
#[cfg(feature = "enigo")]
mod enigo;
#[cfg(feature = "kdotool")]
mod kdotool;
#[cfg(feature = "ydotool")]
mod ydotool;

/// A generic test runner for injection backends.
///
/// This function handles the boilerplate of setting up the test environment,
/// launching the test app, running the injection, and verifying the result.
pub async fn run_test(test_text: &str, injector: &dyn TextInjector) {
    let env = TestEnvironment::current();
    if !env.can_run_real_tests() {
        eprintln!(
            "Skipping real injection test for backend '{}': no display server found.",
            injector.backend_name()
        );
        return;
    }

    let app = TestAppManager::launch_gtk_app().expect("Failed to launch GTK app.");
    // Allow time for the app to initialize and for the AT-SPI bus to register it.
    tokio::time::sleep(Duration::from_millis(500)).await;

    injector
        .inject_text(test_text, None)
        .await
        .unwrap_or_else(|e| {
            panic!(
                "Injection failed for backend '{}' with text '{}': {:?}",
                injector.backend_name(),
                test_text,
                e
            )
        });

    verify_injection(&app.output_file, test_text)
        .await
        .unwrap_or_else(|e| {
            panic!(
                "Verification failed for backend '{}' with text '{}': {}",
                injector.backend_name(),
                test_text,
                e
            )
        });
}

/// A placeholder test to verify that the test harness, build script, and
/// environment detection are all working correctly.
#[tokio::test]
async fn harness_self_test_launch_gtk_app() {
    let env = TestEnvironment::current();
    if !env.can_run_real_tests() {
        eprintln!("Skipping real injection test: no display server found.");
        return;
    }

    println!("Attempting to launch GTK test app...");
    let app_handle = TestAppManager::launch_gtk_app()
        .expect("Failed to launch GTK test app. Check build.rs output and ensure GTK3 dev libraries are installed.");

    // The app should be running. We'll give it a moment to stabilize.
    tokio::time::sleep(Duration::from_millis(200)).await;

    // The test passes if the app launches without error and is cleaned up.
    // The cleanup is handled by the `Drop` implementation of `TestApp`.
    println!(
        "GTK test app launched successfully and will be cleaned up. PID: {}",
        app_handle.pid
    );
}
