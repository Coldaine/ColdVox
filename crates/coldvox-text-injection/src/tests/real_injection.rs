//! # Real Injection Tests
//!
//! This module contains tests that perform real text injection into lightweight
//! test applications. These tests require a graphical environment (X11 or Wayland)
//! and are therefore ignored by default.
//!
//! To run these tests, use the following command:
//! `cargo test -p coldvox-text-injection --features real-injection-tests`

#![cfg(feature = "real-injection-tests")]

// NOTE: Using modular injectors from the injectors module
#[cfg(feature = "wl_clipboard")]
use crate::clipboard_paste_injector::ClipboardPasteInjector;
#[cfg(feature = "enigo")]
use crate::enigo_injector::EnigoInjector;
#[cfg(feature = "atspi")]
use crate::injectors::atspi::AtspiInjector;
#[cfg(feature = "ydotool")]
use crate::ydotool_injector::YdotoolInjector;
// Bring trait into scope so async trait methods (inject_text, is_available) resolve.
#[cfg(any(
    feature = "atspi",
    feature = "enigo",
    feature = "wl_clipboard",
    feature = "ydotool"
))]
use crate::TextInjector;

use crate::tests::test_harness::{TestApp, TestAppManager, TestEnvironment};

#[cfg(any(
    feature = "atspi",
    feature = "enigo",
    feature = "wl_clipboard",
    feature = "ydotool"
))]
use crate::tests::test_harness::verify_injection;
use std::time::Duration;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// A placeholder test to verify that the test harness, build script, and
/// environment detection are all working correctly.
#[tokio::test]

async fn harness_self_test_launch_gtk_app() {
    // Setup logging
    let _ = std::fs::create_dir_all("target/logs");
    let file_appender = tracing_appender::rolling::never("target/logs", "text_injection_tests.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    let _ = tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false),
        )
        .with(tracing_subscriber::fmt::layer().with_test_writer())
        .try_init();

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

/// Waits for the test application to be ready by polling for its output file.
/// This is much faster than a fixed-duration sleep.
async fn wait_for_app_ready(app: &TestApp) {
    let max_wait = Duration::from_secs(5);
    let poll_interval = Duration::from_millis(50);
    let start_time = std::time::Instant::now();

    while start_time.elapsed() < max_wait {
        if app.output_file.exists() {
            // A small extra delay to ensure the app is fully interactive
            tokio::time::sleep(Duration::from_millis(50)).await;
            return;
        }
        tokio::time::sleep(poll_interval).await;
    }
    panic!("Test application did not become ready within 5 seconds.");
}

//--- AT-SPI Tests ---

/// Helper function to run a complete injection and verification test for the AT-SPI backend.
async fn run_atspi_test(test_text: &str) {
    // Setup logging
    let _ = std::fs::create_dir_all("target/logs");
    let file_appender = tracing_appender::rolling::never("target/logs", "text_injection_tests.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    let _ = tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false),
        )
        .with(tracing_subscriber::fmt::layer().with_test_writer())
        .try_init();

    let env = TestEnvironment::current();
    if !env.can_run_real_tests() {
        // This check is technically redundant if the tests are run with the top-level skip,
        // but it's good practice to keep it for clarity and direct execution.
        eprintln!("Skipping AT-SPI test: no display server found.");
        return;
    }

    let app = TestAppManager::launch_gtk_app().expect("Failed to launch GTK app.");

    // Allow time for the app to initialize and for the AT-SPI bus to register it.
    // This is a common requirement in UI testing.
    tokio::time::sleep(Duration::from_millis(500)).await;
    // Wait for the app to be fully initialized before interacting with it.
    wait_for_app_ready(&app).await;

    #[cfg(feature = "atspi")]
    {
        let injector = AtspiInjector::new(Default::default());
        if !injector.is_available().await {
            println!(
                "Skipping AT-SPI test: backend is not available (is at-spi-bus-launcher running?)."
            );
            return;
        }

        injector.inject_text(test_text).await.unwrap_or_else(|e| {
            panic!("AT-SPI injection failed for text '{}': {:?}", test_text, e)
        });

        verify_injection(&app.output_file, test_text)
            .await
            .unwrap_or_else(|e| {
                panic!(
                    "Verification failed for AT-SPI with text '{}': {}",
                    test_text, e
                )
            });
    }

    #[cfg(not(feature = "atspi"))]
    {
        // Suppress unused variable warning when atspi is disabled
        let _ = test_text;
        let _ = app;
        println!("Skipping AT-SPI test: atspi feature not enabled");
    }
}

#[tokio::test]

async fn test_atspi_simple_text() {
    run_atspi_test("Hello from AT-SPI!").await;
}

#[tokio::test]

async fn test_atspi_unicode_text() {
    run_atspi_test("Hello ColdVox 🎤 测试").await;
}

#[tokio::test]

async fn test_atspi_long_text() {
    // A long string to test for buffer issues.
    let long_text =
        "This is a long string designed to test the injection capabilities of the backend. "
            .repeat(50);
    assert!(long_text.len() > 1000);
    run_atspi_test(&long_text).await;
}

#[tokio::test]

async fn test_atspi_special_chars() {
    run_atspi_test("Line 1\nLine 2\twith a tab\nAnd some symbols: !@#$%^&*()_+").await;
}

//--- Ydotool Tests ---
#[cfg(feature = "ydotool")]

/// Helper function to run a complete injection and verification test for the ydotool backend.
/// This test involves setting the clipboard, as ydotool's primary injection method is paste.
async fn run_ydotool_test(test_text: &str) {
    // Setup logging
    let _ = std::fs::create_dir_all("target/logs");
    let file_appender = tracing_appender::rolling::never("target/logs", "text_injection_tests.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    let _ = tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false),
        )
        .with(tracing_subscriber::fmt::layer().with_test_writer())
        .try_init();

    let env = TestEnvironment::current();
    if !env.can_run_real_tests() {
        eprintln!("Skipping ydotool test: no display server found.");
        return;
    }

    // ydotool requires a running daemon and access to /dev/uinput.
    // The injector's `is_available` check will handle this.
    let injector = YdotoolInjector::new(Default::default());
    if !injector.is_available().await {
        println!("Skipping ydotool test: backend is not available (is ydotool daemon running?).");
        return;
    }

    // Set the clipboard content. We use `arboard` as it works on both X11 and Wayland.
    let mut clipboard = arboard::Clipboard::new().expect("Failed to create clipboard context.");
    clipboard
        .set_text(test_text.to_string())
        .expect("Failed to set clipboard text.");

    // Verify that clipboard content was set correctly before proceeding
    let clipboard_content = clipboard.get_text().expect("Failed to get clipboard text.");
    assert_eq!(
        clipboard_content, test_text,
        "Clipboard content was not set correctly."
    );

    let app = TestAppManager::launch_gtk_app().expect("Failed to launch GTK app.");
    tokio::time::sleep(Duration::from_millis(500)).await;
    wait_for_app_ready(&app).await;

    // The inject_text for ydotool will trigger a paste (Ctrl+V).
    injector
        .inject_text(test_text)
        .await
        .unwrap_or_else(|e| panic!("ydotool injection failed for text '{}': {:?}", test_text, e));

    verify_injection(&app.output_file, test_text)
        .await
        .unwrap_or_else(|e| {
            panic!(
                "Verification failed for ydotool with text '{}': {}",
                test_text, e
            )
        });
}

#[tokio::test]
#[cfg(feature = "ydotool")]

async fn test_ydotool_simple_text() {
    run_ydotool_test("Hello from ydotool!").await;
}

#[tokio::test]
#[cfg(feature = "ydotool")]

async fn test_ydotool_unicode_text() {
    run_ydotool_test("Hello ColdVox 🎤 测试 (via ydotool)").await;
}

#[tokio::test]
#[cfg(feature = "ydotool")]

async fn test_ydotool_long_text() {
    let long_text = "This is a long string for ydotool. ".repeat(50);
    assert!(long_text.len() > 1000);
    run_ydotool_test(&long_text).await;
}

#[tokio::test]
#[cfg(feature = "ydotool")]

async fn test_ydotool_special_chars() {
    run_ydotool_test("ydotool line 1\nydotool line 2\twith tab").await;
}

//--- Clipboard + Paste Tests ---

/// Helper to test clipboard injection followed by a paste action.
/// This simulates a realistic clipboard workflow.
async fn run_clipboard_paste_test(test_text: &str) {
    // Setup logging
    let _ = std::fs::create_dir_all("target/logs");
    let file_appender = tracing_appender::rolling::never("target/logs", "text_injection_tests.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    let _ = tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false),
        )
        .with(tracing_subscriber::fmt::layer().with_test_writer())
        .try_init();

    let env = TestEnvironment::current();
    if !env.can_run_real_tests() {
        eprintln!("Skipping clipboard test: no display server found.");
        return;
    }

    // This test requires both a clipboard manager and a paste mechanism.
    // We use ClipboardInjector (Wayland) and Enigo (cross-platform paste).
    #[cfg(all(feature = "wl_clipboard", feature = "enigo"))]
    {
        // Use ClipboardPasteInjector which sets clipboard and attempts a paste (via AT-SPI/ydotool).
        let clipboard_paste = ClipboardPasteInjector::new(Default::default());
        if !clipboard_paste.is_available().await {
            println!("Skipping clipboard test: backend is not available (not on Wayland?).");
            return;
        }

        // Launch the app to paste into.
        let app = TestAppManager::launch_gtk_app().expect("Failed to launch GTK app.");
        tokio::time::sleep(Duration::from_millis(500)).await;
        wait_for_app_ready(&app).await;

        // Perform clipboard+paste using the combined injector (it will try AT-SPI first then ydotool).
        clipboard_paste
            .inject_text(test_text)
            .await
            .expect("Clipboard+paste injection failed.");

        // Verify the result.
        verify_injection(&app.output_file, test_text)
            .await
            .unwrap_or_else(|e| {
                panic!(
                    "Verification failed for clipboard paste with text '{}': {}",
                    test_text, e
                )
            });
    }

    #[cfg(not(all(feature = "wl_clipboard", feature = "enigo")))]
    {
        // Suppress unused variable warning when features are disabled
        let _ = test_text;
        println!("Skipping clipboard test: required features (wl_clipboard, enigo) not enabled");
    }
}

#[tokio::test]

async fn test_clipboard_simple_text() {
    run_clipboard_paste_test("Hello from the clipboard!").await;
}

#[tokio::test]

async fn test_clipboard_unicode_text() {
    run_clipboard_paste_test("Clipboard 🎤 and paste 🎤").await;
}

//--- Enigo (Typing) Tests ---

/// Helper to test the direct typing capability of the Enigo backend.
async fn run_enigo_typing_test(test_text: &str) {
    // Setup logging
    let _ = std::fs::create_dir_all("target/logs");
    let file_appender = tracing_appender::rolling::never("target/logs", "text_injection_tests.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    let _ = tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false),
        )
        .with(tracing_subscriber::fmt::layer().with_test_writer())
        .try_init();

    let env = TestEnvironment::current();
    if !env.can_run_real_tests() {
        eprintln!("Skipping enigo typing test: no display server found.");
        return;
    }

    #[cfg(feature = "enigo")]
    {
        let injector = EnigoInjector::new(Default::default());
        if !injector.is_available().await {
            println!("Skipping enigo typing test: backend is not available.");
            return;
        }

        let app = TestAppManager::launch_gtk_app().expect("Failed to launch GTK app.");
        tokio::time::sleep(Duration::from_millis(500)).await;
        wait_for_app_ready(&app).await;

        // Use the test-only helper to force typing instead of pasting.
        injector
            .type_text_directly(test_text)
            .await
            .unwrap_or_else(|e| panic!("Enigo typing failed for text '{}': {:?}", test_text, e));

        verify_injection(&app.output_file, test_text)
            .await
            .unwrap_or_else(|e| {
                panic!(
                    "Verification failed for enigo typing with text '{}': {}",
                    test_text, e
                )
            });
    }

    #[cfg(not(feature = "enigo"))]
    {
        // Suppress unused variable warning when feature is disabled
        let _ = test_text;
        println!("Skipping enigo typing test: enigo feature not enabled");
    }
}

#[tokio::test]

async fn test_enigo_typing_simple_text() {
    run_enigo_typing_test("Enigo types this text.").await;
}

#[tokio::test]

async fn test_enigo_typing_unicode_text() {
    // Note: Enigo's unicode support can be platform-dependent. This test will verify it.
    run_enigo_typing_test("Enigo 🎤 typing 🎤 unicode").await;
}

#[tokio::test]

async fn test_enigo_typing_special_chars() {
    run_enigo_typing_test("Enigo types\nnew lines and\ttabs.").await;
}

// TODO(#40): Add tests for kdotool, combo injectors etc.
