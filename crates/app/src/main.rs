use coldvox_app::audio::*;
use coldvox_app::foundation::*;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt().with_env_filter("info").init();

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
