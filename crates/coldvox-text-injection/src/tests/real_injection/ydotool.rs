#![cfg(feature = "ydotool")]

use crate::ydotool_injector::YdotoolInjector;
use crate::TextInjector;

/// Helper function to run a complete injection and verification test for the ydotool backend.
/// This test involves setting the clipboard, as ydotool's primary injection method is paste.
async fn run_ydotool_test(test_text: &str) {
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

    // The inject_text for ydotool will trigger a paste (Ctrl+V).
    crate::tests::real_injection::run_test(test_text, &injector).await;
}

#[tokio::test]
async fn test_ydotool_simple_text() {
    run_ydotool_test("Hello from ydotool!").await;
}

#[tokio::test]
async fn test_ydotool_unicode_text() {
    run_ydotool_test("Hello ColdVox 🎤 测试 (via ydotool)").await;
}

#[tokio::test]
async fn test_ydotool_long_text() {
    let long_text = "This is a long string for ydotool. ".repeat(50);
    assert!(long_text.len() > 1000);
    run_ydotool_test(&long_text).await;
}

#[tokio::test]
async fn test_ydotool_special_chars() {
    run_ydotool_test("ydotool line 1\nydotool line 2\twith tab").await;
}
