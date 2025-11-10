//! Basic wl-copy stdin piping test
//!
//! This test verifies that wl-copy stdin piping fix works correctly
//! without complex type handling that causes compilation issues.

use crate::injectors::clipboard::ClipboardInjector;
use crate::types::InjectionConfig;

use super::test_utils::{command_exists, is_wayland_environment, read_clipboard_with_wl_paste};

/// Test that wl-copy properly receives content via stdin
/// This is the core test for the stdin piping fix
#[tokio::test]
#[ignore] // Requires Wayland environment
async fn test_wl_copy_stdin_piping_basic() {
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
    let long_text =
        "This is a very long text designed to test that stdin piping works correctly. ".repeat(100);
    let test_cases = [
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
        &long_text[..5000], // Truncate to reasonable length
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

        // Verify that content was actually copied correctly
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

// helper functions provided by test_utils
