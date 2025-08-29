use coldvox_app::vad::config::{UnifiedVadConfig, VadMode};
use coldvox_app::audio::vad_processor::{AudioFrame as VadFrame, VadProcessor};
use tokio::sync::{broadcast, mpsc};

#[tokio::test]
async fn vad_processor_silence_no_events_level3() {
    // Use Level3 to avoid ONNX model dependency in unit tests
    let mut cfg = UnifiedVadConfig::default();
    cfg.mode = VadMode::Level3;
    cfg.level3.enabled = true;
    cfg.frame_size_samples = 320; // Level3 default
    cfg.sample_rate_hz = 16_000;

    let (tx, _rx) = broadcast::channel::<VadFrame>(8);
    let (event_tx, mut event_rx) = mpsc::channel(8);
    let rx = tx.subscribe();

    let handle = VadProcessor::spawn(cfg, rx, event_tx, None).expect("spawn vad");

    // Send a few frames of silence at 16k/320-sample frames
    for i in 0..10u64 {
        let frame = VadFrame { data: vec![0i16; 320], timestamp_ms: i * 20 };
        let _ = tx.send(frame);
    }

    // Allow processing
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Drop sender to end the processor loop and wait for join
    drop(tx);
    handle.abort();

    // Ensure no events were produced
    assert!(event_rx.try_recv().is_err());
}

