#[cfg(kde_globalaccel)]
use coldvox_app::hotkey::kglobalaccel::KGlobalAccelBackend;
use coldvox_app::hotkey::{backend, backend::HotkeyBackend};
use coldvox_vad::types::VadEvent;
use tokio::sync::mpsc;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // Initialize logging with more detail
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug")),
        )
        .init();

    println!("\n=== ColdVox KGlobalAccel Hotkey Test ===\n");

    // Check environment
    println!(
        "Desktop Environment: {}",
        std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_else(|_| "Unknown".to_string())
    );
    println!("KDE_FULL_SESSION: {:?}", std::env::var("KDE_FULL_SESSION"));
    println!("PLASMA_SESSION: {:?}", std::env::var("PLASMA_SESSION"));
    println!();

    #[cfg(kde_globalaccel)]
    {
        // Check if KGlobalAccel is available
        let available = KGlobalAccelBackend::is_available().await;
        println!("KGlobalAccel backend available: {}\n", available);

        if !available {
            println!("KGlobalAccel service is not available on this system.");
            println!("Please ensure you're running KDE Plasma.");
            return;
        }

        // Create backend
        let mut backend = Box::new(KGlobalAccelBackend::new());

        // Initialize
        println!("Initializing KGlobalAccel backend...");
        if let Err(e) = backend.initialize().await {
            eprintln!("Failed to initialize backend: {}", e);
            return;
        }
        println!("✓ Backend initialized\n");

        // Register shortcut
        let shortcut = backend::Shortcut {
            id: "coldvox_ptt".to_string(),
            description: "ColdVox Push-to-talk".to_string(),
            default_keys: Some("Ctrl+Alt+Space".to_string()),
        };

        println!("Registering shortcut...");
        if let Err(e) = backend.register_shortcut(&shortcut).await {
            eprintln!("Failed to register shortcut: {}", e);
            return;
        }
        println!("✓ Shortcut registered\n");

        // Create channels for events
        let (event_tx, mut event_rx) = mpsc::channel::<VadEvent>(100);
        let (status_tx, mut status_rx) = mpsc::channel::<backend::BackendStatus>(100);

        // Start listening in background
        println!("Starting hotkey listener...");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("IMPORTANT: To test the hotkey:");
        println!("1. Open KDE System Settings → Shortcuts");
        println!("2. Search for 'coldvox' or look for 'ColdVox'");
        println!("3. Find the 'push_to_talk' action");
        println!("4. Assign a shortcut (e.g., Ctrl+Alt+Space)");
        println!("5. Press and release the shortcut to test");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("\nListening for hotkey events (press Ctrl+C to exit)...\n");

        let listener_handle = tokio::spawn(async move {
            if let Err(e) = backend.start_listening(event_tx, Some(status_tx)).await {
                eprintln!("Listener error: {}", e);
            }
        });

        // Monitor events
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(event) = event_rx.recv() => {
                        match event {
                            VadEvent::SpeechStart { timestamp_ms, .. } => {
                                println!("🎤 HOTKEY PRESSED at {}ms", timestamp_ms);
                            }
                            VadEvent::SpeechEnd { timestamp_ms, duration_ms, .. } => {
                                println!("🔇 HOTKEY RELEASED at {}ms (held for {}ms)",
                                    timestamp_ms, duration_ms);
                            }
                        }
                    }
                    Some(status) = status_rx.recv() => {
                        match status {
                            backend::BackendStatus::Connected => {
                                println!("✅ Backend connected to KGlobalAccel");
                            }
                            backend::BackendStatus::Disconnected => {
                                println!("⚠️ Backend disconnected from KGlobalAccel");
                            }
                            backend::BackendStatus::Error(e) => {
                                println!("❌ Backend error: {}", e);
                            }
                            backend::BackendStatus::ShortcutActivated(id) => {
                                println!("➡️ Shortcut activated: {}", id);
                            }
                            backend::BackendStatus::ShortcutDeactivated(id) => {
                                println!("⬅️ Shortcut deactivated: {}", id);
                            }
                            _ => {}
                        }
                    }
                }
            }
        });

        // Wait for Ctrl+C
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for Ctrl+C");
        println!("\n\nShutting down...");
        listener_handle.abort();
    }

    #[cfg(not(kde_globalaccel))]
    {
        println!("KGlobalAccel backend not compiled in.");
        println!("This test requires running on a KDE Plasma desktop.");
    }
}
