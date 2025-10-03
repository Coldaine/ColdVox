// Logging behavior:
// - Writes logs to both stderr and a daily-rotated file at logs/coldvox.log.
// - Log level is controlled via the RUST_LOG environment variable (e.g., "info", "debug").
// - The logs/ directory is created on startup if missing; file output uses a non-blocking writer.
// - File layer disables ANSI to keep logs clean for analysis.
use std::time::Duration;

use clap::Args;
use clap::{Parser, ValueEnum};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use coldvox_app::runtime::{self as app_runtime, ActivationMode as RuntimeMode, AppRuntimeOptions};
use coldvox_audio::{DeviceManager, ResamplerQuality};
use coldvox_foundation::{AppState, HealthMonitor, ShutdownHandler, StateManager};

#[cfg(feature = "tui")]
use coldvox_app::tui;

fn init_logging() -> Result<tracing_appender::non_blocking::WorkerGuard, Box<dyn std::error::Error>>
{
    std::fs::create_dir_all("logs")?;
    let file_appender = RollingFileAppender::new(Rotation::DAILY, "logs", "coldvox.log");
    let (non_blocking_file, guard) = tracing_appender::non_blocking(file_appender);
    let log_level = std::env::var("RUST_LOG").unwrap_or_else(|_| "debug".to_string());
    let env_filter = EnvFilter::try_new(log_level).unwrap_or_else(|_| EnvFilter::new("debug"));

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

    /// Enable TUI dashboard
    #[arg(long = "tui")]
    tui: bool,

    /// Enable background device monitoring / hotplug polling (may emit ALSA warnings)
    #[arg(long = "enable-device-monitor", env = "COLDVOX_ENABLE_DEVICE_MONITOR")]
    enable_device_monitor: bool,

    /// Activation mode: "vad" or "hotkey"
    #[arg(long = "activation-mode", default_value = "hotkey", value_enum)]
    activation_mode: ActivationMode,

    #[command(flatten)]
    stt: SttArgs,

    #[cfg(feature = "text-injection")]
    #[command(flatten)]
    injection: InjectionArgs,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq)]
enum ActivationMode {
    Vad,
    Hotkey,
}

#[derive(Args, Debug)]
#[command(next_help_heading = "Speech-to-Text")]
struct SttArgs {
    /// Preferred STT plugin ID (e.g., "vosk", "whisper", "mock")
    #[arg(long = "stt-preferred", env = "COLDVOX_STT_PREFERRED")]
    preferred: Option<String>,

    /// Comma-separated list of fallback plugin IDs
    #[arg(
        long = "stt-fallbacks",
        env = "COLDVOX_STT_FALLBACKS",
        value_delimiter = ','
    )]
    fallbacks: Option<Vec<String>>,

    /// Require local processing (no cloud STT services)
    #[arg(long = "stt-require-local", env = "COLDVOX_STT_REQUIRE_LOCAL")]
    require_local: bool,

    /// Maximum memory usage in MB
    #[arg(long = "stt-max-mem-mb", env = "COLDVOX_STT_MAX_MEM_MB")]
    max_mem_mb: Option<u32>,

    /// Required language (ISO 639-1 code, e.g., "en", "fr")
    #[arg(long = "stt-language", env = "COLDVOX_STT_LANGUAGE")]
    language: Option<String>,

    /// Number of consecutive errors before switching to fallback plugin
    #[arg(
        long = "stt-failover-threshold",
        env = "COLDVOX_STT_FAILOVER_THRESHOLD",
        default_value = "3"
    )]
    failover_threshold: u32,

    /// Cooldown period in seconds before retrying a failed plugin
    #[arg(
        long = "stt-failover-cooldown-secs",
        env = "COLDVOX_STT_FAILOVER_COOLDOWN_SECS",
        default_value = "30"
    )]
    failover_cooldown_secs: u32,

    /// Time to live in seconds for inactive models (GC threshold)
    #[arg(
        long = "stt-model-ttl-secs",
        env = "COLDVOX_STT_MODEL_TTL_SECS",
        default_value = "300"
    )]
    model_ttl_secs: u32,

    /// Disable garbage collection of inactive models
    #[arg(long = "stt-disable-gc", env = "COLDVOX_STT_DISABLE_GC")]
    disable_gc: bool,

    /// Interval in seconds for periodic metrics logging (0 to disable)
    #[arg(
        long = "stt-metrics-log-interval-secs",
        env = "COLDVOX_STT_METRICS_LOG_INTERVAL_SECS",
        default_value = "60"
    )]
    metrics_log_interval_secs: u32,

    /// Enable debug dumping of transcription events to logs
    #[arg(long = "stt-debug-dump-events", env = "COLDVOX_STT_DEBUG_DUMP_EVENTS")]
    debug_dump_events: bool,

    /// Automatically extract model from a zip archive if not found
    #[arg(
        long = "stt-auto-extract",
        env = "COLDVOX_STT_AUTO_EXTRACT",
        default_value = "true"
    )]
    auto_extract: bool,
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

    // Build STT configuration from CLI arguments
    let stt_selection = {
        use coldvox_stt::plugin::{FailoverConfig, GcPolicy, MetricsConfig, PluginSelectionConfig};

        // Default to Vosk as preferred STT plugin
        let mut preferred_plugin = cli.stt.preferred.clone().or(Some("vosk".to_string()));
        tracing::info!("Defaulting to Vosk STT plugin as preferred");

        // Handle backward compatibility with VOSK_MODEL_PATH
        if preferred_plugin.is_none() {
            if let Ok(vosk_model_path) = std::env::var("VOSK_MODEL_PATH") {
                tracing::warn!(
                    "VOSK_MODEL_PATH environment variable is deprecated. Use --stt-preferred=vosk instead."
                );
                tracing::info!(
                    "Setting preferred plugin to 'vosk' based on VOSK_MODEL_PATH={}",
                    vosk_model_path
                );
                preferred_plugin = Some("vosk".to_string());
            }
        }

        let fallback_plugins = cli
            .stt
            .fallbacks
            .unwrap_or_else(|| vec!["vosk".to_string(), "mock".to_string()]);

        let failover = FailoverConfig {
            failover_threshold: cli.stt.failover_threshold,
            failover_cooldown_secs: cli.stt.failover_cooldown_secs,
        };

        let gc_policy = GcPolicy {
            model_ttl_secs: cli.stt.model_ttl_secs,
            enabled: !cli.stt.disable_gc,
        };

        let metrics = MetricsConfig {
            log_interval_secs: if cli.stt.metrics_log_interval_secs == 0 {
                None
            } else {
                Some(cli.stt.metrics_log_interval_secs)
            },
            debug_dump_events: cli.stt.debug_dump_events,
        };

        Some(PluginSelectionConfig {
            preferred_plugin,
            fallback_plugins,
            require_local: cli.stt.require_local,
            max_memory_mb: cli.stt.max_mem_mb,
            required_language: cli.stt.language,
            failover: Some(failover),
            gc_policy: Some(gc_policy),
            metrics: Some(metrics),
            auto_extract_model: cli.stt.auto_extract,
        })
    };

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
        stt_selection,
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
    enable_device_monitor: cli.enable_device_monitor,
    };

    let app = app_runtime::start(opts)
        .await
        .map_err(|e| e as Box<dyn std::error::Error>)?;
    // make sharable for spawn + shutdown
    let app = std::sync::Arc::new(app);

    // Spawn TUI if requested
    #[cfg(feature = "tui")]
    if cli.tui {
        tracing::info!("Starting TUI dashboard...");
        tracing::debug!("About to call tui::run_tui - validating module import");
        let tui_app = app.clone();
        let tui_handle = tokio::spawn(async move {
            if let Err(e) = tui::run_tui(tui_app).await {
                tracing::error!("TUI error: {}", e);
            }
        });

        // Wait for TUI to complete
        if let Err(e) = tui_handle.await {
            tracing::error!("TUI task error: {}", e);
        }
    } else {
        // Standard mode: periodic stats log
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
    }

    #[cfg(not(feature = "tui"))]
    {
        // Standard mode: periodic stats log
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
    }

    // Shutdown
    tracing::info!("Beginning graceful shutdown");
    state_manager.transition(AppState::Stopping)?;
    // Shutdown directly on the Arc<AppHandle>
    app.shutdown().await;
    state_manager.transition(AppState::Stopped)?;
    tracing::info!("Shutdown complete");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_cli_parsing_basic() {
        let args = vec!["coldvox"];
        let cli = Cli::try_parse_from(args).unwrap();

        assert_eq!(cli.activation_mode, ActivationMode::Hotkey);
        assert_eq!(cli.stt.failover_threshold, 3);
        assert_eq!(cli.stt.failover_cooldown_secs, 30);
        assert_eq!(cli.stt.model_ttl_secs, 300);
        assert_eq!(cli.stt.metrics_log_interval_secs, 60);
        assert!(!cli.stt.disable_gc);
        assert!(!cli.stt.debug_dump_events);
        assert!(!cli.stt.require_local);
    }

    #[test]
    fn test_cli_parsing_stt_flags() {
        let args = vec![
            "coldvox",
            "--stt-preferred",
            "vosk",
            "--stt-fallbacks",
            "whisper,mock",
            "--stt-require-local",
            "--stt-max-mem-mb",
            "512",
            "--stt-language",
            "en",
            "--stt-failover-threshold",
            "5",
            "--stt-failover-cooldown-secs",
            "60",
            "--stt-model-ttl-secs",
            "600",
            "--stt-disable-gc",
            "--stt-metrics-log-interval-secs",
            "120",
            "--stt-debug-dump-events",
        ];

        let cli = Cli::try_parse_from(args).unwrap();

        assert_eq!(cli.stt.preferred, Some("vosk".to_string()));
        assert_eq!(
            cli.stt.fallbacks,
            Some(vec!["whisper".to_string(), "mock".to_string()])
        );
        assert!(cli.stt.require_local);
        assert_eq!(cli.stt.max_mem_mb, Some(512));
        assert_eq!(cli.stt.language, Some("en".to_string()));
        assert_eq!(cli.stt.failover_threshold, 5);
        assert_eq!(cli.stt.failover_cooldown_secs, 60);
        assert_eq!(cli.stt.model_ttl_secs, 600);
        assert!(cli.stt.disable_gc);
        assert_eq!(cli.stt.metrics_log_interval_secs, 120);
        assert!(cli.stt.debug_dump_events);
    }

    #[test]
    fn test_build_plugin_selection_config() {
        use coldvox_stt::plugin::{FailoverConfig, GcPolicy, MetricsConfig, PluginSelectionConfig};

        let stt_args = SttArgs {
            preferred: Some("vosk".to_string()),
            fallbacks: Some(vec!["whisper".to_string()]),
            require_local: true,
            max_mem_mb: Some(256),
            language: Some("fr".to_string()),
            failover_threshold: 2,
            failover_cooldown_secs: 45,
            model_ttl_secs: 180,
            disable_gc: false,
            metrics_log_interval_secs: 90,
            debug_dump_events: true,
            auto_extract: std::env::var("COLDVOX_STT_AUTO_EXTRACT")
                .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
                .unwrap_or(true),
        };

        let config = PluginSelectionConfig {
            preferred_plugin: stt_args.preferred,
            fallback_plugins: stt_args.fallbacks.unwrap_or_default(),
            require_local: stt_args.require_local,
            max_memory_mb: stt_args.max_mem_mb,
            required_language: stt_args.language,
            failover: Some(FailoverConfig {
                failover_threshold: stt_args.failover_threshold,
                failover_cooldown_secs: stt_args.failover_cooldown_secs,
            }),
            gc_policy: Some(GcPolicy {
                model_ttl_secs: stt_args.model_ttl_secs,
                enabled: !stt_args.disable_gc,
            }),
            metrics: Some(MetricsConfig {
                log_interval_secs: if stt_args.metrics_log_interval_secs == 0 {
                    None
                } else {
                    Some(stt_args.metrics_log_interval_secs)
                },
                debug_dump_events: stt_args.debug_dump_events,
            }),
            auto_extract_model: std::env::var("COLDVOX_STT_AUTO_EXTRACT")
                .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
                .unwrap_or(true),
        };

        assert_eq!(config.preferred_plugin, Some("vosk".to_string()));
        assert_eq!(config.fallback_plugins, vec!["whisper".to_string()]);
        assert!(config.require_local);
        assert_eq!(config.max_memory_mb, Some(256));
        assert_eq!(config.required_language, Some("fr".to_string()));

        let failover = config.failover.unwrap();
        assert_eq!(failover.failover_threshold, 2);
        assert_eq!(failover.failover_cooldown_secs, 45);

        let gc_policy = config.gc_policy.unwrap();
        assert_eq!(gc_policy.model_ttl_secs, 180);
        assert!(gc_policy.enabled);

        let metrics = config.metrics.unwrap();
        assert_eq!(metrics.log_interval_secs, Some(90));
        assert!(metrics.debug_dump_events);
    }
}
