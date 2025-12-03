use crate::injectors::atspi::AtspiInjector;
use crate::tests::real_injection::run_test;
use crate::TextInjector;
use std::time::Duration;

/// Helper function to run a complete injection and verification test for the AT-SPI backend.
async fn run_atspi_test(test_text: &str) {
    let injector = AtspiInjector::new(Default::default());
    if !injector.is_available().await {
        println!("Skipping AT-SPI test: backend is not available (is at-spi-bus-launcher running?).");
        return;
    }
    run_test(test_text, &injector).await;
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
    let long_text = "This is a long string designed to test the injection capabilities of the backend. ".repeat(50);
    assert!(long_text.len() > 1000);
    run_atspi_test(&long_text).await;
}

#[tokio::test]
async fn test_atspi_special_chars() {
    run_atspi_test("Line 1\nLine 2\twith a tab\nAnd some symbols: !@#$%^&*()_+").await;
}
