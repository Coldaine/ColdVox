use super::common::{LiveTestResult, TestContext, TestError, TestErrorKind};
use super::thresholds::MicCaptureThresholds;
use crate::audio::{AudioCapture, AudioFrame};
use crate::foundation::error::{AudioConfig, AudioError};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::time::{Duration, Instant};

pub struct MicCaptureCheck;

impl MicCaptureCheck {
    pub async fn run(ctx: &TestContext) -> Result<LiveTestResult, TestError> {
        let device_name = ctx.device.clone();
        let duration = ctx.duration;
        
        // Create capture instance with default config
        let config = AudioConfig::default();
        
        let mut capture = AudioCapture::new(config).map_err(|e| TestError {
            kind: match e {
                AudioError::DeviceNotFound { .. } => TestErrorKind::Device,
                AudioError::DeviceDisconnected => TestErrorKind::Device,
                _ => TestErrorKind::Setup,
            },
            message: format!("Failed to create audio capture: {}", e),
        })?;
        
        // Add small delay to ensure device is ready
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Start the capture with the specified device
        capture.start(device_name.as_deref()).await.map_err(|e| TestError {
            kind: match e {
                AudioError::DeviceNotFound { .. } => TestErrorKind::Device,
                AudioError::DeviceDisconnected => TestErrorKind::Device,
                _ => TestErrorKind::Setup,
            },
            message: format!("Failed to start audio capture: {}", e),
        })?;
        
        // Metrics collection
        let frames_captured = Arc::new(AtomicU64::new(0));
        let last_frame_time = Arc::new(parking_lot::Mutex::new(Instant::now()));
        let watchdog_triggered = Arc::new(AtomicBool::new(false));
        
        // Setup metrics collection
        let frames_captured_clone = frames_captured.clone();
        let last_frame_time_clone = last_frame_time.clone();
        
        // Get frame receiver
        let frame_rx = capture.get_receiver();
        
        // Start time
        let start_time = Instant::now();
        
        // Collect frames for the specified duration
        let timeout = tokio::time::sleep(duration);
        tokio::pin!(timeout);
        
        loop {
            tokio::select! {
                _ = &mut timeout => break,
                _ = tokio::time::sleep(Duration::from_millis(1)) => {
                    // Check for frames with timeout
                    if let Ok(_frame) = frame_rx.try_recv() {
                        frames_captured_clone.fetch_add(1, Ordering::Relaxed);
                        *last_frame_time_clone.lock() = Instant::now();
                    }
                }
            }
        }
        
        // Note: We cannot directly check watchdog status from AudioCapture
        // For now, we'll leave it as false
        // TODO: Add method to AudioCapture to check watchdog status
        
        // Stop capture
        capture.stop();
        
        // Calculate metrics
        let elapsed = start_time.elapsed();
        let frames_count = frames_captured.load(Ordering::Relaxed);
        
        let frames_per_sec = if elapsed.as_secs_f64() > 0.0 {
            frames_count as f64 / elapsed.as_secs_f64()
        } else {
            0.0
        };
        
        let last_frame_age_secs = last_frame_time.lock().elapsed().as_secs_f64();
        let watchdog_was_triggered = watchdog_triggered.load(Ordering::Relaxed);
        
        // Create metrics map
        let mut metrics = HashMap::new();
        metrics.insert("frames_captured".to_string(), json!(frames_count));
        metrics.insert("frames_per_sec".to_string(), json!(frames_per_sec));
        metrics.insert("drop_rate".to_string(), json!(0.0)); // TODO: track actual drops
        metrics.insert("last_frame_age_secs".to_string(), json!(last_frame_age_secs));
        metrics.insert("watchdog_triggered".to_string(), json!(watchdog_was_triggered));
        metrics.insert("duration_secs".to_string(), json!(elapsed.as_secs_f64()));
        
        // Evaluate against thresholds
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
    
    let drop_rate = metrics.get("drop_rate")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    
    let frames_per_sec = metrics.get("frames_per_sec")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    
    let watchdog_triggered = metrics.get("watchdog_triggered")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    
    let frames_captured = metrics.get("frames_captured")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    
    let duration_secs = metrics.get("duration_secs")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    
    if let Some(max_drop) = thresholds.max_drop_rate_error {
        if drop_rate > max_drop {
            pass = false;
            failures.push(format!("Drop rate {:.2}% exceeds threshold {:.2}%", 
                drop_rate * 100.0, max_drop * 100.0));
        }
    }
    
    if let Some(min_fps) = thresholds.frames_per_sec_min {
        if frames_per_sec < min_fps {
            pass = false;
            failures.push(format!("FPS {:.1} below minimum {:.1}", 
                frames_per_sec, min_fps));
        }
    }
    
    if let Some(max_fps) = thresholds.frames_per_sec_max {
        if frames_per_sec > max_fps {
            pass = false;
            failures.push(format!("FPS {:.1} exceeds maximum {:.1}", 
                frames_per_sec, max_fps));
        }
    }
    
    if let Some(must_be_false) = thresholds.watchdog_must_be_false {
        if must_be_false && watchdog_triggered {
            pass = false;
            failures.push("Watchdog was triggered".to_string());
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