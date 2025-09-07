//! # Real Injection Tests (Refactored)
//!
//! These tests perform real text injection into a lightweight GTK test app.
//! They are designed to be robust, fail fast, and provide clear diagnostics.
//! They require a graphical environment and are gated by the `real-injection-tests` feature.

use crate::constants::TEST_WATCHDOG_TIMEOUT_SECS;
use crate::manager::StrategyManager;
use crate::probe::{probe_environment, ProbeState};
use crate::InjectionConfig;
use crate::InjectionMetrics;
use std::time::Duration;
use tokio::time::timeout;

// Import the test harness. Note the new module name.
use super::harness::TestApp;

/// A helper function to create a default manager and metrics sink for tests.
fn setup_test_environment() -> (StrategyManager, InjectionMetrics) {
    let config = InjectionConfig::default();
    let manager = StrategyManager::new(config);
    let metrics = InjectionMetrics::default();
    (manager, metrics)
}

/// The main test for the AT-SPI backend.
/// This test follows the new fail-fast pattern.
#[cfg(feature = "atspi")]
#[tokio::test]
async fn test_atspi_injection_e2e() {
    let test_body = async {
        // 1. Probe the environment first.
        let probe = probe_environment().await;
        if !matches!(probe, ProbeState::FullyAvailable { .. } | ProbeState::Degraded { .. }) {
            println!(
                "{{\"skip\":\"AT-SPI test skipped: environment not ready: {:?}\"}}",
                probe
            );
            return;
        }

        // 2. Launch the test application harness.
        let app = TestApp::launch().expect("Failed to launch GTK test app.");
        if !app.wait_ready(1000).await {
            panic!("Test app failed to become ready in time.");
        }

        // 3. Setup the injection manager.
        let (manager, mut metrics) = setup_test_environment();
        let test_text = "Hello from a robust AT-SPI test! 🎤";

        // 4. Perform the injection.
        let result = manager.inject_with_fail_fast(test_text, &mut metrics).await;

        // 5. Verify the outcome.
        assert!(
            result.is_ok(),
            "AT-SPI injection failed: {:?}",
            result.err()
        );
        let outcome = result.unwrap();
        assert_eq!(outcome.backend, crate::probe::BackendId::Atspi);

        // 6. Verify the text was received by the app.
        app.verify(test_text)
            .await
            .expect("Text verification failed.");
    };

    // Wrap the entire test in a watchdog timeout.
    match timeout(
        Duration::from_secs(TEST_WATCHDOG_TIMEOUT_SECS),
        test_body,
    )
    .await
    {
        Ok(_) => (),
        Err(_) => panic!("Test watchdog timeout exceeded! The test hung."),
    }
}

/// The main test for the Clipboard backends (Wayland/X11).
#[tokio::test]
async fn test_clipboard_injection_e2e() {
    let test_body = async {
        // 1. Probe the environment.
        let probe = probe_environment().await;
        let has_clipboard_backend = match &probe {
            ProbeState::FullyAvailable { usable } | ProbeState::Degraded { usable, .. } => usable
                .iter()
                .any(|b| matches!(b, crate::probe::BackendId::ClipboardX11 | crate::probe::BackendId::ClipboardWayland)),
            _ => false,
        };

        if !has_clipboard_backend {
            println!(
                "{{\"skip\":\"Clipboard test skipped: no clipboard backend available: {:?}\"}}",
                probe
            );
            return;
        }

        // 2. Launch app and setup manager.
        let app = TestApp::launch().expect("Failed to launch test app.");
        assert!(app.wait_ready(1000).await, "Test app not ready in time.");
        let (manager, mut metrics) = setup_test_environment();
        let test_text = "Hello from a robust Clipboard test! 📋";

        // 3. Perform injection.
        // NOTE: This test assumes an external mechanism would trigger the "paste" action
        // (e.g., Ctrl+V). The `ClipboardInjector` only sets the content.
        // For this test, we can simulate the paste by reading the clipboard content
        // ourselves, which is what a real paste would do.
        let result = manager.inject_with_fail_fast(test_text, &mut metrics).await;
        assert!(result.is_ok(), "Clipboard injection failed: {:?}", result.err());

        // 4. Verify.
        // We can't easily verify the UI, but we can verify the clipboard content was set.
        // A more complex test would involve a second injector to send a paste command.
        // For now, we trust the outcome struct.
        let outcome = result.unwrap();
        assert!(matches!(
            outcome.backend,
            crate::probe::BackendId::ClipboardX11 | crate::probe::BackendId::ClipboardWayland
        ));

        // To make the test more complete, let's get the clipboard content back.
        // This requires a bit of a hack since the manager doesn't expose this.
        let clipboard_content = if outcome.backend == crate::probe::BackendId::ClipboardWayland {
            crate::subprocess::run_tool_with_timeout("wl-paste", &["--no-newline"], 200).await
        } else {
            crate::subprocess::run_tool_with_timeout("xclip", &["-selection", "clipboard", "-o"], 200).await
        };

        assert_eq!(
            clipboard_content.unwrap_or_default(),
            test_text,
            "Clipboard content was not set correctly."
        );
    };

    match timeout(
        Duration::from_secs(TEST_WATCHDOG_TIMEOUT_SECS),
        test_body,
    )
    .await
    {
        Ok(_) => (),
        Err(_) => panic!("Test watchdog timeout exceeded! The test hung."),
    }
}
