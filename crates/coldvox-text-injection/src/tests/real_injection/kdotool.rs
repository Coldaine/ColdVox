#![cfg(feature = "kdotool")]

use crate::kdotool_injector::KdotoolInjector;
use crate::TextInjector;

/// Helper function to run a complete injection and verification test for the kdotool backend.
async fn run_kdotool_test(test_text: &str) {
    let injector = KdotoolInjector::new(Default::default());
    if !injector.is_available().await {
        println!("Skipping kdotool test: backend is not available (is kdotool running?).");
        return;
    }

    crate::tests::real_injection::run_test(test_text, &injector).await;
}

#[tokio::test]
async fn test_kdotool_simple_text() {
    run_kdotool_test("Hello from kdotool!").await;
}

#[tokio::test]
async fn test_kdotool_unicode_text() {
    run_kdotool_test("Hello ColdVox 🎤 kdotool").await;
}

#[tokio::test]
async fn test_kdotool_special_chars() {
    run_kdotool_test("kdotool line 1\nkdotool line 2\twith tab").await;
}
