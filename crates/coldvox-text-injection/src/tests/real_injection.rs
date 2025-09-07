//! # Real Injection Tests
//!
//! This module contains tests that perform real text injection into lightweight
//! test applications. These tests require a graphical environment (X11 or Wayland)
//! and are therefore ignored by default.
//!
//! To run these tests, use the following command:
//! `cargo test -p coldvox-text-injection --features real-injection-tests`

// Allow dead code for now, as this is a new module and not all helpers
// might be used immediately.
#![allow(dead_code)]

use crate::atspi_injector::AtSpiInjector;
use crate::backend::Backend;
use crate::clipboard_injector::ClipboardInjector;
use crate::enigo_injector::EnigoInjector;
use crate::manager::StrategyManager;
use crate::types::{InjectionConfig, InjectionError, InjectionMethod};
use crate::ydotool_injector::YdotoolInjector;

use super::test_harness::{verify_injection, TestAppManager, TestEnvironment};
use std::time::Duration;

/// A placeholder test to verify that the test harness, build script, and
/// environment detection are all working correctly.
#[tokio::test]
#[cfg_attr(not(feature = "real-injection-tests"), ignore)]
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

//--- AT-SPI Tests ---

/// Helper function to run a complete injection and verification test for the AT-SPI backend.
async fn run_atspi_test(test_text: &str) {
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

    let injector =
        AtSpiInjector::new(Default::default()).expect("Failed to create AT-SPI injector");
    if !injector.is_available().await {
        println!(
            "Skipping AT-SPI test: backend is not available (is at-spi-bus-launcher running?)."
        );
        return;
    }

    injector
        .inject_text(test_text)
        .await
        .unwrap_or_else(|e| panic!("AT-SPI injection failed for text '{}': {:?}", test_text, e));

    verify_injection(&app.output_file, test_text).unwrap_or_else(|e| {
        panic!(
            "Verification failed for AT-SPI with text '{}': {}",
            test_text, e
        )
    });
}

#[tokio::test]
#[cfg_attr(not(feature = "real-injection-tests"), ignore)]
async fn test_atspi_simple_text() {
    run_atspi_test("Hello from AT-SPI!").await;
}

#[tokio::test]
#[cfg_attr(not(feature = "real-injection-tests"), ignore)]
async fn test_atspi_unicode_text() {
    run_atspi_test("Hello ColdVox 🎤 测试").await;
}

#[tokio::test]
#[cfg_attr(not(feature = "real-injection-tests"), ignore)]
async fn test_atspi_long_text() {
    // A long string to test for buffer issues.
    let long_text =
        "This is a long string designed to test the injection capabilities of the backend. "
            .repeat(50);
    assert!(long_text.len() > 1000);
    run_atspi_test(&long_text).await;
}

#[tokio::test]
#[cfg_attr(not(feature = "real-injection-tests"), ignore)]
async fn test_atspi_special_chars() {
    run_atspi_test("Line 1\nLine 2\twith a tab\nAnd some symbols: !@#$%^&*()_+").await;
}

//--- Ydotool Tests ---

/// Helper function to run a complete injection and verification test for the ydotool backend.
/// This test involves setting the clipboard, as ydotool's primary injection method is paste.
async fn run_ydotool_test(test_text: &str) {
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

    // The inject_text for ydotool will trigger a paste (Ctrl+V).
    injector
        .inject_text(test_text)
        .await
        .unwrap_or_else(|e| panic!("ydotool injection failed for text '{}': {:?}", test_text, e));

    verify_injection(&app.output_file, test_text).unwrap_or_else(|e| {
        panic!(
            "Verification failed for ydotool with text '{}': {}",
            test_text, e
        )
    });
}

#[tokio::test]
#[cfg_attr(not(feature = "real-injection-tests"), ignore)]
async fn test_ydotool_simple_text() {
    run_ydotool_test("Hello from ydotool!").await;
}

#[tokio::test]
#[cfg_attr(not(feature = "real-injection-tests"), ignore)]
async fn test_ydotool_unicode_text() {
    run_ydotool_test("Hello ColdVox 🎤 测试 (via ydotool)").await;
}

#[tokio::test]
#[cfg_attr(not(feature = "real-injection-tests"), ignore)]
async fn test_ydotool_long_text() {
    let long_text = "This is a long string for ydotool. ".repeat(50);
    assert!(long_text.len() > 1000);
    run_ydotool_test(&long_text).await;
}

#[tokio::test]
#[cfg_attr(not(feature = "real-injection-tests"), ignore)]
async fn test_ydotool_special_chars() {
    run_ydotool_test("ydotool line 1\nydotool line 2\twith tab").await;
}

//--- Clipboard + Paste Tests ---

/// Helper to test clipboard injection followed by a paste action.
/// This simulates a realistic clipboard workflow.
async fn run_clipboard_paste_test(test_text: &str) {
    let env = TestEnvironment::current();
    if !env.can_run_real_tests() {
        eprintln!("Skipping clipboard test: no display server found.");
        return;
    }

    // This test requires both a clipboard manager and a paste mechanism.
    // We use ClipboardInjector (Wayland) and Enigo (cross-platform paste).
    let clipboard_injector = ClipboardInjector::new(Default::default());
    if !clipboard_injector.is_available().await {
        println!("Skipping clipboard test: backend is not available (not on Wayland?).");
        return;
    }

    let enigo_injector = EnigoInjector::new(Default::default());
    if !enigo_injector.is_available().await {
        println!("Skipping clipboard test: Enigo backend for pasting is not available.");
        return;
    }

    // 1. Set clipboard content using the ClipboardInjector.
    clipboard_injector
        .inject_text(test_text)
        .await
        .expect("Setting clipboard failed.");

    // 2. Launch the app to paste into.
    let app = TestAppManager::launch_gtk_app().expect("Failed to launch GTK app.");
    tokio::time::sleep(Duration::from_millis(500)).await;

    // 3. Trigger a paste action. We can use enigo for this.
    enigo_injector
        .inject_text("")
        .await
        .expect("Enigo paste action failed.");

    // 4. Verify the result.
    verify_injection(&app.output_file, test_text).unwrap_or_else(|e| {
        panic!(
            "Verification failed for clipboard paste with text '{}': {}",
            test_text, e
        )
    });
}

#[tokio::test]
#[cfg_attr(not(feature = "real-injection-tests"), ignore)]
async fn test_clipboard_simple_text() {
    run_clipboard_paste_test("Hello from the clipboard!").await;
}

#[tokio::test]
#[cfg_attr(not(feature = "real-injection-tests"), ignore)]
async fn test_clipboard_unicode_text() {
    run_clipboard_paste_test("Clipboard 🎤 and paste 🎤").await;
}

//--- Enigo (Typing) Tests ---

/// Helper to test the direct typing capability of the Enigo backend.
async fn run_enigo_typing_test(test_text: &str) {
    let env = TestEnvironment::current();
    if !env.can_run_real_tests() {
        eprintln!("Skipping enigo typing test: no display server found.");
        return;
    }

    let injector = EnigoInjector::new(Default::default());
    if !injector.is_available().await {
        println!("Skipping enigo typing test: backend is not available.");
        return;
    }

    let app = TestAppManager::launch_gtk_app().expect("Failed to launch GTK app.");
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Use the test-only helper to force typing instead of pasting.
    injector
        .type_text_directly(test_text)
        .await
        .unwrap_or_else(|e| panic!("Enigo typing failed for text '{}': {:?}", test_text, e));

    verify_injection(&app.output_file, test_text).unwrap_or_else(|e| {
        panic!(
            "Verification failed for enigo typing with text '{}': {}",
            test_text, e
        )
    });
}

#[tokio::test]
#[cfg_attr(not(feature = "real-injection-tests"), ignore)]
async fn test_enigo_typing_simple_text() {
    run_enigo_typing_test("Enigo types this text.").await;
}

#[tokio::test]
#[cfg_attr(not(feature = "real-injection-tests"), ignore)]
async fn test_enigo_typing_unicode_text() {
    // Note: Enigo's unicode support can be platform-dependent. This test will verify it.
    run_enigo_typing_test("Enigo 🎤 typing 🎤 unicode").await;
}

#[tokio::test]
#[cfg_attr(not(feature = "real-injection-tests"), ignore)]
async fn test_enigo_typing_special_chars() {
    run_enigo_typing_test("Enigo types\nnew lines and\ttabs.").await;
}

// TODO: Add tests for kdotool, combo injectors etc.
