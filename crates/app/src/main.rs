use anyhow::anyhow;
use coldvox_app::audio::chunker::{AudioChunker, ChunkerConfig};
use coldvox_app::audio::ring_buffer::AudioRingBuffer;
use coldvox_app::audio::*;
use coldvox_app::foundation::*;
use coldvox_app::stt::{processor::SttProcessor, TranscriptionConfig, TranscriptionEvent};
use coldvox_app::vad::config::{UnifiedVadConfig, VadMode};
use coldvox_app::vad::types::VadEvent;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::fmt::writer::MakeWriterExt;

fn init_logging() -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all("logs")?;
    let file_appender = RollingFileAppender::new(Rotation::DAILY, "logs", "coldvox.log");
    let (non_blocking_file, _guard) = tracing_appender::non_blocking(file_appender);
    let log_level = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    tracing_subscriber::fmt()
        .with_writer(std::io::stdout.and(non_blocking_file))
        .with_env_filter(log_level)
        .init();
    std::mem::forget(_guard);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging()?;
    tracing::info!("Starting ColdVox application");

    let state_manager = StateManager::new();
    let _health_monitor = HealthMonitor::new(Duration::from_secs(10)).start();
    let shutdown = ShutdownHandler::new().install().await;

    state_manager.transition(AppState::Running)?;
    tracing::info!("Application state: {:?}", state_manager.current());

    // --- 1. Audio Capture ---
    let audio_config = AudioConfig::default();
    let ring_buffer = AudioRingBuffer::new(16384 * 4);
    let (audio_producer, audio_consumer) = ring_buffer.split();
    let (audio_capture, sample_rate) =
        AudioCaptureThread::spawn(audio_config, audio_producer, None)?;
    tracing::info!("Audio capture thread started successfully.");

    // --- 2. Audio Chunker ---
    let frame_reader =
        coldvox_app::audio::frame_reader::FrameReader::new(audio_consumer, sample_rate);
    let chunker_cfg = ChunkerConfig {
        frame_size_samples: 512,
        sample_rate_hz: sample_rate,
    };
    
    // --- 3. VAD Processor ---
    let vad_cfg = UnifiedVadConfig {
        mode: VadMode::Silero,
        frame_size_samples: 512,  // Silero requires 512 samples
        sample_rate_hz: 16000,    // Silero requires 16kHz - resampler will handle conversion
        ..Default::default()
    };
    
    // This broadcast channel will distribute audio frames to all interested components.
    let (audio_tx, _) =
        broadcast::channel::<coldvox_app::audio::vad_processor::AudioFrame>(200);
    let chunker = AudioChunker::new(frame_reader, audio_tx.clone(), chunker_cfg);
    let chunker_handle = chunker.spawn();
    tracing::info!("Audio chunker task started.");
    let (event_tx, event_rx) = mpsc::channel::<VadEvent>(100);
    let vad_audio_rx = audio_tx.subscribe();
    let vad_handle = match coldvox_app::audio::vad_processor::VadProcessor::spawn(
        vad_cfg,
        vad_audio_rx,
        event_tx,
    ) {
        Ok(h) => h,
        Err(e) => {
            chunker_handle.abort();
            return Err(anyhow!(e).into());
        }
    };
    tracing::info!("VAD processor task started.");

    // --- 4. STT Processor ---
    // Check for Vosk model path from environment or use default
    let model_path = std::env::var("VOSK_MODEL_PATH")
        .unwrap_or_else(|_| "models/vosk-model-small-en-us-0.15".to_string());
    
    // Check if model exists to determine if STT should be enabled
    let stt_enabled = std::path::Path::new(&model_path).exists();
    
    if !stt_enabled && !model_path.is_empty() {
        tracing::warn!(
            "STT disabled: Vosk model not found at '{}'. \
            Download a model from https://alphacephei.com/vosk/models \
            or set VOSK_MODEL_PATH environment variable.",
            model_path
        );
    }
    
    // Create STT configuration
    let stt_config = TranscriptionConfig {
        enabled: stt_enabled,
        model_path,
        partial_results: true,
        max_alternatives: 1,
        include_words: false,
        buffer_size_ms: 512,
    };
    
    // Only spawn STT processor if enabled
    let stt_handle = if stt_config.enabled {
        // Create transcription event channel
        let (transcription_tx, mut transcription_rx) = mpsc::channel::<TranscriptionEvent>(100);
        
        let stt_audio_rx = audio_tx.subscribe();
        let stt_processor = SttProcessor::new(stt_audio_rx, event_rx, transcription_tx, stt_config.clone())
            .map_err(|e| anyhow!("Failed to create STT processor: {}", e))?;
        
        // Spawn transcription event handler
        tokio::spawn(async move {
            while let Some(event) = transcription_rx.recv().await {
                match event {
                    TranscriptionEvent::Partial { text, .. } => {
                        tracing::info!(target: "main", "Partial transcription: {}", text);
                    }
                    TranscriptionEvent::Final { text, .. } => {
                        tracing::info!(target: "main", "Final transcription: {}", text);
                    }
                    TranscriptionEvent::Error { code, message } => {
                        tracing::error!(target: "main", "Transcription error [{}]: {}", code, message);
                    }
                }
            }
        });
        
        tracing::info!("STT processor task started with model: {}", stt_config.model_path);
        Some(tokio::spawn(stt_processor.run()))
    } else {
        tracing::info!("STT processor disabled - no model available");
        None
    };

    // --- Main Application Loop ---
    let mut stats_interval = tokio::time::interval(Duration::from_secs(30));
    loop {
        tokio::select! {
            _ = shutdown.wait() => {
                tracing::info!("Shutdown signal received");
                break;
            }
            _ = stats_interval.tick() => {
                tracing::info!("Pipeline running...");
                // TODO: Add proper stats collection from metrics
            }
        }
    }

    // --- Graceful Shutdown ---
    tracing::info!("Beginning graceful shutdown");
    state_manager.transition(AppState::Stopping)?;

    // 1. Stop the source of the audio stream.
    audio_capture.stop();
    tracing::info!("Audio capture thread stopped.");

    // 2. Abort the tasks. This will drop their channel senders, causing downstream
    //    tasks with `recv()` loops to terminate gracefully.
    chunker_handle.abort();
    vad_handle.abort();
    if let Some(handle) = stt_handle {
        handle.abort();
    }
    tracing::info!("Async tasks aborted.");

    // 3. Await all handles to ensure they have fully cleaned up.
    // We ignore the results since we are aborting them and expect JoinError.
    let _ = chunker_handle.await;
    let _ = vad_handle.await;

    state_manager.transition(AppState::Stopped)?;
    tracing::info!("Shutdown complete");

    Ok(())
}