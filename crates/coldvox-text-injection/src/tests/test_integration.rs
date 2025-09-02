#[cfg(test)]
mod integration_tests {
    use crate::manager::StrategyManager;
    use crate::types::{InjectionConfig, InjectionMetrics};
    use std::sync::{Arc, Mutex};

    #[tokio::test]
    async fn test_full_injection_flow() {
        let config = InjectionConfig {
            allow_ydotool: false, // Disable external dependencies for testing
            restore_clipboard: true,
            ..Default::default()
        };

        let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));
        let manager = StrategyManager::new(config, metrics.clone());

        // Test getting current app ID
        let app_id = manager.get_current_app_id().await;
        assert!(app_id.is_ok());
        let app_id = app_id.unwrap();
        println!("Current app ID: {}", app_id);

        // Test method priority
        let methods = manager.get_method_priority(&app_id);
        assert!(
            !methods.is_empty(),
            "Should have at least one injection method available"
        );
        println!("Available methods: {:?}", methods);

        // Check metrics
        let metrics_guard = metrics.lock().unwrap();
        println!(
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
        let manager = StrategyManager::new(config, metrics);

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
        let manager = StrategyManager::new(config, metrics);

        assert!(!manager.is_app_allowed("terminal"));
        assert!(!manager.is_app_allowed("console"));
        assert!(manager.is_app_allowed("firefox"));
    }

    #[test]
    fn test_configuration_defaults() {
        let config = InjectionConfig::default();

        // Check default values
        assert!(!config.allow_ydotool);
        assert!(!config.allow_kdotool);
        assert!(!config.allow_enigo);
        assert!(!config.allow_mki);
        assert!(!config.restore_clipboard);
        assert!(config.inject_on_unknown_focus);
        assert!(config.enable_window_detection);

        assert_eq!(config.focus_cache_duration_ms, 200);
        assert_eq!(config.min_success_rate, 0.3);
        assert_eq!(config.min_sample_size, 5);
        assert_eq!(config.clipboard_restore_delay_ms, Some(500));

        assert!(config.allowlist.is_empty());
        assert!(config.blocklist.is_empty());
    }
}
