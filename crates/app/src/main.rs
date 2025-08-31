// Logging behavior:
// - Writes logs to both stdout and a daily-rotated file at logs/coldvox.log.
// - Log level is controlled via the RUST_LOG environment variable (e.g., "info", "debug").
// - The logs/ directory is created on startup if missing; file output uses a non-blocking writer.
// - This ensures persistent logs for post-run analysis while keeping console output for live use.
use anyhow::anyhow;
use coldvox_app::audio::chunker::{AudioChunker, ChunkerConfig};
use coldvox_app::audio::ring_buffer::AudioRingBuffer;
use coldvox_app::audio::*;
use coldvox_app::foundation::*;
use coldvox_app::stt::{processor::SttProcessor, TranscriptionConfig, TranscriptionEvent};
#[cfg(feature = "vosk")]
use coldvox_app::stt::persistence::{PersistenceConfig, TranscriptFormat, AudioFormat, SessionMetadata};
use coldvox_app::text_injection::{self, AsyncInjectionProcessor, InjectionProcessorConfig};
use coldvox_app::vad::config::{UnifiedVadConfig, VadMode};
use coldvox_app::vad::constants::{FRAME_SIZE_SAMPLES, SAMPLE_RATE_HZ};
use coldvox_app::vad::types::VadEvent;
use coldvox_app::telemetry::pipeline_metrics::PipelineMetrics;
use coldvox_app::hotkey::spawn_hotkey_listener;
use std::time::Duration;
use clap::{Args, Parser, ValueEnum};
use tokio::sync::{broadcast, mpsc};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

fn init_logging() -> Result<tracing_appender::non_blocking::WorkerGuard, Box<dyn std::error::Error>> {
    std::fs::create_dir_all("logs")?;
    let file_appender = RollingFileAppender::new(Rotation::DAILY, "logs", "coldvox.log");
    let (non_blocking_file, guard) = tracing_appender::non_blocking(file_appender);
    let log_level = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    let env_filter = EnvFilter::try_new(log_level).unwrap_or_else(|_| EnvFilter::new("info"));

    let stderr_layer = fmt::layer().with_writer(std::io::stderr);
    let file_layer = fmt::layer().with_writer(non_blocking_file);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(stderr_layer)
        .with(file_layer)
        .init();
    Ok(guard)
}

#[derive(Parser, Debug)]
#[command(name = "coldvox", author, version, about = "ColdVox voice pipeline")]
struct Cli {
    /// Preferred input device name (exact or substring)
    #[arg(short = 'D', long = "device")]
    device: Option<String>,

    /// List available input devices and exit
    #[arg(long = "list-devices")]
    list_devices: bool,

    /// Resampler quality: fast, balanced, quality
    #[arg(long = "resampler-quality", default_value = "balanced")]
    resampler_quality: String,

    /// Enable transcription persistence to disk
    #[arg(long = "save-transcriptions")]
    save_transcriptions: bool,

    /// Save audio alongside transcriptions
    #[arg(long = "save-audio", requires = "save_transcriptions")]
    save_audio: bool,

    /// Output directory for transcriptions
    #[arg(long = "output-dir", default_value = "transcriptions")]
    output_dir: String,

    /// Transcription format: json, csv, text
    #[arg(long = "transcript-format", default_value = "json")]
    transcript_format: String,

    /// Keep transcription files for N days (0 = forever)
    #[arg(long = "retention-days", default_value = "30")]
    retention_days: u32,

    /// Activation mode: "vad" or "hotkey"
    #[arg(long = "activation-mode", default_value = "hotkey", value_enum)]
    activation_mode: ActivationMode,

    #[cfg(feature = "text-injection")]
    #[command(flatten)]
    injection: InjectionArgs,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ActivationMode {
    Vad,
    Hotkey,
}

#[cfg(feature = "text-injection")]
#[derive(Args, Debug)]
#[command(next_help_heading = "Text Injection")]
struct InjectionArgs {
    /// Enable text injection after transcription
    #[arg(long = "enable-text-injection", env = "COLDVOX_ENABLE_TEXT_INJECTION")]
    enable: bool,

    /// Allow ydotool as an injection fallback
    #[arg(long = "allow-ydotool", env = "COLDVOX_ALLOW_YDOTOOL")]
    allow_ydotool: bool,

    /// Allow kdotool as an injection fallback
    #[arg(long = "allow-kdotool", env = "COLDVOX_ALLOW_KDOTOOL")]
    allow_kdotool: bool,

    /// Allow enigo as an injection fallback
    #[arg(long = "allow-enigo", env = "COLDVOX_ALLOW_ENIGO")]
    allow_enigo: bool,

    /// Allow mki (uinput) as an injection fallback
    #[arg(long = "allow-mki", env = "COLDVOX_ALLOW_MKI")]
    allow_mki: bool,

    /// Attempt injection even if the focused application is unknown
    #[arg(long = "inject-on-unknown-focus", env = "COLDVOX_INJECT_ON_UNKNOWN_FOCUS")]
    inject_on_unknown_focus: bool,

    /// Restore clipboard contents after injection
    #[arg(long = "restore-clipboard", env = "COLDVOX_RESTORE_CLIPBOARD")]
    restore_clipboard: bool,

    /// Max total latency for an injection call (ms)
    #[arg(long, env = "COLDVOX_INJECTION_MAX_LATENCY_MS")]
    max_total_latency_ms: Option<u64>,

    /// Timeout for each injection method (ms)
    #[arg(long, env = "COLDVOX_INJECTION_METHOD_TIMEOUT_MS")]
    per_method_timeout_ms: Option<u64>,

    /// Initial cooldown on failure (ms)
    #[arg(long, env = "COLDVOX_INJECTION_COOLDOWN_MS")]
    cooldown_initial_ms: Option<u64>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Give PipeWire better routing hints if using its ALSA bridge
    std::env::set_var(
        "PIPEWIRE_PROPS",
        "{ application.name=ColdVox media.role=capture }",
    );
    let _log_guard = init_logging()?;
    tracing::info!("Starting ColdVox application");

    let cli = Cli::parse();

    // Apply environment variable overrides
    let device = cli.device.clone().or_else(|| std::env::var("COLDVOX_DEVICE").ok());
    let resampler_quality = std::env::var("COLDVOX_RESAMPLER_QUALITY").unwrap_or(cli.resampler_quality.clone());

    if cli.list_devices {
        let dm = coldvox_app::audio::device::DeviceManager::new()?;
        tracing::info!("CPAL host: {:?}", dm.host_id());
        let devices = dm.enumerate_devices();
        println!("Input devices (host: {:?}):", dm.host_id());
        for d in devices {
            let def = if d.is_default { " (default)" } else { "" };
            println!("- {}{}", d.name, def);
        }
        return Ok(());
    }

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Give PipeWire better routing hints if using its ALSA bridge
    std::env::set_var(
        "PIPEWIRE_PROPS",
        "{ application.name=ColdVox media.role=capture }",
    );
    let _log_guard = init_logging()?;
    tracing::info!("Starting ColdVox application");

    let cli = Cli::parse();

    // Apply environment variable overrides
    let device = cli.device.clone().or_else(|| std::env::var("COLDVOX_DEVICE").ok());
    let resampler_quality = std::env::var("COLDVOX_RESAMPLER_QUALITY").unwrap_or(cli.resampler_quality.clone());

    if cli.list_devices {
        let dm = coldvox_app::audio::device::DeviceManager::new()?;
        tracing::info!("CPAL host: {:?}", dm.host_id());
        let devices = dm.enumerate_devices();
        println!("Input devices (host: {:?}):", dm.host_id());
        for d in devices {
            let def = if d.is_default { " (default)" } else { "" };
            println!("- {}{}", d.name, def);
        }
        return Ok(());
    }

    let state_manager = StateManager::new();
    let _health_monitor = HealthMonitor::new(Duration::from_secs(10)).start();
    let shutdown = ShutdownHandler::new().install().await;

    state_manager.transition(AppState::Running)?;
    tracing::info!("Application state: {:?}", state_manager.current());

    // --- 1. Audio Capture ---
    let audio_config = AudioConfig::default();
    // Shared pipeline metrics for telemetry and dashboard
    let metrics = std::sync::Arc::new(PipelineMetrics::default());
    let ring_buffer = AudioRingBuffer::new(16384 * 4);
    let (audio_producer, audio_consumer) = ring_buffer.split();
    let (audio_capture, device_cfg, device_config_rx) =
        AudioCaptureThread::spawn(audio_config, audio_producer, device.clone())?;
    tracing::info!("Audio capture thread started successfully.");

    // --- 2. Audio Chunker ---
    let frame_reader = coldvox_app::audio::frame_reader::FrameReader::new(
        audio_consumer,
        device_cfg.sample_rate,
        device_cfg.channels,
        16384 * 4,
        Some(metrics.clone()),
    );
    let chunker_cfg = ChunkerConfig {
        frame_size_samples: FRAME_SIZE_SAMPLES,
        // Target 16k for VAD; resampler in chunker will convert from device rate
        sample_rate_hz: SAMPLE_RATE_HZ,
        resampler_quality: match resampler_quality.to_lowercase().as_str() {
            "fast" => coldvox_app::audio::chunker::ResamplerQuality::Fast,
            "quality" => coldvox_app::audio::chunker::ResamplerQuality::Quality,
            _ => coldvox_app::audio::chunker::ResamplerQuality::Balanced, // default/balanced
        },
    };

    // --- 3. VAD Processor ---
    let vad_cfg = UnifiedVadConfig {
        mode: VadMode::Silero,
        frame_size_samples: FRAME_SIZE_SAMPLES,  // Both Silero and Level3 use 512 samples
        sample_rate_hz: SAMPLE_RATE_HZ,    // Standard 16kHz - resampler will handle conversion
        ..Default::default()
    };

    // This broadcast channel will distribute audio frames to all interested components.
    let (audio_tx, _) =
        broadcast::channel::<coldvox_app::audio::vad_processor::AudioFrame>(200);
    let chunker = AudioChunker::new(frame_reader, audio_tx.clone(), chunker_cfg)
        .with_metrics(metrics.clone())
        .with_device_config(device_config_rx.resubscribe());
    let chunker_handle = chunker.spawn();
    tracing::info!("Audio chunker task started.");

    // Set up device config monitoring to update FrameReader
    // We no longer need a separate monitor; the chunker reads device config updates directly.

    let (event_tx, mut event_rx) = mpsc::channel::<VadEvent>(100);
    let trigger_handle = match cli.activation_mode {
        ActivationMode::Vad => {
            let vad_audio_rx = audio_tx.subscribe();
            match coldvox_app::audio::vad_processor::VadProcessor::spawn(
                vad_cfg.clone(),
                vad_audio_rx,
                event_tx,
                Some(metrics.clone()),
            ) {
                Ok(h) => {
                    tracing::info!("VAD processor task started.");
                    h
                }
                Err(e) => {
                    chunker_handle.abort();
                    return Err(anyhow!(e).into());
                }
            }
        }
        ActivationMode::Hotkey => {
            tracing::info!("Hotkey listener started.");
            spawn_hotkey_listener(event_tx)
        }
    };

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
    let mut injection_shutdown_tx: Option<mpsc::Sender<()>> = None;
    let (stt_handle, _persistence_handle, injection_handle) = if stt_config.enabled {
        // Create mpsc channel for STT processor to send transcription events
        let (stt_transcription_tx, mut stt_transcription_rx) = mpsc::channel::<TranscriptionEvent>(100);
        
        // Create broadcast channel for distributing transcription events to multiple consumers
        let (broadcast_tx, _) = broadcast::channel::<TranscriptionEvent>(100);
        
        // Relay from STT processor to broadcast channel
        let broadcast_tx_clone = broadcast_tx.clone();
        tokio::spawn(async move {
            while let Some(event) = stt_transcription_rx.recv().await {
                let _ = broadcast_tx_clone.send(event);
            }
        });
        
        // Create mpsc channel for injection processor
        let (injection_tx, injection_rx) = mpsc::channel::<TranscriptionEvent>(100);
        let mut injection_relay_rx = broadcast_tx.subscribe();
        tokio::spawn(async move {
            while let Ok(event) = injection_relay_rx.recv().await {
                let _ = injection_tx.send(event).await;
            }
        });
        
        // Create mpsc channel for persistence if needed
        let persistence_rx = if cli.save_transcriptions {
            let (persist_tx, persist_rx) = mpsc::channel::<TranscriptionEvent>(100);
            let mut persist_relay_rx = broadcast_tx.subscribe();
            tokio::spawn(async move {
                while let Ok(event) = persist_relay_rx.recv().await {
                    let _ = persist_tx.send(event).await;
                }
            });
            Some(persist_rx)
        } else {
            None
        };

        // Create channels for persistence if enabled
        let (persist_vad_tx, persist_vad_rx) = mpsc::channel::<VadEvent>(100);

        // Split event_rx for both STT and persistence
        let (vad_relay_tx, stt_event_rx) = mpsc::channel::<VadEvent>(100);

        // Relay VAD events to both STT and persistence
        tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                let _ = vad_relay_tx.send(event.clone()).await;
                if cli.save_transcriptions {
                    let _ = persist_vad_tx.send(event).await;
                }
            }
        });

        let stt_audio_rx = audio_tx.subscribe();
        let stt_processor = SttProcessor::new(stt_audio_rx, stt_event_rx, stt_transcription_tx, stt_config.clone())
            .map_err(|e| anyhow!("Failed to create STT processor: {}", e))?;

        // --- 5. Text Injection Processor ---
        let injection_handle = if cfg!(feature = "text-injection") && cli.injection.enable {
            // Build the full injection config from CLI args and defaults
            let injection_config = text_injection::InjectionConfig {
                allow_ydotool: cli.injection.allow_ydotool,
                allow_kdotool: cli.injection.allow_kdotool,
                allow_enigo: cli.injection.allow_enigo,
                allow_mki: cli.injection.allow_mki,
                restore_clipboard: cli.injection.restore_clipboard,
                inject_on_unknown_focus: cli.injection.inject_on_unknown_focus,
                max_total_latency_ms: cli.injection.max_total_latency_ms.unwrap_or(text_injection::types::InjectionConfig::default().max_total_latency_ms),
                per_method_timeout_ms: cli.injection.per_method_timeout_ms.unwrap_or(text_injection::types::InjectionConfig::default().per_method_timeout_ms),
                cooldown_initial_ms: cli.injection.cooldown_initial_ms.unwrap_or(text_injection::types::InjectionConfig::default().cooldown_initial_ms),
                ..Default::default()
            };

            let (shutdown_tx, injection_shutdown_rx) = mpsc::channel::<()>(1);
            injection_shutdown_tx = Some(shutdown_tx);
            let injection_processor = AsyncInjectionProcessor::new(
                injection_config,
                injection_rx,
                injection_shutdown_rx,
                Some(metrics.clone()),
            );

            // Spawn injection processor
            tracing::info!("Text injection enabled.");
            Some(tokio::spawn(async move {
                if let Err(e) = injection_processor.run().await {
                    tracing::error!("Injection processor failed: {}", e);
                }
            }))
        } else {
            tracing::info!("Text injection disabled.");
            None
        };

        // Note: For now, we've removed the separate transcription persistence handler
        // since transcription events go directly to the injection processor.
        // Future enhancement could add persistence by tapping into the injection processor events.

        // Set up persistence if enabled
        #[cfg(feature = "vosk")]
        let persistence_handle = if cli.save_transcriptions {
            let persist_config = PersistenceConfig {
                enabled: true,
                output_dir: std::path::PathBuf::from(cli.output_dir.clone()),
                save_audio: cli.save_audio,
                audio_format: AudioFormat::Wav,
                transcript_format: match cli.transcript_format.as_str() {
                    "csv" => TranscriptFormat::Csv,
                    "text" => TranscriptFormat::Text,
                    _ => TranscriptFormat::Json,
                },
                retention_days: cli.retention_days,
                sample_rate: SAMPLE_RATE_HZ,
            };

            let metadata = SessionMetadata {
                device_name: device.clone().unwrap_or_else(|| "default".to_string()),
                sample_rate: SAMPLE_RATE_HZ,
                vad_mode: match cli.activation_mode {
                    ActivationMode::Vad => format!("{:?}", vad_cfg.mode),
                    ActivationMode::Hotkey => "Hotkey".to_string(),
                },
                stt_model: stt_config.model_path.clone(),
                app_version: env!("CARGO_PKG_VERSION").to_string(),
            };

            let persist_audio_rx = audio_tx.subscribe();

            tracing::info!(
                "Persistence enabled: output_dir={}, save_audio={}, format={:?}",
                persist_config.output_dir.display(),
                persist_config.save_audio,
                persist_config.transcript_format
            );

            Some(coldvox_app::stt::persistence::spawn_persistence_handler(
                persist_config,
                metadata,
                persist_audio_rx,
                persist_vad_rx,
                persistence_rx.unwrap(),
            ))
        } else {
            None
        };

        #[cfg(not(feature = "vosk"))]
        let persistence_handle = None;

        tracing::info!("STT processor task started with model: {}", stt_config.model_path);
        (Some(tokio::spawn(stt_processor.run())), persistence_handle, injection_handle)
    } else {
        tracing::info!("STT processor disabled - no model available");
        (None, None, None)
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
                let cap_fps = metrics.capture_fps.load(std::sync::atomic::Ordering::Relaxed);
                let chk_fps = metrics.chunker_fps.load(std::sync::atomic::Ordering::Relaxed);
                let vad_fps = metrics.vad_fps.load(std::sync::atomic::Ordering::Relaxed);
                let cap_fill = metrics.capture_buffer_fill.load(std::sync::atomic::Ordering::Relaxed);
                let chk_fill = metrics.chunker_buffer_fill.load(std::sync::atomic::Ordering::Relaxed);
                tracing::info!(
                    capture_fps = cap_fps,
                    chunker_fps = chk_fps,
                    vad_fps = vad_fps,
                    capture_buffer_fill_pct = cap_fill,
                    chunker_buffer_fill_pct = chk_fill,
                    "Pipeline running..."
                );
            }
        }
    }

    // --- Graceful Shutdown ---
    tracing::info!("Beginning graceful shutdown");
    state_manager.transition(AppState::Stopping)?;

    // 1. Stop the source of the audio stream.
    audio_capture.stop();
    tracing::info!("Audio capture thread stopped.");

    // 2. Signal graceful shutdown to injection processor before aborting
    if let Some(tx) = injection_shutdown_tx {
        let _ = tx.send(()).await;
    }
    
    // 3. Abort the tasks. This will drop their channel senders, causing downstream
    //    tasks with `recv()` loops to terminate gracefully.
    chunker_handle.abort();
    trigger_handle.abort();
    if let Some(handle) = &stt_handle {
        handle.abort();
    }
    if let Some(handle) = &injection_handle {
        handle.abort();
    }
    tracing::info!("Async tasks aborted.");

    // 4. Await all handles to ensure they have fully cleaned up.
    // We ignore the results since we are aborting them and expect JoinError.
    let _ = chunker_handle.await;
    let _ = trigger_handle.await;
    if let Some(handle) = stt_handle {
        let _ = handle.await;
    }
    if let Some(handle) = injection_handle {
        let _ = handle.await;
    }

    state_manager.transition(AppState::Stopped)?;
    tracing::info!("Shutdown complete");

    Ok(())
}