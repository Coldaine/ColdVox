use coldvox_app::audio::*;
use coldvox_app::foundation::*;
use std::time::Duration;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::fmt::writer::MakeWriterExt;

fn init_logging() -> Result<(), Box<dyn std::error::Error>> {
    // Create logs directory if it doesn't exist
    std::fs::create_dir_all("logs")?;

    // Set up file appender with daily rotation
    let file_appender = RollingFileAppender::new(Rotation::DAILY, "logs", "coldvox.log");
    let (non_blocking_file, _guard) = tracing_appender::non_blocking(file_appender);

    // Configure log level from environment or default to info
    let log_level = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());

    // Set up logging to both console and file
    tracing_subscriber::fmt()
        .with_writer(std::io::stdout.and(non_blocking_file))
        .with_env_filter(log_level)
        .init();

    // Keep guard alive for the entire program
    std::mem::forget(_guard);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize enhanced logging with file rotation
    init_logging()?;

    tracing::info!("Starting ColdVox application");

    // Create foundation components
    let state_manager = StateManager::new();
    let _health_monitor = HealthMonitor::new(Duration::from_secs(10)).start();
    let shutdown = ShutdownHandler::new().install().await;

    // Transition to running state
    state_manager.transition(AppState::Running)?;
    tracing::info!("Application state: {:?}", state_manager.current());

    // Create audio capture with default config
    let audio_config = AudioConfig::default();
    let mut audio_capture = AudioCapture::new(audio_config)?;

    // Start audio capture
    if let Err(e) = audio_capture.start(None).await {
        tracing::error!("Failed to start audio capture: {}", e);
        return Err(e.into());
    }

    tracing::info!("Audio capture started successfully");

    // Main application loop
    let mut stats_interval = tokio::time::interval(Duration::from_secs(30));

    loop {
        tokio::select! {
            // Check for shutdown
            _ = shutdown.wait() => {
                tracing::info!("Shutdown signal received");
                break;
            }

            // Print periodic stats
            _ = stats_interval.tick() => {
                let stats = audio_capture.get_stats();
                tracing::info!(
                    "Audio stats: {} frames captured, {} dropped, {} disconnects, {} reconnects",
                    stats.frames_captured,
                    stats.frames_dropped,
                    stats.disconnections,
                    stats.reconnections
                );

                // Check for potential issues
                if let Some(age) = stats.last_frame_age {
                    if age > Duration::from_secs(5) {
                        tracing::warn!("No audio frames received for {:?}", age);
                    }
                }
            }

            // Handle audio recovery if needed
            _ = async {
                if audio_capture.get_watchdog().is_triggered() {
                    tracing::warn!("Audio watchdog triggered, attempting recovery");
                    if let Err(e) = audio_capture.recover().await {
                        tracing::error!("Audio recovery failed: {}", e);
                    }
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            } => {}
        }
    }

    // Graceful shutdown
    tracing::info!("Beginning graceful shutdown");
    state_manager.transition(AppState::Stopping)?;
    // Stop audio capture
    audio_capture.stop();

    // Give components time to clean up
    tokio::time::sleep(Duration::from_millis(500)).await;

    state_manager.transition(AppState::Stopped)?;
    tracing::info!("Shutdown complete");

    Ok(())
}
