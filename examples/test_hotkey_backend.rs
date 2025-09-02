#[cfg(kde_globalaccel)]
use coldvox_app::hotkey::kglobalaccel;
use coldvox_app::hotkey::{backend, backend::HotkeyBackend};

#[tokio::main]
async fn main() {
    // Check which backend is available
    println!(
        "Testing hotkey backend availability on {}",
        std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_else(|_| "Unknown".to_string())
    );
    println!("KDE_FULL_SESSION: {:?}", std::env::var("KDE_FULL_SESSION"));
    println!("PLASMA_SESSION: {:?}", std::env::var("PLASMA_SESSION"));

    // Test KGlobalAccel availability
    #[cfg(kde_globalaccel)]
    {
        let kglobal_available = kglobalaccel::KGlobalAccelBackend::is_available().await;
        println!("KGlobalAccel backend available: {}", kglobal_available);
    }
    #[cfg(not(kde_globalaccel))]
    {
        println!("KGlobalAccel backend not compiled in (not on KDE Plasma)");
    }

    // Portal backend removed; no portal availability test

    // Detect best backend
    let backend = backend::detect_best_backend().await;
    println!("Selected backend: {}", backend.name());
}
