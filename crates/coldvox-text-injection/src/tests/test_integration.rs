#[cfg(test)]
mod integration_tests {
    use crate::manager::StrategyManager;
    use crate::types::{InjectionConfig, InjectionMetrics};
    use serial_test::serial;
    use std::sync::{Arc, Mutex};
    use tracing::{debug, info};

    /// Initialize tracing for tests with debug level - resilient to multiple calls
    fn init_test_tracing() {
        use std::sync::Once;
        use tracing_subscriber::{fmt, EnvFilter};

        static INIT: Once = Once::new();
        INIT.call_once(|| {
            let filter =
                EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug"));

            // Try to init, but ignore if already set to avoid panic
            let _ = fmt().with_env_filter(filter).with_test_writer().try_init();
        });
    }

    #[tokio::test]
    #[serial]
    async fn test_full_injection_flow() {
        init_test_tracing();
        info!("Starting test_full_injection_flow");
        let config = InjectionConfig {
            // clipboard restoration is automatic
            ..Default::default()
        };

        let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));
        let manager = StrategyManager::new(config, metrics.clone()).await;
        debug!("StrategyManager created successfully");

        // Test getting current app ID
        info!("Attempting to get current app ID...");
        let app_id = manager.get_current_app_id().await;
        debug!("get_current_app_id completed, result: {:?}", app_id);
        assert!(app_id.is_ok());
        let app_id = app_id.unwrap();
        info!("Current app ID: {}", app_id);

        // Test method priority
        info!("Attempting to get method priority...");
        let methods = manager.get_method_priority(&app_id);
        debug!("get_method_priority completed, result: {:?}", methods);
        assert!(
            !methods.is_empty(),
            "Should have at least one injection method available"
        );
        info!("Available methods: {:?}", methods);

        // Check metrics
        let metrics_guard = metrics.lock().unwrap();
        debug!(
            "Initial metrics: attempts={}, successes={}",
            metrics_guard.attempts, metrics_guard.successes
        );
    }

    #[tokio::test]
    async fn test_app_allowlist_blocklist() {
        let config = InjectionConfig {
            allowlist: vec!["firefox".to_string(), "chrome".to_string()],
            blocklist: vec!["terminal".to_string()],
            ..Default::default()
        };

        let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));
        let manager = StrategyManager::new(config, metrics).await;

        // Test allowlist
        assert!(manager.is_app_allowed("firefox"));
        assert!(manager.is_app_allowed("chrome"));
        assert!(!manager.is_app_allowed("notepad"));

        // Clear allowlist and test blocklist
        let config = InjectionConfig {
            blocklist: vec!["terminal".to_string(), "console".to_string()],
            ..Default::default()
        };

        let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));
        let manager = StrategyManager::new(config, metrics).await;

        assert!(!manager.is_app_allowed("terminal"));
        assert!(!manager.is_app_allowed("console"));
        assert!(manager.is_app_allowed("firefox"));
    }

    #[test]
    fn test_configuration_defaults() {
        let config = InjectionConfig::default();

        // Check default values
        assert!(!config.allow_kdotool);
        assert!(!config.allow_enigo);
        // Clipboard restoration is automatic; verify restore delay default exists
        assert!(config.clipboard_restore_delay_ms.is_some());
        assert!(config.inject_on_unknown_focus);
        assert!(config.enable_window_detection);

        assert_eq!(config.focus_cache_duration_ms, 200);
        assert_eq!(config.min_success_rate, 0.3);
        assert_eq!(config.min_sample_size, 5);
        assert_eq!(config.clipboard_restore_delay_ms, Some(500));

        assert!(config.allowlist.is_empty());
        assert!(config.blocklist.is_empty());
    }

    #[tokio::test]
    #[serial]
    async fn test_clipboard_restoration() {
        init_test_tracing();
        info!("Starting test_clipboard_restoration");

        // Set a known initial clipboard value
        let _initial_clipboard = "test_initial_clipboard_content";
        #[cfg(feature = "wl_clipboard")]
        {
            use wl_clipboard_rs::copy::{MimeType, Options, Source};
            let source = Source::Bytes(_initial_clipboard.as_bytes().to_vec().into());
            let opts = Options::new();
            let _ = opts.copy(source, MimeType::Text);
        }

        // Create config with short restore delay for testing
        let config = InjectionConfig {
            clipboard_restore_delay_ms: Some(100), // Short delay for test
            ..Default::default()
        };

        let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));
        let mut manager = StrategyManager::new(config, metrics.clone()).await;

        // Perform injection (this should temporarily change clipboard)
        let result = manager.inject("test_injection_text").await;
        debug!("Injection result: {:?}", result);

        // Wait for restoration delay + buffer
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // Verify clipboard was restored
        #[cfg(feature = "wl_clipboard")]
        {
            use std::io::Read;
            use wl_clipboard_rs::paste::{
                get_contents, ClipboardType, MimeType as PasteMimeType, Seat,
            };

            match get_contents(
                ClipboardType::Regular,
                Seat::Unspecified,
                PasteMimeType::Text,
            ) {
                Ok((mut pipe, _mime)) => {
                    let mut current_clipboard = String::new();
                    if pipe.read_to_string(&mut current_clipboard).is_ok() {
                        assert_eq!(
                            current_clipboard, _initial_clipboard,
                            "Clipboard should be restored to initial value after injection"
                        );
                        info!("Clipboard restoration verified successfully");
                    } else {
                        panic!("Failed to read restored clipboard content");
                    }
                }
                Err(e) => {
                    panic!("Failed to read clipboard after restoration: {}", e);
                }
            }
        }

        #[cfg(not(feature = "wl_clipboard"))]
        {
            info!("Clipboard restoration test skipped - wl_clipboard feature not enabled");
        }
    }
}
