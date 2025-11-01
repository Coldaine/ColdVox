use coldvox_telemetry::pipeline_metrics::PipelineMetrics;
use std::sync::Arc;

use super::common::{LiveTestResult, TestContext, TestError};
use crate::audio::vad_processor::VadProcessor;
use crate::probes::common::TestErrorKind;
use coldvox_audio::capture::AudioCaptureThread;
use coldvox_audio::chunker::{AudioChunker, ChunkerConfig, ResamplerQuality};
use coldvox_audio::frame_reader::FrameReader;
use coldvox_audio::ring_buffer::AudioRingBuffer;
use coldvox_audio::SharedAudioFrame;
use coldvox_foundation::error::AudioConfig;
use coldvox_vad::config::{UnifiedVadConfig, VadMode};
use coldvox_vad::types::VadEvent;
use serde_json::json;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc};

#[derive(Debug)]
pub struct VadMicCheck;

impl VadMicCheck {
    pub async fn run(ctx: &TestContext) -> Result<LiveTestResult, TestError> {
        // Device selection priority: env var > context > default detection
        let device_name = std::env::var("COLDVOX_TEST_DEVICE")
            .ok()
            .or_else(|| ctx.device.clone())
            .or_else(|| {
                tracing::warn!("No device specified, using default detection");
                None
            });
        let duration = ctx.duration;

        if let Some(ref dev) = device_name {
            tracing::info!("VAD Mic Test: Using device: {}", dev);
        } else {
            tracing::info!("VAD Mic Test: Using default device detection");
        }

        let config = AudioConfig::default();

        // Prepare ring buffer and spawn capture thread
        // Use the same buffer size as the main runtime for consistency
        let rb = AudioRingBuffer::new(config.capture_buffer_samples);
        let (audio_producer, audio_consumer) = rb.split();
        let audio_producer = Arc::new(parking_lot::Mutex::new(audio_producer));
        let (capture_thread, dev_cfg, device_cfg_rx, _device_event_rx) =
            AudioCaptureThread::spawn(config, audio_producer, device_name, false).map_err(|e| {
                TestError {
                    kind: TestErrorKind::Setup,
                    message: format!("Failed to create audio capture thread: {}", e),
                }
            })?;

        tokio::time::sleep(Duration::from_millis(200)).await; // Give the thread time to start

        // Create metrics for this test instance
        let metrics = Arc::new(PipelineMetrics::default());

        // Periodic metrics logging every 2s (short tests)
        let metrics_clone = metrics.clone();
        let log_handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(2));
            loop {
                interval.tick().await;
                let cap_fps = metrics_clone
                    .capture_fps
                    .load(std::sync::atomic::Ordering::Relaxed);
                let chk_fps = metrics_clone
                    .chunker_fps
                    .load(std::sync::atomic::Ordering::Relaxed);
                let vad_fps = metrics_clone
                    .vad_fps
                    .load(std::sync::atomic::Ordering::Relaxed);
                let cap_fill = metrics_clone
                    .capture_buffer_fill
                    .load(std::sync::atomic::Ordering::Relaxed);
                let chk_fill = metrics_clone
                    .chunker_buffer_fill
                    .load(std::sync::atomic::Ordering::Relaxed);
                tracing::info!(
                    target: "vad_mic",
                    "FPS c:{} ch:{} vad:{} | Fill c:{}% ch:{}%",
                    cap_fps, chk_fps, vad_fps, cap_fill, chk_fill
                );
            }
        });

        // Set up VAD processing pipeline
        let (audio_tx, _) = broadcast::channel::<SharedAudioFrame>(200);
        let (event_tx, mut event_rx) = mpsc::channel::<VadEvent>(100);

        let chunker_cfg = ChunkerConfig {
            frame_size_samples: 512,
            sample_rate_hz: 16_000,
            resampler_quality: ResamplerQuality::Balanced,
        };

        let frame_reader = FrameReader::new(
            audio_consumer,
            dev_cfg.sample_rate,
            dev_cfg.channels,
            config.capture_buffer_samples,
            Some(metrics.clone()),
        );
        let chunker = AudioChunker::new(frame_reader, audio_tx.clone(), chunker_cfg)
            .with_metrics(metrics.clone())
            .with_device_config(device_cfg_rx);
        let chunker_handle = chunker.spawn();

        let vad_cfg = UnifiedVadConfig {
            mode: VadMode::Silero,
            silero: coldvox_vad::config::SileroConfig {
                threshold: 0.2,
                ..Default::default()
            },
            frame_size_samples: 512,
            sample_rate_hz: 16000, // Silero requires 16kHz - resampler will handle conversion
        };

        let vad_audio_rx = audio_tx.subscribe();
        let vad_handle =
            match VadProcessor::spawn(vad_cfg, vad_audio_rx, event_tx, Some(metrics.clone())) {
                Ok(h) => h,
                Err(e) => {
                    capture_thread.stop();
                    chunker_handle.abort();
                    return Err(TestError {
                        kind: TestErrorKind::Internal,
                        message: format!("Failed to spawn VAD processor: {}", e),
                    });
                }
            };

        // Collect VAD events during the test
        let start_time = Instant::now();
        let mut vad_events = Vec::new();
        let mut speech_segments = 0;
        let mut total_speech_duration_ms = 0;

        let timeout = tokio::time::sleep(duration);
        tokio::pin!(timeout);

        loop {
            tokio::select! {
                Some(event) = event_rx.recv() => {
                    let timestamp_ms = start_time.elapsed().as_millis() as u64;
                    match &event {
                        VadEvent::SpeechStart { .. } => {
                            speech_segments += 1;
                        }
                        VadEvent::SpeechEnd { duration_ms, .. } => {
                            total_speech_duration_ms += *duration_ms;
                        }
                    }
                    vad_events.push((timestamp_ms, event));
                }
                _ = &mut timeout => break,
            }
        }

        // Clean up
        capture_thread.stop();
        chunker_handle.abort();
        vad_handle.abort();
        log_handle.abort();

        let elapsed = start_time.elapsed();

        // Calculate metrics
        let mut result_metrics = HashMap::new();
        result_metrics.insert("vad_events_count".to_string(), json!(vad_events.len()));
        result_metrics.insert("speech_segments".to_string(), json!(speech_segments));
        result_metrics.insert(
            "total_speech_duration_ms".to_string(),
            json!(total_speech_duration_ms),
        );
        result_metrics.insert(
            "test_duration_secs".to_string(),
            json!(elapsed.as_secs_f64()),
        );
        result_metrics.insert("device_sample_rate".to_string(), json!(dev_cfg.sample_rate));
        result_metrics.insert("device_channels".to_string(), json!(dev_cfg.channels));
        // Runtime FPS/buffer metrics snapshot
        result_metrics.insert(
            "capture_fps".to_string(),
            json!(metrics
                .capture_fps
                .load(std::sync::atomic::Ordering::Relaxed)),
        );
        result_metrics.insert(
            "chunker_fps".to_string(),
            json!(metrics
                .chunker_fps
                .load(std::sync::atomic::Ordering::Relaxed)),
        );
        result_metrics.insert(
            "vad_fps".to_string(),
            json!(metrics.vad_fps.load(std::sync::atomic::Ordering::Relaxed)),
        );
        result_metrics.insert(
            "capture_buffer_fill".to_string(),
            json!(metrics
                .capture_buffer_fill
                .load(std::sync::atomic::Ordering::Relaxed)),
        );
        result_metrics.insert(
            "chunker_buffer_fill".to_string(),
            json!(metrics
                .chunker_buffer_fill
                .load(std::sync::atomic::Ordering::Relaxed)),
        );

        // Calculate speech ratio
        let speech_ratio = if elapsed.as_millis() > 0 {
            total_speech_duration_ms as f64 / elapsed.as_millis() as f64
        } else {
            0.0
        };
        result_metrics.insert("speech_ratio".to_string(), json!(speech_ratio));

        // Evaluate results
        let (pass, notes) = evaluate_vad_performance(&result_metrics, &vad_events);

        Ok(LiveTestResult {
            test: "vad_mic".to_string(),
            pass,
            metrics: result_metrics,
            notes: Some(notes),
            artifacts: vec![],
        })
    }
}

fn evaluate_vad_performance(
    metrics: &HashMap<String, serde_json::Value>,
    events: &[(u64, VadEvent)],
) -> (bool, String) {
    let mut pass = true;
    let mut issues = Vec::new();

    let events_count = metrics
        .get("vad_events_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let speech_segments = metrics
        .get("speech_segments")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let speech_ratio = metrics
        .get("speech_ratio")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    // Check for basic VAD functionality
    if events_count == 0 {
        pass = false;
        issues.push("No VAD events detected - VAD processor may not be working".to_string());
    }

    // Check for reasonable speech ratio (not too high or too low)
    if speech_ratio > 0.9 {
        pass = false;
        issues.push(format!(
            "Speech ratio too high ({:.1}%) - may indicate over-sensitive VAD",
            speech_ratio * 100.0
        ));
    } else if speech_ratio < 0.01 && events_count > 0 {
        issues.push(format!(
            "Very low speech ratio ({:.3}%) - VAD may be too conservative",
            speech_ratio * 100.0
        ));
    }

    // Check for balanced speech/silence events
    let speech_starts = events
        .iter()
        .filter(|(_, e)| matches!(e, VadEvent::SpeechStart { .. }))
        .count();
    let speech_ends = events
        .iter()
        .filter(|(_, e)| matches!(e, VadEvent::SpeechEnd { .. }))
        .count();

    if speech_starts != speech_ends {
        pass = false;
        issues.push(format!(
            "Unbalanced VAD events: {} starts, {} ends",
            speech_starts, speech_ends
        ));
    }

    // Check for minimum speech segments if any speech detected
    if speech_segments == 0 && events_count > 0 {
        issues.push("VAD events detected but no complete speech segments".to_string());
    }

    let notes = if issues.is_empty() {
        format!(
            "VAD test completed successfully. Detected {} events, {} speech segments, {:.1}% speech ratio",
            events_count,
            speech_segments,
            speech_ratio * 100.0
        )
    } else {
        format!("Issues found: {}", issues.join("; "))
    };

    (pass, notes)
}
