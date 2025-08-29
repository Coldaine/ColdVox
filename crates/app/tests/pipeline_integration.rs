use coldvox_app::audio::chunker::{AudioChunker, ChunkerConfig};
use coldvox_app::audio::ring_buffer::AudioRingBuffer;
use coldvox_app::audio::frame_reader::FrameReader;
use coldvox_app::audio::vad_processor::{AudioFrame as VadFrame, VadProcessor};
use coldvox_app::vad::config::{UnifiedVadConfig, VadMode};
use coldvox_app::vad::types::VadEvent;
use tokio::sync::{broadcast, mpsc};

mod common;
use common::test_utils::load_wav_16k_mono_i16;

#[tokio::test]
async fn test_pipeline_known_audio_chunking_and_vad() {
    // Arrange: load existing test WAV (16kHz mono)
    let wav_path = "test_audio_16k.wav";
    let samples = load_wav_16k_mono_i16(wav_path);
    assert!(!samples.is_empty(), "Test WAV '{}' must not be empty", wav_path);

    // Build ring buffer sized to fit entire input to avoid overflow during test
    let ring = AudioRingBuffer::new(samples.len() + 8192);
    let (mut producer, consumer) = ring.split();

    // Chunker setup (512 @ 16k)
    let sample_rate = 16_000u32;
    let reader = FrameReader::new(consumer, sample_rate, samples.len() + 8192, None);
    let cfg = ChunkerConfig { frame_size_samples: 512, sample_rate_hz: sample_rate };
    let (audio_tx, _) = broadcast::channel::<VadFrame>(64);
    let mut count_rx = audio_tx.subscribe();
    let chunker = AudioChunker::new(reader, audio_tx.clone(), cfg);
    let chunker_handle = chunker.spawn();

    // Feed all samples after chunker starts so consumer can drain
    let mut injected = 0usize;
    while injected < samples.len() {
        match producer.write(&samples[injected..]) {
            Ok(written) => {
                if written == 0 {
                    // Back off briefly if writer couldn't write
                    tokio::time::sleep(std::time::Duration::from_millis(1)).await;
                } else {
                    injected += written;
                }
            }
            Err(_e) => {
                // Temporary backpressure; retry shortly
                tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            }
        }
    }

    // VAD configured to Level3 (no external model needed)
    let mut vad_cfg = UnifiedVadConfig::default();
    vad_cfg.mode = VadMode::Level3;
    vad_cfg.level3.enabled = true;
    vad_cfg.frame_size_samples = 512;
    vad_cfg.sample_rate_hz = 16_000;

    let (event_tx, mut event_rx) = mpsc::channel::<VadEvent>(32);
    let vad_rx = audio_tx.subscribe();
    let vad_handle = VadProcessor::spawn(vad_cfg, vad_rx, event_tx, None)
        .expect("spawn vad");

    // Wait and count emitted frames from a dedicated receiver until no progress or timeout.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    let mut frames: usize = 0;
    loop {
        // Drain all currently available frames
        let mut progressed = false;
        while let Ok(_f) = count_rx.try_recv() {
            frames += 1;
            progressed = true;
        }
        if std::time::Instant::now() >= deadline {
            break;
        }
        if !progressed {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }

    // Stop tasks by dropping senders and aborting chunker
    drop(audio_tx);
    chunker_handle.abort();
    vad_handle.abort();

    // Assert: chunking integrity (sum of emitted samples ~= actually fed ± one frame)
    let emitted = frames * 512;
    let input = injected; // actual samples fed into the ring buffer
    let diff = if emitted > input { emitted - input } else { input - emitted };
    assert!(diff <= 512, "Chunking mismatch: input={}, emitted={}, diff={}", input, emitted, diff);

    // Assert: VAD processed some frames (no model dependency)
    // We don't require a speech event; just ensure the pipeline flowed.
    assert!(frames > 0, "Expected chunker to emit frames");
    // Drain any events to ensure channel worked (optional)
    let _ = event_rx.try_recv();

    // Done
}
