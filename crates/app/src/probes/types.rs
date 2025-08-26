use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[async_trait::async_trait]
pub trait LiveTest: Send {
    fn name(&self) -> &'static str;
    async fn run(&mut self, ctx: &mut TestContext) -> Result<LiveTestResult, TestError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveTestResult {
    pub pass: bool,
    pub metrics: serde_json::Value,
    pub notes: Option<String>,
    pub artifacts: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TestError {
    pub kind: TestErrorKind,
    pub message: String,
}

#[derive(Debug, Clone)]
pub enum TestErrorKind {
    Setup,
    Device,
    Permission,
    Timeout,
    Internal,
}

#[derive(Debug, Clone)]
pub struct TestContext {
    pub duration_secs: u64,
    pub device: Option<String>,
    pub output_dir: PathBuf,
    pub thresholds: Option<Thresholds>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Thresholds {
    pub mic_capture: Option<MicCaptureThresholds>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MicCaptureThresholds {
    pub max_drop_rate: f64,
    pub frames_per_sec_min: f64,
    pub frames_per_sec_max: f64,
    pub watchdog_must_be_false: bool,
}

impl Default for MicCaptureThresholds {
    fn default() -> Self {
        Self {
            max_drop_rate: 0.20,
            frames_per_sec_min: 1.0,
            frames_per_sec_max: 2000.0,
            watchdog_must_be_false: false,
        }
    }
}