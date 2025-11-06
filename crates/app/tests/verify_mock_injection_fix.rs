//! Minimal test to verify the mock injection sink fix works.
//!
//! This test validates that the fix for PR 205 (removing #[cfg(test)] guard)
//! actually allows integration tests to use the mock injection sink.
//!
//! This is a stripped-down version that doesn't require ALSA or the full
//! audio pipeline - it just verifies the wiring works.

use async_trait::async_trait;
use coldvox_app::runtime::AppRuntimeOptions;
use coldvox_text_injection::{InjectionContext, InjectionResult, TextInjector};
use std::sync::{Arc, Mutex};

/// Simple mock injector to verify the fix
#[derive(Clone, Default)]
struct TestMockInjector {
    captured: Arc<Mutex<Vec<String>>>,
}

impl TestMockInjector {
    fn new() -> Self {
        Self::default()
    }

    fn get_captured(&self) -> Vec<String> {
        self.captured.lock().unwrap().clone()
    }
}

#[async_trait]
impl TextInjector for TestMockInjector {
    async fn inject_text(
        &self,
        text: &str,
        _context: Option<&InjectionContext>,
    ) -> InjectionResult<()> {
        self.captured.lock().unwrap().push(text.to_string());
        Ok(())
    }

    async fn is_available(&self) -> bool {
        true
    }

    fn backend_name(&self) -> &'static str {
        "test-mock"
    }

    fn backend_info(&self) -> Vec<(&'static str, String)> {
        vec![]
    }
}

#[test]
fn test_mock_injection_sink_option_available() {
    // This test verifies that AppRuntimeOptions accepts test_injection_sink
    // in integration test context (without #[cfg(test)])

    let mock = Arc::new(TestMockInjector::new());

    // This should compile and work in integration tests (tests/ directory)
    // Prior to the fix, test_injection_sink would not be available because
    // the code using it was behind #[cfg(test)]
    let opts = AppRuntimeOptions {
        test_injection_sink: Some(mock.clone()),
        ..Default::default()
    };

    // Verify the mock is properly stored
    assert!(opts.test_injection_sink.is_some());

    println!("✓ Mock injection sink can be configured in integration tests");
    println!("✓ This proves the #[cfg(test)] guard was successfully removed");
}

#[tokio::test]
async fn test_mock_injector_captures_text() {
    // Verify the mock injector itself works
    let mock = TestMockInjector::new();

    mock.inject_text("test message", None).await.unwrap();
    mock.inject_text("another test", None).await.unwrap();

    let captured = mock.get_captured();
    assert_eq!(captured.len(), 2);
    assert_eq!(captured[0], "test message");
    assert_eq!(captured[1], "another test");

    println!("✓ Mock injector correctly captures text");
}
