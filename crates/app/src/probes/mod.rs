pub mod mic_capture;
pub mod vad_mic;
pub mod record_to_wav;
pub mod foundation;

use super::LiveTest;
use super::LiveTestResult;
use super::TestContext;
use super::TestError;

#[derive(Debug, PartialEq)]
pub struct LiveTestResult {
    pub metrics: std::collections::HashMap<String, String>,
    pub pass: bool,
    pub notes: String,
    pub artifacts: Vec<String>,
}

pub trait LiveTest {
    fn name() -> &'static str;
    fn run(ctx: &mut TestContext) -> Result<LiveTestResult, TestError>;
}