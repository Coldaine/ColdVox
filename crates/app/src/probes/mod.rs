pub mod mic_capture;
pub mod vad_mic;
pub mod record_to_wav;
pub mod foundation;
pub mod common;

use mic_capture::MicCaptureCheck;
use vad_mic::VadFromMicCheck;
use record_to_wav::RecordToWav;
use foundation::FoundationHealth;

pub trait LiveTest {
    fn name(&self) -> &'static str;
    fn run(&mut self, ctx: &mut TestContext) -> Result<LiveTestResult, TestError>;
}

pub fn all_tests() -> Vec<Box<dyn LiveTest>> {
    vec![
        Box::new(MicCaptureCheck::default()),
        Box::new(VadFromMicCheck::default()),
        Box::new(RecordToWav::default()),
        Box::new(Foundation::FoundationHealth::default()),
    ]
}