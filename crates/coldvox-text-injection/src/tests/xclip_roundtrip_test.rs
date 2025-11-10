//! X11 clipboard roundtrip tests exercised through xclip.

#![cfg(all(test, unix, not(target_os = "macos")))]

use std::time::Duration;

use crate::injectors::clipboard::ClipboardInjector;
use crate::types::InjectionConfig;

use super::test_utils::{command_exists, is_x11_environment, read_clipboard_with_xclip};

/// Verifies that ClipboardInjector::write_clipboard seeds the X11 clipboard via xclip.
#[tokio::test]
#[ignore] // Requires DISPLAY + xclip binary
async fn test_xclip_roundtrip_write_path() {
    if !is_x11_environment() {
        println!("Skipping xclip test: DISPLAY/X11 not detected");
        return;
    }

    if !command_exists("xclip") {
        println!("Skipping xclip test: xclip command not found");
        return;
    }

    let injector = ClipboardInjector::new(InjectionConfig::default());
    let payload = format!("X11 clipboard roundtrip {:?}", std::time::SystemTime::now());

    injector
        .write_clipboard(payload.as_bytes(), "text/plain")
        .await
        .expect("Failed to seed clipboard via ClipboardInjector");

    // Give xclip time to drain stdin and own the clipboard
    tokio::time::sleep(Duration::from_millis(50)).await;

    let clipboard_content = read_clipboard_with_xclip()
        .await
        .expect("Failed to read clipboard content via xclip");

    assert_eq!(
        payload, clipboard_content,
        "Clipboard read via xclip did not match payload"
    );

    println!(
        "✅ X11 clipboard roundtrip succeeded ({} chars)",
        payload.len()
    );
}
