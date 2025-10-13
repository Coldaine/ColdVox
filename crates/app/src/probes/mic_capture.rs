use crate::probes::MicCaptureThresholds;
use coldvox_telemetry::PipelineMetrics;
use std::sync::Arc;

use super::common::{LiveTestResult, TestContext, TestError, TestErrorKind};
use coldvox_audio::{AudioCaptureThread, AudioRingBuffer, FrameReader};
use coldvox_foundation::{AudioConfig, AudioError};
use serde_json::json;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::time::interval;

pub struct MicCaptureCheck;

impl MicCaptureCheck {
    pub async fn run(ctx: &TestContext) -> Result<LiveTestResult, TestError> {
        let device_name = ctx.device.clone();
        let duration = ctx.duration;

        let config = AudioConfig::default();

        // Prepare ring buffer and spawn capture thread
        // Use the same buffer size as the main runtime for consistency
        let rb = AudioRingBuffer::new(config.capture_buffer_samples);
        let (audio_producer, audio_consumer) = rb.split();
        let audio_producer = Arc::new(parking_lot::Mutex::new(audio_producer));
        let (capture_thread, dev_cfg, _config_rx, _device_event_rx) =
            AudioCaptureThread::spawn(config.clone(), audio_producer, device_name, false).map_err(
                |e| TestError {
                    kind: match e {
                        AudioError::DeviceNotFound { .. } => TestErrorKind::Device,
                        _ => TestErrorKind::Setup,
                    },
                    message: format!("Failed to create audio capture thread: {}", e),
                },
            )?;

        tokio::time::sleep(Duration::from_millis(200)).await; // Give the thread time to start

        // Create metrics for this test instance
        let metrics = Arc::new(PipelineMetrics::default());

        // Add optional logging of metrics every 30s
        let metrics_clone = metrics.clone();
        let log_handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                let capture_fps = metrics_clone.capture_fps.load(Ordering::Relaxed);
                let capture_fill = metrics_clone.capture_buffer_fill.load(Ordering::Relaxed);
                tracing::info!(
                        target: "mic_capture",
                "Capture FPS: {}, Capture Buffer Fill: {}%",
                capture_fps,
                        capture_fill
                    );
            }
        });

        let frames_captured = Arc::new(AtomicU64::new(0));
        let start_time = Instant::now();
        let timeout = tokio::time::sleep(duration);
        tokio::pin!(timeout);

        // Build a single reader for the duration of the test
        let mut reader = FrameReader::new(
            audio_consumer,
            dev_cfg.sample_rate,
            dev_cfg.channels,
            config.capture_buffer_samples,
            Some(metrics.clone()),
        );

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
        log_handle.abort();

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
        metrics.insert("device_sample_rate".to_string(), json!(dev_cfg.sample_rate));
        metrics.insert("device_channels".to_string(), json!(dev_cfg.channels));

        let default_thresholds = MicCaptureThresholds {
            max_drop_rate_error: Some(0.20),
            max_drop_rate_warn: Some(0.10),
            frames_per_sec_min: Some(1.0),
            frames_per_sec_max: Some(2000.0),
            watchdog_must_be_false: Some(false),
        };

        let thresholds = ctx
            .thresholds
            .as_ref()
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

pub fn evaluate_mic_capture(
    metrics: &HashMap<String, serde_json::Value>,
    thresholds: &MicCaptureThresholds,
) -> (bool, String) {
    let mut pass = true;
    let mut failures = vec![];

    let frames_per_sec = metrics
        .get("frames_per_sec")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let frames_captured = metrics
        .get("frames_captured")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let duration_secs = metrics
        .get("duration_secs")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    if let Some(min_fps) = thresholds.frames_per_sec_min {
        if frames_per_sec < min_fps {
            pass = false;
            failures.push(format!(
                "FPS {:.1} below minimum {:.1}",
                frames_per_sec, min_fps
            ));
        }
    }

    if let Some(max_fps) = thresholds.frames_per_sec_max {
        if frames_per_sec > max_fps {
            pass = false;
            failures.push(format!(
                "fps {:.1} exceeds maximum {:.1}",
                frames_per_sec, max_fps
            ));
        }
    }

    let notes = if failures.is_empty() {
        format!(
            "All checks passed. Captured {} frames in {:.1}s at {:.1} FPS",
            frames_captured, duration_secs, frames_per_sec
        )
    } else {
        failures.join("; ")
    };

    (pass, notes)
}
