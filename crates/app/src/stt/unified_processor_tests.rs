#![cfg(test)]

use super::*;
use crate::stt::plugin_manager::SttPluginManager;
use coldvox_audio::AudioFrame;
use coldvox_stt::SttProcessingMode;
use coldvox_vad::VadEvent;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::time::Instant;

#[tokio::test]
async fn test_mode_switch_during_idle() {
    // Setup processor in batch mode
    let (stt_mode_tx, mut stt_mode_rx) = mpsc::channel(1);
    let (event_tx, mut event_rx) = mpsc::channel(100);
    let (vad_tx, mut vad_rx) = mpsc::channel(100);
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

    let plugin_manager = Arc::new(RwLock::new(SttPluginManager::new()));
    let mode = Arc::new(RwLock::new(SttProcessingMode::Batch));
    let config = TranscriptionConfig::default();

    let mut processor = UnifiedSttProcessor::new(
        broadcast::channel(10).1, // dummy audio rx
        vad_rx,
        event_tx,
        stt_mode_rx,
        plugin_manager,
        mode.clone(),
        config,
        shutdown_rx,
    );

    // Send mode change request
    let switch_handle = tokio::spawn(async move {
        stt_mode_tx.send(SttProcessingMode::Streaming).await.unwrap();
    });

    // Process some VAD events to advance the processor
    let mut vad_handle = tokio::spawn(async move {
        // Send some dummy VAD events
        vad_tx.send(VadEvent::SpeechStart { timestamp_ms: 1000, energy_db: -20.0 }).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        vad_tx.send(VadEvent::SpeechEnd { timestamp_ms: 2000, duration_ms: 1000, energy_db: -20.0 }).await.unwrap();
    });

    // Run processor briefly
    let processor_handle = tokio::spawn(async move {
        processor.run().await;
    });

    // Wait for mode change to complete
    switch_handle.await.unwrap();
    vad_handle.abort();
    processor_handle.abort().await.unwrap();

    // Verify mode was switched
    let final_mode = mode.read().await;
    assert_eq!(*final_mode, SttProcessingMode::Streaming);
}

#[tokio::test]
async fn test_mode_switch_during_speech() {
    // Setup processor in batch mode
    let (stt_mode_tx, mut stt_mode_rx) = mpsc::channel(1);
    let (event_tx, mut event_rx) = mpsc::channel(100);
    let (vad_tx, mut vad_rx) = mpsc::channel(100);
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

    let plugin_manager = Arc::new(RwLock::new(SttPluginManager::new()));
    let mode = Arc::new(RwLock::new(SttProcessingMode::Batch));
    let config = TranscriptionConfig::default();

    let mut processor = UnifiedSttProcessor::new(
        broadcast::channel(10).1, // dummy audio rx
        vad_rx,
        event_tx,
        stt_mode_rx,
        plugin_manager,
        mode.clone(),
        config,
        shutdown_rx,
    );

    // Start speech
    let mut vad_handle = tokio::spawn(async move {
        vad_tx.send(VadEvent::SpeechStart { timestamp_ms: 1000, energy_db: -20.0 }).await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
    });

    // Send some audio frames during speech
    let audio_tx = broadcast::channel(10).0;
    let mut audio_handle = tokio::spawn(async move {
        for i in 0..3 {
            let frame = AudioFrame {
                samples: vec![0.0; 512],
                sample_rate: 16000,
                timestamp: Instant::now(),
            };
            let _ = audio_tx.send(frame);
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
    });

    // Wait a bit for processing to start
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Send mode change request during speech
    let switch_handle = tokio::spawn(async move {
        stt_mode_tx.send(SttProcessingMode::Streaming).await.unwrap();
    });

    // Run processor briefly
    let processor_handle = tokio::spawn(async move {
        processor.run().await;
    });

    // Check for interruption event
    let mut interruption_received = false;
    for _ in 0..10 {
        if let Ok(event) = event_rx.try_recv() {
            if matches!(event, TranscriptionEvent::Error { code, .. } if code == "MODE_SWITCH_INTERRUPTION") {
                interruption_received = true;
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Cleanup
    switch_handle.await.unwrap();
    vad_handle.abort();
    audio_handle.abort();
    processor_handle.abort().await.unwrap();

    // Verify interruption was sent
    assert!(interruption_received, "Expected interruption event during mode switch");
    // Verify mode was switched
    let final_mode = mode.read().await;
    assert_eq!(*final_mode, SttProcessingMode::Streaming);
}

#[tokio::test]
async fn test_rapid_mode_switches() {
    // Setup processor in batch mode
    let (stt_mode_tx, mut stt_mode_rx) = mpsc::channel(10);
    let (event_tx, mut event_rx) = mpsc::channel(100);
    let (vad_tx, mut vad_rx) = mpsc::channel(100);
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

    let plugin_manager = Arc::new(RwLock::new(SttPluginManager::new()));
    let mode = Arc::new(RwLock::new(SttProcessingMode::Batch));
    let config = TranscriptionConfig::default();

    let mut processor = UnifiedSttProcessor::new(
        broadcast::channel(10).1, // dummy audio rx
        vad_rx,
        event_tx,
        stt_mode_rx,
        plugin_manager,
        mode.clone(),
        config,
        shutdown_rx,
    );

    // Rapidly switch modes 10 times
    for i in 0..10 {
        let new_mode = if i % 2 == 0 {
            SttProcessingMode::Batch
        } else {
            SttProcessingMode::Streaming
        };
        stt_mode_tx.send(new_mode).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Run processor briefly to process mode changes
    let processor_handle = tokio::spawn(async move {
        processor.run().await;
    });

    // Send some VAD events during rapid switching
    let mut vad_handle = tokio::spawn(async move {
        for _ in 0..5 {
            vad_tx.send(VadEvent::SpeechStart { timestamp_ms: 1000, energy_db: -20.0 }).await.unwrap();
            tokio::time::sleep(Duration::from_millis(20)).await;
            vad_tx.send(VadEvent::SpeechEnd { timestamp_ms: 2000, duration_ms: 1000, energy_db: -20.0 }).await.unwrap();
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
    });

    // Wait for processing to complete
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Cleanup
    vad_handle.abort();
    processor_handle.abort().await.unwrap();

    // Verify final mode was set to the last requested mode
    let final_mode = mode.read().await;
    let expected_final = if 10 % 2 == 0 { SttProcessingMode::Batch } else { SttProcessingMode::Streaming };
    assert_eq!(*final_mode, expected_final);
    
    // Verify no panics occurred and state is clean
    let state = processor.state.read().await;
    assert!(!state.is_switching);
    match state.utterance_state {
        UtteranceState::Idle => {}, // Expected after cleanup
        UtteranceState::SpeechActive { .. } => panic!("Expected idle state after rapid switches"),
    }
}