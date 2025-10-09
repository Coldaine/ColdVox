//! Quick verification that Enigo implementation is live and functional

#[tokio::main]
async fn main() {
    println!("🔍 Testing Enigo Implementation...\n");

    #[cfg(feature = "enigo")]
    {
        use coldvox_text_injection::enigo_injector::EnigoInjector;

        // Create injector with default config
        let mut config = InjectionConfig::default();
        config.allow_enigo = true;

        let injector = EnigoInjector::new(config);

        // Check availability
        let is_available = injector.is_available().await;
        println!("✅ Enigo injector created successfully");
        println!("   Backend: {}", injector.backend_name());
        println!("   Available: {}", is_available);

        // Get backend info
        println!("\n📋 Backend Information:");
        for (key, value) in injector.backend_info() {
            println!("   {}: {}", key, value);
        }

        if is_available {
            println!("\n✅ Enigo is LIVE and ready to use!");
            println!("   Note: Actual text injection requires a target application with focus");
            std::process::exit(0);
        } else {
            println!("\n⚠️  Enigo library loaded but not available");
            println!("   This may be due to:");
            println!("   - Missing display server");
            println!("   - Insufficient permissions");
            println!("   - Platform-specific requirements");
            std::process::exit(1);
        }
    }

    #[cfg(not(feature = "enigo"))]
    {
        println!("❌ Enigo feature not enabled");
        println!("   Run with: cargo run --example test_enigo_live --features enigo");
        std::process::exit(1);
    }
}
