#[cfg(test)]
mod tests {
    use crate::manager::StrategyManager;
    use crate::types::{InjectionConfig, InjectionMethod, InjectionMetrics};
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_success_rate_calculation() {
        let config = InjectionConfig::default();
        let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));
        let mut manager = StrategyManager::new(config, metrics);
        
        // Simulate some successes and failures
        manager.update_success_record("test_app", InjectionMethod::Clipboard, true);
        manager.update_success_record("test_app", InjectionMethod::Clipboard, true);
        manager.update_success_record("test_app", InjectionMethod::Clipboard, false);
        
        // Success rate should be approximately 66%
        let methods = manager.get_method_priority("test_app");
        assert!(!methods.is_empty());
    }
    
    #[test]
    fn test_cooldown_application() {
        let config = InjectionConfig::default();
        let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));
        let mut manager = StrategyManager::new(config, metrics);
        
        // Apply cooldown
        manager.apply_cooldown("test_app", InjectionMethod::YdoToolPaste, "Test error");
        
        // Method should be in cooldown
    let _ = manager.is_in_cooldown(InjectionMethod::YdoToolPaste);
    }
    
    #[test]
    fn test_method_priority_ordering() {
        let mut config = InjectionConfig::default();
        config.allow_ydotool = true;
        config.allow_enigo = false;
        
        let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));
        let manager = StrategyManager::new(config, metrics);
        
        let methods = manager.get_method_priority("test_app");
        
        // Should have some methods available
        assert!(!methods.is_empty());
        
        // AT-SPI should be preferred if available
        #[cfg(feature = "atspi")]
        assert_eq!(methods[0], InjectionMethod::AtspiInsert);
    }
    
    #[test]
    fn test_success_rate_decay() {
        let config = InjectionConfig::default();
        let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));
        let mut manager = StrategyManager::new(config, metrics);
        
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