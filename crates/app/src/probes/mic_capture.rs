use super::common::{LiveTestResult, TestContext, TestError, TestErrorKind};
use super::thresholds::MicCaptureThresholds;
use crate::audio::capture::AudioCaptureThread;
use crate::audio::frame_reader::FrameReader;
use crate::audio::ring_buffer::AudioRingBuffer;
use crate::foundation::error::{AudioConfig, AudioError};
use serde_json::json;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub struct MicCaptureCheck;

impl MicCaptureCheck {
    pub async fn run(ctx: &TestContext) -> Result<LiveTestResult, TestError> {
        let device_name = ctx.device.clone();
        let duration = ctx.duration;

        let config = AudioConfig::default();

    // Prepare ring buffer and spawn capture thread
    let rb = AudioRingBuffer::new(16_384);
    let (audio_producer, audio_consumer) = rb.split();
    let (capture_thread, _sample_rate) = AudioCaptureThread::spawn(config, audio_producer, device_name).map_err(|e| TestError {
            kind: match e {
                AudioError::DeviceNotFound { .. } => TestErrorKind::Device,
                _ => TestErrorKind::Setup,
            },
            message: format!("Failed to create audio capture thread: {}", e),
        })?;

        tokio::time::sleep(Duration::from_millis(200)).await; // Give the thread time to start

        let frames_captured = Arc::new(AtomicU64::new(0));
        let start_time = Instant::now();
        let timeout = tokio::time::sleep(duration);
        tokio::pin!(timeout);

    // Build a single reader for the duration of the test
    let mut reader = FrameReader::new(audio_consumer, 16_000);

        loop {
            tokio::select! {
                _ = &mut timeout => break,
                _ = tokio::time::sleep(Duration::from_millis(10)) => {
                    if let Some(_frame) = reader.read_frame(4096) {
                        frames_captured.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        }

        capture_thread.stop();

        let elapsed = start_time.elapsed();
        let frames_count = frames_captured.load(Ordering::Relaxed);

        let frames_per_sec = if elapsed.as_secs_f64() > 0.0 {
            frames_count as f64 / elapsed.as_secs_f64()
        } else {
            0.0
        };

        let mut metrics = HashMap::new();
        metrics.insert("frames_captured".to_string(), json!(frames_count));
        metrics.insert("frames_per_sec".to_string(), json!(frames_per_sec));
        metrics.insert("duration_secs".to_string(), json!(elapsed.as_secs_f64()));

        let default_thresholds = MicCaptureThresholds {
            max_drop_rate_error: Some(0.20),
            max_drop_rate_warn: Some(0.10),
            frames_per_sec_min: Some(1.0),
            frames_per_sec_max: Some(2000.0),
            watchdog_must_be_false: Some(false),
        };

        let thresholds = ctx.thresholds.as_ref()
            .map(|t| &t.mic_capture)
            .unwrap_or(&default_thresholds);

        let (pass, notes) = evaluate_mic_capture(&metrics, thresholds);

        Ok(LiveTestResult {
            test: "mic_capture".to_string(),
            pass,
            metrics,
            notes: Some(notes),
            artifacts: vec![],
        })
    }
}

pub fn evaluate_mic_capture(metrics: &HashMap<String, serde_json::Value>, thresholds: &MicCaptureThresholds) -> (bool, String) {
    let mut pass = true;
    let mut failures = vec![];

    let frames_per_sec = metrics.get("frames_per_sec").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let frames_captured = metrics.get("frames_captured").and_then(|v| v.as_u64()).unwrap_or(0);
    let duration_secs = metrics.get("duration_secs").and_then(|v| v.as_f64()).unwrap_or(0.0);

    if let Some(min_fps) = thresholds.frames_per_sec_min {
        if frames_per_sec < min_fps {
            pass = false;
            failures.push(format!("FPS {:.1} below minimum {:.1}", frames_per_sec, min_fps));
        }
    }

    if let Some(max_fps) = thresholds.frames_per_sec_max {
        if frames_per_sec > max_fps {
            pass = false;
            failures.push(format!("FPS {:.1} exceeds maximum {:.1}", frames_per_sec, max_fps));
        }
    }

    let notes = if failures.is_empty() {
        format!("All checks passed. Captured {} frames in {:.1}s at {:.1} FPS",
            frames_captured, duration_secs, frames_per_sec)
    } else {
        failures.join("; ")
    };

    (pass, notes)
}
