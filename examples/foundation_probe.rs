use clap::Parser;
use coldvox_foundation::*;
use std::time::Duration;

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "60")]
    duration: u64,

    #[arg(long)]
    simulate_panics: bool,

    #[arg(long)]
    simulate_errors: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Initialize tracing
    tracing_subscriber::fmt().with_env_filter("debug").init();

    // Create foundation components
    let state_manager = StateManager::new();
    let _health_monitor = HealthMonitor::new(Duration::from_secs(5));
    let shutdown = ShutdownHandler::new().install().await;

    // Test state transitions
    state_manager.transition(AppState::Running)?;

    if args.simulate_errors {
        // Simulate various errors
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(10)).await;
                tracing::error!("Simulated error!");
                // Test recovery
            }
        });
    }

    if args.simulate_panics {
        std::thread::spawn(|| {
            std::thread::sleep(Duration::from_secs(15));
            panic!("Simulated panic for testing!");
        });
    }

    // Run for specified duration
    tokio::select! {
        _ = tokio::time::sleep(Duration::from_secs(args.duration)) => {
            tracing::info!("Test duration reached");
        }
        _ = shutdown.wait() => {
            tracing::info!("Shutdown requested");
        }
    }

    // Clean shutdown
    state_manager.transition(AppState::Stopping)?;
    tokio::time::sleep(Duration::from_secs(1)).await;
    state_manager.transition(AppState::Stopped)?;

    println!("Test completed successfully");
    Ok(())
}
