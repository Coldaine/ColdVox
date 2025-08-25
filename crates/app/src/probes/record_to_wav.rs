use super::common::{LiveTestResult, TestContext, TestError, TestErrorKind, ensure_results_dir, write_result_json};
use super::LiveTest;
use coldvox_app::audio::{AudioCapture, AudioConfig, DeviceManager};
use hound::{WavSpec, WavWriter};
use serde_json::json;
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Default)]
pub struct RecordToWav;

impl LiveTest for RecordToWav {
    fn name(&self) -> &'static str { "RecordToWav" }
    fn run(&mut self, ctx: &mut TestContext) -> Result<LiveTestResult, TestError> {
        let mut metrics = HashMap::new();
        // Start capture
        let mut cap = AudioCapture::new(AudioConfig::default()).map_err(|e| TestError{ kind: TestErrorKind::Device, message: e.to_string() })?;
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        rt.block_on(async {
            cap.start(ctx.device.as_deref()).await
        }).map_err(|e| TestError{ kind: TestErrorKind::Device, message: e.to_string() })?;

        let rx = cap.get_receiver();
        // Prepare WAV in output dir
        let out_dir = ensure_results_dir(ctx.output_dir.as_deref()).map_err(|e| TestError{ kind: TestErrorKind::Internal, message: e.to_string() })?;
        let path = out_dir.join("record_10s.wav");
        let spec = WavSpec{ channels: 1, sample_rate: 16000, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
        let mut writer = WavWriter::create(&path, spec).map_err(|e| TestError{ kind: TestErrorKind::Internal, message: e.to_string() })?;

        let start = Instant::now();
        let dur = if ctx.duration.is_zero() { Duration::from_secs(10) } else { ctx.duration };
        let mut samples = 0usize;
        while start.elapsed() < dur {
            match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(f) => {
                    for s in f.samples { let _ = writer.write_sample(s); samples+=1; }
                }
                Err(_) => {}
            }
        }
        writer.finalize().ok();
        cap.stop();

        metrics.insert("samples_written".into(), json!(samples));
        let result = LiveTestResult { test: self.name().into(), pass: samples>0, metrics, notes: None, artifacts: vec![path.to_string_lossy().to_string()] };
        Ok(result)
    }
}
