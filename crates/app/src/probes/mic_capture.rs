use super::{LiveTest, TestContext, LiveTestResult, TestError};

#[derive(Debug, PartialEq)]
pub struct MicCaptureCheck {
    duration: u64,
}

impl MicCaptureCheck {
    pub fn new(duration: u64) -> Self {
        MicCaptureCheck { duration }
    }
}

impl LiveTest for MicCaptureCheck {
    fn name() -> &'static str {
        "mic_capture"
    }

    fn run(ctx: &mut TestContext) -> Result<LiveTestResult, TestError> {
        // Initialize audio capture with device selection from context
        let mut capture = match audio::AudioCapture::new(&ctx.device_selection) {
            Ok(c) => c,
            Err(e) => return Err(TestError::Device),
        };

        // Start capture for N seconds
        let start_time = std::time::Instant::now();
        let mut frames_captured = 0;
        let mut silent_frames = 0;
        let mut last_frame_age = 0;
        let mut watchdog_triggered = false;

        while start_time.elapsed().as_secs() < ctx.timeouts {
            if let Err(e) = capture.start() {
                return Err(TestError::Internal);
            }

            // Simulate frame processing
            let frame = capture.read_frame()?;
            frames_captured += 1;

            // Calculate metrics
            if frame.is_silent() {
                silent_frames += 1;
                last_frame_age = (start_time.elapsed().as_secs() * 1000) as u64;
            }

            // Check watchdog
            if capture.watchdog_triggered() {
                watchdog_triggered = true;
            }
        }

        // Calculate metrics
        let frames_per_sec = frames_captured as f64 / ctx.timeouts as f64;
        let drop_rate = (silent_frames as f64 / frames_captured as f64) * 100.0;

        // Evaluate thresholds
        let pass = !ctx.thresholds
            .mic_capture
            .check(&[
                (&format!("drop_rate {}", drop_rate), ctx.thresholds.mic_capture.max_drop_rate),
                (&format!("frames_per_sec {}", frames_per_sec), ctx.thresholds.mic_capture.frames_per_sec),
                ("watchdog_triggered", ctx.thresholds.mic_capture.watchdog_must_be_false),
            ]);

        Ok(LiveTestResult {
            metrics: {
                let mut metrics = std::collections::HashMap::new();
                metrics.insert("frames_captured".to_string(), frames_captured.to_string());
                metrics.insert("drop_rate".to_string(), drop_rate.to_string());
                metrics.insert("frames_per_sec".to_string(), frames_per_sec.to_string());
                metrics.insert("watchdog_triggered".to_string(), watchdog_triggered.to_string());
                metrics
            },
            pass,
            notes: if pass {
                "OK".to_string()
            } else {
                "Thresholds violated".to_string()
            },
            artifacts: vec![],
        })
    }
}