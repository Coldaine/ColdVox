// cargo run -p coldvox-app --example text_injection_demo --features "text-injection,text-injection-clipboard"
// or swap/append features as you like.

use coldvox_app::text_injection::{InjectionConfig, StrategyManager};

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .without_time()
        .init();

    let cfg = InjectionConfig {
        silence_timeout_ms: 600,
        inject_on_unknown_focus: true,
        allow_ydotool: false,
        restore_clipboard: true,
        max_total_latency_ms: 900,
    };

    // This picks up whatever injectors are compiled in and available at runtime.
    let mgr = StrategyManager::with_default_order(cfg);

    let text = "Hello from ColdVox Phase 2 demo 👋";
    match mgr.try_inject(text) {
        Ok(()) => println!("Injected successfully"),
        Err(e) => eprintln!("Injection failed: {e}"),
    }
}
