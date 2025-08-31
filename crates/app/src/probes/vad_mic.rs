use std::sync::Arc;
use coldvox_telemetry::pipeline_metrics::PipelineMetrics;

use super::common::{LiveTestResult, TestContext, TestError, TestErrorKind};
use coldvox_audio::capture::AudioCaptureThread;
use coldvox_audio::chunker::{AudioChunker, ChunkerConfig, ResamplerQuality};
use coldvox_audio::frame_reader::FrameReader;
use coldvox_audio::ring_buffer::AudioRingBuffer;
use coldvox_audio::chunker::AudioFrame as VadFrame;
use crate::audio::vad_processor::VadProcessor;
use coldvox_vad::types::VadEvent;
use coldvox_vad::config::{UnifiedVadConfig, VadMode};
use coldvox_foundation::error::AudioConfig;
use serde_json::json;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc};

#[derive(Debug)]
pub struct VadMicCheck;

impl VadMicCheck {
    pub async fn run(ctx: &TestContext) -> Result<LiveTestResult, TestError> {
        let device_name = ctx.device.clone();
        let duration = ctx.duration;

        let config = AudioConfig::default();

        // Prepare ring buffer and spawn capture thread
        let rb = AudioRingBuffer::new(16_384);
        let (audio_producer, audio_consumer) = rb.split();
    let (capture_thread, dev_cfg, _config_rx) = AudioCaptureThread::spawn(config, audio_producer, device_name).map_err(|e| TestError {
            kind: TestErrorKind::Setup,
            message: format!("Failed to create audio capture thread: {}", e),
        })?;

        tokio::time::sleep(Duration::from_millis(200)).await; // Give the thread time to start

        // Create metrics for this test instance
        let metrics = Arc::new(PipelineMetrics::default());

        // Periodic metrics logging every 30s
        let metrics_clone = metrics.clone();
        let log_handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                let cap_fps = metrics_clone.capture_fps.load(std::sync::atomic::Ordering::Relaxed);
                let chk_fps = metrics_clone.chunker_fps.load(std::sync::atomic::Ordering::Relaxed);
                let vad_fps = metrics_clone.vad_fps.load(std::sync::atomic::Ordering::Relaxed);
                let cap_fill = metrics_clone.capture_buffer_fill.load(std::sync::atomic::Ordering::Relaxed);
                let chk_fill = metrics_clone.chunker_buffer_fill.load(std::sync::atomic::Ordering::Relaxed);
                tracing::info!(
                    target: "vad_mic",
                    "FPS c:{} ch:{} vad:{} | Fill c:{}% ch:{}%",
                    cap_fps, chk_fps, vad_fps, cap_fill, chk_fill
                );
            }
        });

        // Set up VAD processing pipeline
        let (audio_tx, _) = broadcast::channel::<VadFrame>(200);
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
            16_384,
            Some(metrics.clone()),
        );
        let chunker = AudioChunker::new(frame_reader, audio_tx.clone(), chunker_cfg)
            .with_metrics(metrics.clone());
        let chunker_handle = chunker.spawn();

        let vad_cfg = UnifiedVadConfig {
            mode: VadMode::Silero,
            frame_size_samples: 512,
            sample_rate_hz: 16000,  // Silero requires 16kHz - resampler will handle conversion
            ..Default::default()
        };

        let vad_audio_rx = audio_tx.subscribe();
        let vad_handle = match VadProcessor::spawn(vad_cfg, vad_audio_rx, event_tx, Some(metrics.clone())) {
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
        let mut metrics = HashMap::new();
        metrics.insert("vad_events_count".to_string(), json!(vad_events.len()));
        metrics.insert("speech_segments".to_string(), json!(speech_segments));
        metrics.insert("total_speech_duration_ms".to_string(), json!(total_speech_duration_ms));
        metrics.insert("test_duration_secs".to_string(), json!(elapsed.as_secs_f64()));
    metrics.insert("device_sample_rate".to_string(), json!(dev_cfg.sample_rate));
    metrics.insert("device_channels".to_string(), json!(dev_cfg.channels));

        // Calculate speech ratio
        let speech_ratio = if elapsed.as_millis() > 0 {
            total_speech_duration_ms as f64 / elapsed.as_millis() as f64
        } else {
            0.0
        };
        metrics.insert("speech_ratio".to_string(), json!(speech_ratio));

        // Evaluate results
        let (pass, notes) = evaluate_vad_performance(&metrics, &vad_events);

        Ok(LiveTestResult {
            test: "vad_mic".to_string(),
            pass,
            metrics,
            notes: Some(notes),
            artifacts: vec![],
        })
    }
}

fn evaluate_vad_performance(metrics: &HashMap<String, serde_json::Value>, events: &[(u64, VadEvent)]) -> (bool, String) {
    let mut pass = true;
    let mut issues = Vec::new();

    let events_count = metrics.get("vad_events_count")
        .and_then(|v| v.as_u64()).unwrap_or(0);
    let speech_segments = metrics.get("speech_segments")
        .and_then(|v| v.as_u64()).unwrap_or(0);
    let speech_ratio = metrics.get("speech_ratio")
        .and_then(|v| v.as_f64()).unwrap_or(0.0);

    // Check for basic VAD functionality
    if events_count == 0 {
        pass = false;
        issues.push("No VAD events detected - VAD processor may not be working".to_string());
    }

    // Check for reasonable speech ratio (not too high or too low)
    if speech_ratio > 0.9 {
        pass = false;
        issues.push(format!("Speech ratio too high ({:.1}%) - may indicate over-sensitive VAD", speech_ratio * 100.0));
    } else if speech_ratio < 0.01 && events_count > 0 {
        issues.push(format!("Very low speech ratio ({:.3}%) - VAD may be too conservative", speech_ratio * 100.0));
    }

    // Check for balanced speech/silence events
    let speech_starts = events.iter().filter(|(_, e)| matches!(e, VadEvent::SpeechStart { .. })).count();
    let speech_ends = events.iter().filter(|(_, e)| matches!(e, VadEvent::SpeechEnd { .. })).count();

    if speech_starts != speech_ends {
        pass = false;
        issues.push(format!("Unbalanced VAD events: {} starts, {} ends", speech_starts, speech_ends));
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