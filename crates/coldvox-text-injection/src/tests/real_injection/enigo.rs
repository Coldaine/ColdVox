#![cfg(feature = "enigo")]

use crate::enigo_injector::EnigoInjector;
use crate::tests::test_harness::{verify_injection, TestAppManager, TestEnvironment};
use crate::TextInjector;
use std::time::Duration;

/// Helper to test the direct typing capability of the Enigo backend.
async fn run_enigo_typing_test(test_text: &str) {
    let env = TestEnvironment::current();
    if !env.can_run_real_tests() {
        println!("Skipping enigo typing test: no display server found.");
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

    verify_injection(&app.output_file, test_text)
        .await
        .unwrap_or_else(|e| {
            panic!(
                "Verification failed for enigo typing with text '{}': {}",
                test_text, e
            )
        });
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
