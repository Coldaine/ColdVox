use coldvox_app::audio::chunker::{AudioChunker, ChunkerConfig};
use coldvox_app::audio::frame_reader::FrameReader;
use coldvox_app::audio::ring_buffer::AudioRingBuffer;
use coldvox_app::audio::vad_processor::AudioFrame as VadFrame;
use coldvox_app::telemetry::pipeline_metrics::PipelineMetrics;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::sync::broadcast;

mod common;
use common::test_utils::{feed_samples_to_ring_buffer, load_wav_16k_mono_i16};

#[tokio::test]
async fn test_chunker_emits_frames_for_known_audio() {
    // Arrange: load existing test WAV (16kHz mono)
    let wav_path = "test_audio_16k.wav";
    let samples = load_wav_16k_mono_i16(wav_path);
    assert!(
        !samples.is_empty(),
        "Test WAV '{}' must not be empty",
        wav_path
    );

    // Build ring buffer sized to fit entire input to avoid overflow during test
    let rb_capacity = samples.len() + 8192;
    let ring = AudioRingBuffer::new(rb_capacity);
    let (mut producer, consumer) = ring.split();

    // Setup metrics
    let metrics = Arc::new(PipelineMetrics::default());

    // Chunker setup (512 @ 16k)
    let sample_rate = 16_000u32;
    let reader = FrameReader::new(consumer, sample_rate, 1, rb_capacity, Some(metrics.clone()));
    let cfg = ChunkerConfig {
        frame_size_samples: 512,
        sample_rate_hz: sample_rate,
        resampler_quality: coldvox_app::audio::chunker::ResamplerQuality::Balanced,
    };
    let (audio_tx, _) = broadcast::channel::<VadFrame>(64);
    let chunker = AudioChunker::new(reader, audio_tx.clone(), cfg).with_metrics(metrics.clone());
    let chunker_handle = chunker.spawn();

    // Feed all samples after chunker starts so consumer can drain
    let written = feed_samples_to_ring_buffer(&mut producer, &samples, 1024);

    // Wait for the chunker to drain most of the input.
    let mut waited_ms = 0u64;
    let mut last_frames = 0usize;
    let mut stall_ms = 0u64;
    loop {
        let frames = metrics.chunker_frames.load(Ordering::Relaxed) as usize;
        if frames != last_frames {
            last_frames = frames;
            stall_ms = 0;
        } else {
            stall_ms += 20;
        }
        let emitted = frames * 512;
        if emitted + 512 >= written || stall_ms >= 300 || waited_ms >= 2000 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        waited_ms += 20;
    }

    // Stop tasks by dropping sender and aborting chunker
    drop(audio_tx);
    chunker_handle.abort();

    // Assert: chunking produced a sensible amount of data for the input
    let frames = metrics.chunker_frames.load(Ordering::Relaxed) as usize;
    assert!(frames > 0, "Expected chunker to emit frames");
    let emitted = frames * 512;
    let input = written; // actual samples fed into the ring buffer
    assert!(
        emitted <= input,
        "Emitted {} should not exceed input {}",
        emitted,
        input
    );
    assert!(
        emitted >= input / 2,
        "Emitted {} unexpectedly low vs input {} (frames={})",
        emitted,
        input,
        frames
    );

    // No VAD in this test
}
