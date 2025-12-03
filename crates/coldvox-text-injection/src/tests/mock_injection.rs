
use crate::tests::mock_harness::{MockTestAppManager, MockTextInjector};
use crate::tests::test_harness::verify_injection;
use crate::{InjectionContext, TextInjector};

// Helper function to run a complete mock injection and verification test.
async fn run_mock_injection_test(test_text: &str) {
    // 1. "Launch" the mock application. This creates a temporary file.
    let app = MockTestAppManager::launch_mock_app();

    // 2. Create the mock injector and the injection context.
    let injector = MockTextInjector;
    let context = InjectionContext {
        test_harness_output_file: Some(app.output_file.clone()),
        ..Default::default()
    };

    // 3. Perform the injection. This writes the text to the temp file.
    injector
        .inject_text(test_text, Some(&context))
        .await
        .expect("Mock injection failed");

    // 4. Verify the content of the temporary file.
    verify_injection(&app.output_file, test_text)
        .await
        .unwrap_or_else(|e| {
            panic!(
                "Verification failed for mock injection with text '{}': {}",
                test_text, e
            )
        });
}

#[tokio::test]
async fn test_mock_simple_text_injection() {
    run_mock_injection_test("Hello from the mock injector!").await;
}

#[tokio::test]
async fn test_mock_unicode_text_injection() {
    run_mock_injection_test("Hello ColdVox 🎤 mock test").await;
}

#[tokio::test]
async fn test_mock_long_text_injection() {
    let long_text =
        "This is a long string designed to test the mock injection capabilities. ".repeat(50);
    assert!(long_text.len() > 1000);
    run_mock_injection_test(&long_text).await;
}

#[tokio::test]
async fn test_mock_special_chars_injection() {
    run_mock_injection_test("Line 1\nLine 2\twith a tab\nAnd symbols: !@#$%^&*()_+").await;
}

#[tokio::test]
async fn test_mock_empty_string_injection() {
    run_mock_injection_test("").await;
}
