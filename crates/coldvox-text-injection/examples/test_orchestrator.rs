//! Test example for the Strategy Orchestrator
//!
//! This example demonstrates the use of the StrategyOrchestrator for text injection
//! across different desktop environments.

use coldvox_text_injection::{InjectionConfig, StrategyOrchestrator};
use tracing::{info, Level};
use tracing_subscriber::fmt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    fmt()
        .with_max_level(Level::DEBUG)
        .with_target(false)
        .init();

    info!("Testing Strategy Orchestrator");

    // Create a default configuration
    let config = InjectionConfig::default();
    
    // Create the orchestrator
    let orchestrator = StrategyOrchestrator::new(config).await;
    
    // Print environment information
    info!("Detected environment: {}", orchestrator.desktop_environment());
    
    // Print backend information
    for (key, value) in orchestrator.backend_info() {
        info!("{}: {}", key, value);
    }
    
    // Check if the orchestrator is available
    if orchestrator.is_available().await {
        info!("Orchestrator is available for text injection");
    } else {
        info!("Orchestrator is not available in this environment");
    }
    
    // Test empty text injection
    let result = orchestrator.inject_text("").await;
    info!("Empty text injection result: {:?}", result);
    
    // Test basic text injection
    let result = orchestrator.inject_text("Hello, World!").await;
    info!("Basic text injection result: {:?}", result);
    
    Ok(())
}