use coldvox_stt::TranscriptionEvent;
use coldvox_text_injection::{InjectionConfig, InjectionProcessor, StrategyManager};
use std::sync::{Arc, Mutex};
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    info!("Starting Text Injection Demo");
    info!("This demo shows how to use the ColdVox text injection system");
    info!("Make sure you have a text editor or application focused for injection");

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let demo_mode = args.get(1).map(|s| s.as_str()).unwrap_or("processor");

    match demo_mode {
        "processor" => run_processor_demo().await?,
        "direct" => run_direct_injection_demo().await?,
        "help" => {
            println!("Usage: cargo run --example inject_demo [mode]");
            println!("Modes:");
            println!("  processor - Full injection processor demo (default)");
            println!("  direct    - Direct strategy manager demo");
            println!("  help      - Show this help");
            return Ok(());
        }
        _ => {
            error!("Unknown demo mode: {}", demo_mode);
            return Ok(());
        }
    }

    Ok(())
}

async fn run_processor_demo() -> Result<(), Box<dyn std::error::Error>> {
    info!("Running Injection Processor Demo");
    info!("This demo simulates the full injection pipeline with session management");

    // Create injection configuration
    let config = InjectionConfig {
        allow_kdotool: false,
        allow_enigo: false,
        // clipboard restore is automatic
        inject_on_unknown_focus: false,
        max_total_latency_ms: 5000,
        per_method_timeout_ms: 2000,
        cooldown_initial_ms: 1000,
        ..Default::default()
    };

    // Create shared injection metrics and injection processor
    let injection_metrics = Arc::new(Mutex::new(
        coldvox_app::text_injection::types::InjectionMetrics::default(),
    ));
    let mut processor =
        InjectionProcessor::new(config.clone(), None, injection_metrics.clone()).await;

    info!(
        "Processor created. Current state: {:?}",
        processor.session_state()
    );

    // Simulate receiving transcriptions
    let test_transcriptions = [
        "Hello world",
        "This is a test of the text injection system",
        "It should automatically inject text when silence is detected",
        "Make sure you have a text editor focused",
    ];

    for (i, text) in test_transcriptions.iter().enumerate() {
        info!("Adding transcription {}: {}", i + 1, text);

        // Simulate receiving a final transcription
        processor.handle_transcription(TranscriptionEvent::Final {
            utterance_id: i as u64 + 1,
            text: text.to_string(),
            words: None,
        });

        info!("State after transcription: {:?}", processor.session_state());
        info!("Buffer preview: {}", processor.buffer_preview());

        // Wait a bit to simulate real-time processing
        sleep(Duration::from_millis(500)).await;
    }

    info!("Waiting for silence timeout to trigger injection...");
    info!("Current state: {:?}", processor.session_state());

    // Wait for the silence timeout (configured in session)
    sleep(Duration::from_millis(2500)).await;

    // Check if we should inject
    if let Some(text) = processor.prepare_injection() {
        info!("Ready to inject: '{}'", text);

        // In a real scenario, this would be handled by the async processor
        // For demo purposes, we'll create a temporary strategy manager
        let config_clone = config.clone();
        let mut temp_manager = StrategyManager::new(config_clone, injection_metrics.clone()).await;
        match temp_manager.inject(&text).await {
            Ok(()) => {
                info!("✅ Injection successful!");
                processor.record_injection_result(true);
            }
            Err(e) => {
                error!("❌ Injection failed: {}", e);
                processor.record_injection_result(false);
            }
        }
    } else {
        info!("No injection needed at this time");
    }

    // Show final metrics
    let metrics = processor.metrics();
    info!("Final Metrics:");
    info!("  Successful injections: {}", metrics.successful_injections);
    info!("  Failed injections: {}", metrics.failed_injections);
    info!("  Final state: {:?}", metrics.session_state);

    Ok(())
}

async fn run_direct_injection_demo() -> Result<(), Box<dyn std::error::Error>> {
    info!("Running Direct Injection Demo");
    info!("This demo shows direct usage of the StrategyManager");

    // Create injection configuration
    let config = InjectionConfig {
        allow_kdotool: false,
        allow_enigo: false,
        inject_on_unknown_focus: false,
        max_total_latency_ms: 5000,
        per_method_timeout_ms: 2000,
        cooldown_initial_ms: 1000,
        ..Default::default()
    };

    // Create shared injection metrics and strategy manager
    let injection_metrics = Arc::new(Mutex::new(
        coldvox_app::text_injection::types::InjectionMetrics::default(),
    ));
    let mut manager = StrategyManager::new(config, injection_metrics).await;

    info!("StrategyManager created");

    // Test different injection texts
    let test_texts = [
        "Direct injection test",
        "Hello from ColdVox!",
        "This demonstrates the text injection capabilities",
    ];

    for (i, text) in test_texts.iter().enumerate() {
        info!("Injecting text {}: '{}'", i + 1, text);

        match manager.inject(text).await {
            Ok(()) => {
                info!("✅ Text {} injected successfully", i + 1);
            }
            Err(e) => {
                warn!("⚠️  Text {} injection failed: {}", i + 1, e);
                // Continue with next text
            }
        }

        // Small delay between injections
        sleep(Duration::from_millis(1000)).await;
    }

    // Show injection statistics
    info!("Demo completed. Injection statistics:");
    manager.print_stats();

    Ok(())
}
