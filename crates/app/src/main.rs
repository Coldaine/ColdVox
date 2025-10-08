// Logging behavior:
// - Writes logs to both stderr and a daily-rotated file at logs/coldvox.log.
// - Log level is controlled via the RUST_LOG environment variable (e.g., "info", "debug").
// - The logs/ directory is created on startup if missing; file output uses a non-blocking writer.
// - File layer disables ANSI to keep logs clean for analysis.
use std::fs;
use std::path::Path;
use std::time::Duration;
use std::time::SystemTime;

use clap::Parser;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use coldvox_app::runtime::{self as app_runtime, ActivationMode as RuntimeMode, AppRuntimeOptions};
use coldvox_app::Settings;
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

/// Prune rotated log files in `logs/` older than `retention_days` days.
/// If `retention_days` is `Some(0)` pruning is disabled. Default is 7 days when `None`.
fn prune_old_logs(retention_days: Option<u64>) {
    let retention = retention_days.unwrap_or(7);
    if retention == 0 {
        tracing::debug!("Log retention disabled (retention_days=0)");
        return;
    }

    let cutoff = match SystemTime::now().checked_sub(Duration::from_secs(retention * 24 * 60 * 60))
    {
        Some(t) => t,
        None => return,
    };

    let logs_dir = Path::new("logs");
    if !logs_dir.exists() {
        return;
    }

    match fs::read_dir(logs_dir) {
        Ok(entries) => {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                    // Only consider rotated files with date suffix like `coldvox.log.YYYY-MM-DD`
                    if name.starts_with("coldvox.log.") {
                        if let Ok(meta) = entry.metadata() {
                            if let Ok(modified) = meta.modified() {
                                if modified < cutoff {
                                    if let Err(e) = fs::remove_file(&path) {
                                        tracing::warn!(
                                            "Failed to remove old log {}: {}",
                                            path.display(),
                                            e
                                        );
                                    } else {
                                        tracing::info!("Removed old log file: {}", path.display());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Err(e) => tracing::warn!("Failed to read logs directory for pruning: {}", e),
    }
}

#[derive(Parser, Debug)]
#[command(name = "coldvox", author, version, about = "ColdVox voice pipeline")]
struct Cli {
    /// List available input devices and exit
    #[arg(long = "list-devices")]
    list_devices: bool,

    /// Enable TUI dashboard
    #[arg(long = "tui")]
    tui: bool,

    /// Exit immediately if all injection methods fail
    #[arg(long = "injection-fail-fast")]
    injection_fail_fast: bool,
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
    // Prune old rotated logs. Set COLDVOX_LOG_RETENTION_DAYS=0 to disable pruning.
    let retention_days = std::env::var("COLDVOX_LOG_RETENTION_DAYS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok());
    prune_old_logs(retention_days);
    tracing::info!("Starting ColdVox application");

    let cli = Cli::parse();
    let mut settings = Settings::new().unwrap_or_else(|e| {
        tracing::error!("Failed to load settings: {}", e);
        Settings::default()
    });

    // Override settings with CLI flags
    if cli.injection_fail_fast {
        settings.injection.fail_fast = true;
    }

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

    // Build STT configuration from settings
    let stt_selection = {
        use coldvox_stt::plugin::{FailoverConfig, GcPolicy, MetricsConfig, PluginSelectionConfig};

        let failover = FailoverConfig {
            failover_threshold: settings.stt.failover_threshold,
            failover_cooldown_secs: settings.stt.failover_cooldown_secs,
        };

        let gc_policy = GcPolicy {
            model_ttl_secs: settings.stt.model_ttl_secs,
            enabled: !settings.stt.disable_gc,
        };

        let metrics = MetricsConfig {
            log_interval_secs: if settings.stt.metrics_log_interval_secs == 0 {
                None
            } else {
                Some(settings.stt.metrics_log_interval_secs)
            },
            debug_dump_events: settings.stt.debug_dump_events,
        };

        Some(PluginSelectionConfig {
            preferred_plugin: settings.stt.preferred,
            fallback_plugins: settings.stt.fallbacks,
            require_local: settings.stt.require_local,
            max_memory_mb: settings.stt.max_mem_mb,
            required_language: settings.stt.language,
            failover: Some(failover),
            gc_policy: Some(gc_policy),
            metrics: Some(metrics),
            auto_extract_model: settings.stt.auto_extract,
        })
    };

    let mut opts = AppRuntimeOptions::default();
    opts.device = settings.device;
    opts.resampler_quality = match settings.resampler_quality.to_lowercase().as_str() {
        "fast" => ResamplerQuality::Fast,
        "quality" => ResamplerQuality::Quality,
        _ => ResamplerQuality::Balanced,
    };
    opts.activation_mode = match settings.activation_mode.as_str() {
        "vad" => RuntimeMode::Vad,
        "hotkey" => RuntimeMode::Hotkey,
        _ => RuntimeMode::Vad,
    };
    opts.stt_selection = stt_selection;
    #[cfg(feature = "text-injection")]
    {
        opts.injection = if cfg!(feature = "text-injection") {
            Some(coldvox_app::runtime::InjectionOptions {
                enable: true, // Assuming text injection is enabled if the feature is on
                allow_kdotool: settings.injection.allow_kdotool,
                allow_enigo: settings.injection.allow_enigo,
                inject_on_unknown_focus: settings.injection.inject_on_unknown_focus,
                max_total_latency_ms: Some(settings.injection.max_total_latency_ms),
                per_method_timeout_ms: Some(settings.injection.per_method_timeout_ms),
                cooldown_initial_ms: Some(settings.injection.cooldown_initial_ms),
                fail_fast: settings.injection.fail_fast,
            })
        } else {
            None
        };
    }
    opts.enable_device_monitor = settings.enable_device_monitor;

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
                tracing::debug!("Shutdown signal received");
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
                tracing::debug!("Shutdown signal received");
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
    tracing::debug!("Beginning graceful shutdown");
    state_manager.transition(AppState::Stopping)?;
    // Shutdown directly on the Arc<AppHandle>
    app.shutdown().await;
    state_manager.transition(AppState::Stopped)?;
    tracing::debug!("Shutdown complete");

    Ok(())
}

