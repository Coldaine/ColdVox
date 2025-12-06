#![cfg(all(feature = "wl_clipboard", feature = "enigo"))]

use crate::injectors::ClipboardPasteInjector;
use crate::TextInjector;

/// Helper to test clipboard injection followed by a paste action.
/// This simulates a realistic clipboard workflow.
async fn run_clipboard_paste_test(test_text: &str) {
    // This test requires both a clipboard manager and a paste mechanism.
    // We use ClipboardInjector (Wayland) and Enigo (cross-platform paste).
    let clipboard_paste = ClipboardPasteInjector::new(Default::default());
    if !clipboard_paste.is_available().await {
        println!("Skipping clipboard test: backend is not available (not on Wayland?).");
        return;
    }

    crate::tests::real_injection::run_test(test_text, &clipboard_paste).await;
}

#[tokio::test]
async fn test_clipboard_simple_text() {
    run_clipboard_paste_test("Hello from the clipboard!").await;
}

#[tokio::test]
async fn test_clipboard_unicode_text() {
    run_clipboard_paste_test("Clipboard 🎤 and paste 🎤").await;
}
