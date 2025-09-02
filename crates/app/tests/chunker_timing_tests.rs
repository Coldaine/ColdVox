use coldvox_app::audio::{
    AudioChunker, AudioFrame as VadFrame, AudioRingBuffer, ChunkerConfig, FrameReader,
    ResamplerQuality,
};
use coldvox_app::telemetry::pipeline_metrics::PipelineMetrics;
use std::sync::Arc;
use tokio::sync::broadcast;

mod common;
use common::test_utils::feed_samples_to_ring_buffer;

#[tokio::test]
async fn chunker_timestamps_are_32ms_apart_at_16k() {
    let metrics = Arc::new(PipelineMetrics::default());
    let rb_capacity = 16_384;
    let ring = AudioRingBuffer::new(rb_capacity);
    let (mut prod, cons) = ring.split();

    let reader = FrameReader::new(cons, 16_000, 1, rb_capacity, Some(metrics.clone()));
    let cfg = ChunkerConfig {
        frame_size_samples: 512,
        sample_rate_hz: 16_000,
        resampler_quality: ResamplerQuality::Balanced,
    };
    let (tx, _) = broadcast::channel::<VadFrame>(64);
    let mut rx = tx.subscribe();
    let chunker = AudioChunker::new(reader, tx.clone(), cfg).with_metrics(metrics.clone());
    let handle = chunker.spawn();

    // Feed about 10 frames worth of audio (5120 samples)
    let input = vec![1i16; 512 * 10];
    feed_samples_to_ring_buffer(&mut prod, &input, 1024);

    // Collect a few frames and verify monotonic 32ms timestamps
    let mut got = Vec::new();
    let mut attempts = 0;
    while got.len() < 5 && attempts < 50 {
        if let Ok(frame) = rx.try_recv() {
            // Convert Instant to relative ms for comparison
            got.push(frame.timestamp.elapsed().as_millis() as u64);
        } else {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            attempts += 1;
        }
    }

    handle.abort();
    assert!(
        got.len() >= 3,
        "expected at least 3 frames, got {}",
        got.len()
    );
    for w in got.windows(2) {
        let delta = w[1] - w[0];
        assert!(
            (delta as i64 - 32).abs() <= 5,
            "timestamp delta ~32ms, got {}",
            delta
        );
    }
}
