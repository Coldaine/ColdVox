// cargo run -p coldvox-app --example text_injection_demo --features "text-injection,text-injection-clipboard"
// or swap/append features as you like.

use coldvox_app::text_injection::{InjectionConfig, StrategyManager};
use std::sync::{Arc, Mutex};
use coldvox_app::text_injection::types::InjectionMetrics;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .without_time()
        .init();

    let cfg = InjectionConfig {
        inject_on_unknown_focus: true,
        allow_ydotool: false,
        restore_clipboard: true,
        max_total_latency_ms: 900,
        ..Default::default()
    };

    let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));

    // This picks up whatever injectors are compiled in and available at runtime.
    let mut mgr = StrategyManager::new(cfg, metrics);

    let text = "Hello from ColdVox Phase 2 demo 👋";
    match mgr.inject(text).await {
        Ok(()) => println!("Injected successfully"),
        Err(e) => eprintln!("Injection failed: {e}"),
    }
}
