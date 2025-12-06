
use async_trait::async_trait;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use tempfile::NamedTempFile;

use crate::{InjectionContext, InjectionResult, TextInjector};

// A mock TextInjector that writes to a temporary file instead of a real UI element.
pub struct MockTextInjector;

#[async_trait]
impl TextInjector for MockTextInjector {
    async fn inject_text(&self, text: &str, context: Option<&InjectionContext>) -> InjectionResult<()> {
        let output_file = context
            .and_then(|c| c.test_harness_output_file.as_ref())
            .expect("MockTextInjector requires a test_harness_output_file in the context");

        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(output_file)?;

        file.write_all(text.as_bytes())?;
        Ok(())
    }

    async fn is_available(&self) -> bool {
        true
    }

    fn backend_name(&self) -> &'static str {
        "mock"
    }

    fn backend_info(&self) -> Vec<(&'static str, String)> {
        vec![]
    }
}

// Manages the lifecycle of a mock application's resources (e.g., a temp file).
pub struct MockTestApp {
    pub output_file: PathBuf,
    // Keep the temp file handle to ensure it's deleted on drop
    _temp_file: NamedTempFile,
}

impl MockTestApp {
    // Creates a new mock application instance.
    pub fn new() -> Self {
        let temp_file = NamedTempFile::new().expect("Failed to create temp file for mock app");
        Self {
            output_file: temp_file.path().to_path_buf(),
            _temp_file: temp_file,
        }
    }
}

// A factory for creating MockTestApp instances.
pub struct MockTestAppManager;

impl MockTestAppManager {
    // "Launches" a new mock application.
    pub fn launch_mock_app() -> MockTestApp {
        MockTestApp::new()
    }
}
