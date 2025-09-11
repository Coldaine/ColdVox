// Logging behavior:
// - Writes logs to both stderr and a daily-rotated file at logs/coldvox.log.
// - Log level is controlled via the RUST_LOG environment variable (e.g., "info", "debug").
// - The logs/ directory is created on startup if missing; file output uses a non-blocking writer.
// - File layer disables ANSI to keep logs clean for analysis.
use std::time::Duration;

#[cfg(feature = "text-injection")]
use clap::Args;
use clap::{Parser, ValueEnum};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use coldvox_app::runtime::{self as app_runtime, ActivationMode as RuntimeMode, AppRuntimeOptions};
use coldvox_audio::{DeviceManager, ResamplerQuality};
use coldvox_foundation::{AppState, HealthMonitor, ShutdownHandler, StateManager};

fn init_logging() -> Result<tracing_appender::non_blocking::WorkerGuard, Box<dyn std::error::Error>>
{
    std::fs::create_dir_all("logs")?;
    let file_appender = RollingFileAppender::new(Rotation::DAILY, "logs", "coldvox.log");
    let (non_blocking_file, guard) = tracing_appender::non_blocking(file_appender);
    let log_level = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    let env_filter = EnvFilter::try_new(log_level).unwrap_or_else(|_| EnvFilter::new("info"));

    let stderr_layer = fmt::layer().with_writer(std::io::stderr);
    let file_layer = fmt::layer().with_writer(non_blocking_file).with_ansi(false);

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

    #[cfg(feature = "vosk")]
    /// Enable transcription persistence to disk
    #[arg(long = "save-transcriptions")]
    save_transcriptions: bool,

    #[cfg(feature = "vosk")]
    /// Save audio alongside transcriptions
    #[arg(long = "save-audio", requires = "save_transcriptions")]
    save_audio: bool,

    #[cfg(feature = "vosk")]
    /// Output directory for transcriptions
    #[arg(long = "output-dir", default_value = "transcriptions")]
    output_dir: String,

    #[cfg(feature = "vosk")]
    /// Transcription format: json, csv, text
    #[arg(long = "transcript-format", default_value = "json")]
    transcript_format: String,

    #[cfg(feature = "vosk")]
    /// Keep transcription files for N days (0 = forever)
    #[arg(long = "retention-days", default_value = "30")]
    retention_days: u32,

    /// Activation mode: "vad" or "hotkey"
    #[arg(long = "activation-mode", default_value = "hotkey", value_enum)]
    activation_mode: ActivationMode,

    /// STT backend selection: "auto", "whisper", "vosk", "mock", "noop"
    #[arg(long = "stt-backend", default_value = "auto", env = "COLDVOX_STT_BACKEND")]
    stt_backend: String,

    /// STT fallback order (comma-separated)
    #[arg(long = "stt-fallback", default_value = "whisper,vosk,mock,noop", env = "COLDVOX_STT_FALLBACK")]
    stt_fallback: String,

    /// Maximum memory usage for STT plugins (MB)
    #[arg(long = "stt-max-mem-mb", env = "COLDVOX_STT_MAX_MEM_MB")]
    stt_max_mem_mb: Option<u64>,

    /// Maximum retries before failover
    #[arg(long = "stt-max-retries", default_value = "3", env = "COLDVOX_STT_MAX_RETRIES")]
    stt_max_retries: usize,

    #[cfg(feature = "whisper")]
    /// Path to Whisper model file
    #[arg(long = "whisper-model-path", env = "WHISPER_MODEL_PATH")]
    whisper_model_path: Option<String>,

    #[cfg(feature = "whisper")]
    /// Whisper mode: "fast" (tiny), "balanced" (small), "quality" (medium)
    #[arg(long = "whisper-mode", default_value = "balanced", env = "COLDVOX_WHISPER_MODE")]
    whisper_mode: String,

    #[cfg(feature = "whisper")]
    /// Whisper quantization: "q5_1", "q8_0", "fp16"
    #[arg(long = "whisper-quant", default_value = "q5_1", env = "COLDVOX_WHISPER_QUANT")]
    whisper_quant: String,

    #[cfg(feature = "vosk")]
    /// Legacy Vosk model path (use --stt-backend=vosk and config instead)
    #[arg(long = "vosk-model-path", env = "VOSK_MODEL_PATH")]
    vosk_model_path: Option<String>,

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

    /// Attempt injection even if the focused application is unknown
    #[arg(
        long = "inject-on-unknown-focus",
        env = "COLDVOX_INJECT_ON_UNKNOWN_FOCUS"
    )]
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
    // Give PipeWire better routing hints if using its ALSA bridge (Linux only)
    #[cfg(target_os = "linux")]
    std::env::set_var(
        "PIPEWIRE_PROPS",
        "{ application.name=ColdVox media.role=capture }",
    );
    let _log_guard = init_logging()?;
    tracing::info!("Starting ColdVox application");

    let cli = Cli::parse();

    // Apply environment variable overrides
    let device = cli
        .device
        .clone()
        .or_else(|| std::env::var("COLDVOX_DEVICE").ok());
    let resampler_quality =
        std::env::var("COLDVOX_RESAMPLER_QUALITY").unwrap_or(cli.resampler_quality.clone());

    if cli.list_devices {
        let dm = DeviceManager::new()?;
        tracing::info!("CPAL host: {:?}", dm.host_id());
        let devices = dm.enumerate_devices();
        println!("Input devices (host: {:?}):", dm.host_id());
        for d in devices {
            let def = if d.is_default { " (default)" } else { "" };
            println!("- {}{}", d.name, def);
        }
        return Ok(());
    }

    // Unified runtime start
    let state_manager = StateManager::new();
    let _health_monitor = HealthMonitor::new(Duration::from_secs(10)).start();
    let shutdown = ShutdownHandler::new().install().await;

    state_manager.transition(AppState::Running)?;
    tracing::info!("Application state: Running");

    let opts = AppRuntimeOptions {
        device,
        resampler_quality: match resampler_quality.to_lowercase().as_str() {
            "fast" => ResamplerQuality::Fast,
            "quality" => ResamplerQuality::Quality,
            _ => ResamplerQuality::Balanced,
        },
        activation_mode: match cli.activation_mode {
            ActivationMode::Vad => RuntimeMode::Vad,
            ActivationMode::Hotkey => RuntimeMode::Hotkey,
        },
        
        // Legacy Vosk options
        #[cfg(feature = "vosk")]
        vosk_model_path: cli.vosk_model_path,
        #[cfg(feature = "vosk")]
        stt_enabled: None,

        // New STT plugin options
        stt_backend: Some(cli.stt_backend),
        stt_fallback: cli.stt_fallback.split(',').map(|s| s.trim().to_string()).collect(),
        stt_max_mem_mb: cli.stt_max_mem_mb,
        stt_max_retries: cli.stt_max_retries,

        #[cfg(feature = "whisper")]
        whisper_model_path: cli.whisper_model_path,
        #[cfg(feature = "whisper")]
        whisper_mode: cli.whisper_mode,
        #[cfg(feature = "whisper")]
        whisper_quant: cli.whisper_quant,

        #[cfg(feature = "text-injection")]
        injection: if cfg!(feature = "text-injection") {
            Some(coldvox_app::runtime::InjectionOptions {
                enable: cli.injection.enable,
                allow_ydotool: cli.injection.allow_ydotool,
                allow_kdotool: cli.injection.allow_kdotool,
                allow_enigo: cli.injection.allow_enigo,
                inject_on_unknown_focus: cli.injection.inject_on_unknown_focus,
                restore_clipboard: cli.injection.restore_clipboard,
                max_total_latency_ms: cli.injection.max_total_latency_ms,
                per_method_timeout_ms: cli.injection.per_method_timeout_ms,
                cooldown_initial_ms: cli.injection.cooldown_initial_ms,
            })
        } else {
            None
        },
    };

    let app = app_runtime::start(opts)
        .await
        .map_err(|e| e as Box<dyn std::error::Error>)?;

    // Periodic stats log
    let mut stats_interval = tokio::time::interval(Duration::from_secs(30));
    let metrics = app.metrics.clone();
    tokio::select! {
        _ = shutdown.wait() => {
            tracing::info!("Shutdown signal received");
        }
        _ = async {
            loop {
                stats_interval.tick().await;
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
        } => {}
    }

    // Shutdown
    tracing::info!("Beginning graceful shutdown");
    state_manager.transition(AppState::Stopping)?;
    app.shutdown().await;
    state_manager.transition(AppState::Stopped)?;
    tracing::info!("Shutdown complete");

    Ok(())
}
