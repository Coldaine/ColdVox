//! # wl-copy stdin piping test
//!
//! This module specifically tests the wl-copy stdin piping fix to ensure
//! clipboard content is properly piped to stdin instead of passed as command-line arguments.
//!
//! To run this test, use the following command:
//! `cargo test -p coldvox-text-injection --features wl_clipboard test_wl_copy_stdin_piping`

use crate::injectors::clipboard::ClipboardInjector;
use crate::types::{InjectionConfig, InjectionContext};
use std::process::Command;
use std::time::Duration;

use super::test_utils::{
    command_exists, is_wayland_environment, read_clipboard_with_wl_paste,
    read_clipboard_with_wl_paste_with_timeout,
};

/// Test that wl-copy properly receives content via stdin
/// This is the core test for the stdin piping fix
#[cfg(all(unix, feature = "wl_clipboard"))]
#[tokio::test]
#[ignore] // Requires Wayland environment
async fn test_wl_copy_stdin_piping() {
    // Skip if not on Wayland
    if !is_wayland_environment() {
        println!("Skipping wl-copy test: Not running on Wayland");
        return;
    }

    // Skip if wl-copy is not available
    if !command_exists("wl-copy") {
        println!("Skipping wl-copy test: wl-copy command not found");
        return;
    }

    let config = InjectionConfig::default();
    let injector = ClipboardInjector::new(config);

    // Test cases that would fail with command-line argument approach
    let long_text_base =
        "This is a very long text designed to test that the stdin piping works correctly. ";
    let long_text = long_text_base.repeat(100);

    let test_cases = vec![
        // Simple text
        "Hello from wl-copy stdin test!",
        // Text with special characters that would break command line
        "Text with \"quotes\" and 'apostrophes'",
        // Text with newlines that would be truncated in command line
        "Line 1\nLine 2\nLine 3",
        // Text with shell metacharacters
        "Text with $VAR and && operators; | pipes; < redirects",
        // Unicode text
        "Unicode test: 🎤 ColdVox 测试 🚀",
        // Long text that would exceed command line limits
        &long_text[..],
    ];

    for (i, test_text) in test_cases.iter().enumerate() {
        println!("Test case {}: {} chars", i + 1, test_text.len());

        // Write to clipboard using the fixed implementation
        let result = injector
            .write_clipboard(test_text.as_bytes(), "text/plain")
            .await;

        assert!(
            result.is_ok(),
            "Failed to write clipboard for test case {}: {:?}",
            i + 1,
            result
        );

        // Verify the content was actually copied correctly
        let clipboard_content = read_clipboard_with_wl_paste()
            .await
            .expect("Failed to read clipboard with wl-paste");
        assert_eq!(
            clipboard_content,
            *test_text,
            "Clipboard content mismatch for test case {}",
            i + 1
        );

        println!("✅ Test case {} passed", i + 1);
    }
}

// Fallback stub on non-Unix or when wl_clipboard feature is disabled
#[cfg(not(all(unix, feature = "wl_clipboard")))]
#[test]
fn test_wl_copy_stdin_piping() {
    eprintln!("Skipping wl-copy stdin piping test: not on Unix or wl_clipboard feature disabled",);
}

/// Test clipboard backup and restore functionality
/// This ensures the complete injection workflow works
#[tokio::test]
#[ignore] // Requires Wayland environment
async fn test_wl_copy_clipboard_backup_restore() {
    // Skip if not on Wayland or wl-copy not available
    if !is_wayland_environment() || !command_exists("wl-copy") {
        println!("Skipping clipboard backup/restore test: Requirements not met");
        return;
    }

    let config = InjectionConfig::default();
    let injector = ClipboardInjector::new(config);
    let context = InjectionContext::default();

    // Set initial clipboard content
    let original_content = "Original clipboard content before test";
    let _ = Command::new("wl-copy")
        .arg(original_content)
        .status()
        .expect("Failed to set initial clipboard");

    // Wait for clipboard to settle
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Test injection text
    let injection_text = "Test injection content 🎤";

    // Perform injection (this should backup, inject, then restore)
    let result = injector.inject(injection_text, &context).await;
    assert!(result.is_ok(), "Injection failed: {:?}", result);

    // Wait for clipboard restore
    tokio::time::sleep(Duration::from_millis(600)).await;

    // Verify clipboard was restored
    let restored_content = read_clipboard_with_wl_paste()
        .await
        .expect("Failed to read clipboard with wl-paste");
    assert_eq!(
        restored_content, original_content,
        "Clipboard not restored correctly"
    );

    println!("✅ Clipboard backup/restore test passed");
}

/// Test timeout handling for wl-copy operations
#[tokio::test]
#[ignore] // Requires Wayland environment
async fn test_wl_copy_timeout_handling() {
    // Skip if not on Wayland or wl-copy not available
    if !is_wayland_environment() || !command_exists("wl-copy") {
        println!("Skipping wl-copy timeout test: Requirements not met");
        return;
    }

    // Create config with very short timeout to force timeout
    let mut config = InjectionConfig::default();
    config.per_method_timeout_ms = 10; // Very short timeout
    config.paste_action_timeout_ms = 10; // Very short timeout

    let injector = ClipboardInjector::new(config);

    // Test with content that might take time to process
    let large_content = "Large content ".repeat(10000);

    let result = injector
        .write_clipboard(large_content.as_bytes(), "text/plain")
        .await;

    // Should fail due to timeout, but not hang
    assert!(
        result.is_err(),
        "Expected timeout error, but operation succeeded"
    );

    let err_string = result.unwrap_err().to_string();
    if err_string.contains("Timeout") {
        println!("✅ Timeout handling works correctly");
    } else {
        println!(
            "⚠️  Got different error than expected timeout: {}",
            err_string
        );
    }
}

/// Test error handling when wl-copy fails
#[tokio::test]
#[ignore] // Requires Wayland environment
async fn test_wl_copy_error_handling() {
    // Skip if not on Wayland
    if !is_wayland_environment() {
        println!("Skipping wl-copy error test: Not running on Wayland");
        return;
    }

    // We can't easily make wl-copy fail in a controlled way
    // but we can test that the error handling path doesn't panic
    let config = InjectionConfig::default();
    let injector = ClipboardInjector::new(config);

    // Try to write a very large amount of data that might cause issues
    let huge_content = "x".repeat(100_000_000); // 100MB

    let result = injector
        .write_clipboard(huge_content.as_bytes(), "text/plain")
        .await;

    // This might succeed or fail, but shouldn't panic
    match result {
        Ok(_) => println!("✅ Large content handled successfully"),
        Err(e) => println!("✅ Error handled gracefully: {:?}", e),
    }
}

/// Test that the fix handles edge cases correctly
#[tokio::test]
#[ignore] // Requires Wayland environment
async fn test_wl_copy_edge_cases() {
    // Skip if not on Wayland or wl-copy not available
    if !is_wayland_environment() || !command_exists("wl-copy") {
        println!("Skipping wl-copy edge cases test: Requirements not met");
        return;
    }

    let config = InjectionConfig::default();
    let injector = ClipboardInjector::new(config);

    // Test empty string
    let result = injector.write_clipboard(b"", "text/plain").await;
    assert!(result.is_ok(), "Empty string should work");

    // Test null bytes (if supported)
    let null_content = b"Text with \0 null byte";
    let result = injector.write_clipboard(null_content, "text/plain").await;
    // This might fail, but shouldn't panic
    println!("Null byte test result: {:?}", result);

    // Test very long single line
    let long_line = "x".repeat(100_000);
    let result = injector
        .write_clipboard(long_line.as_bytes(), "text/plain")
        .await;
    assert!(
        result.is_ok(),
        "Very long line should work with stdin piping"
    );

    println!("✅ Edge cases handled correctly");
}

// helper functions provided by test_utils
