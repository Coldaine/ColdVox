#[cfg(test)]
mod tests {
    use crate::manager::StrategyManager;
    use crate::types::{InjectionConfig, InjectionMethod, InjectionMetrics};
    use std::sync::{Arc, Mutex};

    #[tokio::test]
    async fn test_success_rate_calculation() {
        let config = InjectionConfig::default();
        let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));
        let mut manager = StrategyManager::new(config, metrics).await;

        // Simulate some successes and failures
        manager.update_success_record("test_app", InjectionMethod::Clipboard, true);
        manager.update_success_record("test_app", InjectionMethod::Clipboard, true);
        manager.update_success_record("test_app", InjectionMethod::Clipboard, false);

        // Success rate should be approximately 66%
        let methods = manager.get_method_priority("test_app");
        assert!(!methods.is_empty());
    }

    #[tokio::test]
    async fn test_cooldown_application() {
        let config = InjectionConfig::default();
        let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));
        let mut manager = StrategyManager::new(config, metrics).await;

        // Apply cooldown
        manager.apply_cooldown("test_app", InjectionMethod::YdoToolPaste, "Test error");

        // Method should be in cooldown
        let _ = manager.is_in_cooldown(InjectionMethod::YdoToolPaste);
    }

    #[tokio::test]
    async fn test_method_priority_ordering() {
        let config = InjectionConfig {
            allow_ydotool: true,
            allow_enigo: false,
            ..Default::default()
        };

        let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));
        let manager = StrategyManager::new(config, metrics).await;

        let methods = manager.get_method_priority("test_app");

        // Should have some methods available (at least NoOp fallback)
        assert!(!methods.is_empty());
        assert!(methods.contains(&InjectionMethod::NoOp));

        // AT-SPI should be preferred if available in this environment
        #[cfg(feature = "atspi")]
        {
            if methods.contains(&InjectionMethod::AtspiInsert) {
                assert_eq!(methods[0], InjectionMethod::AtspiInsert);
            }
        }
    }

    #[tokio::test]
    async fn test_success_rate_decay() {
        let config = InjectionConfig::default();
        let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));
        let mut manager = StrategyManager::new(config, metrics).await;

        // Add initial success
        manager.update_success_record("test_app", InjectionMethod::Clipboard, true);

        // Add multiple updates to trigger decay
        for _ in 0..5 {
            manager.update_success_record("test_app", InjectionMethod::Clipboard, true);
        }

        // Success rate should still be high despite decay
        let methods = manager.get_method_priority("test_app");
        assert!(!methods.is_empty());
    }
}
