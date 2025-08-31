#[cfg(test)]
mod tests {
    use coldvox_app::text_injection::manager::StrategyManager;
    use coldvox_app::text_injection::types::{InjectionConfig, InjectionMetrics};
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::Mutex;

    #[tokio::test]
    async fn test_text_injection_end_to_end() {
        // Create configuration
        let config = InjectionConfig::default();
        
        // Create shared metrics
        let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));
        
        // Create strategy manager
        let mut manager = StrategyManager::new(config, metrics.clone());
        
        // Test with normal text
        let result = manager.inject("Hello world").await;
        assert!(result.is_ok(), "Should successfully inject text");
        
        // Verify metrics
        let metrics_guard = metrics.lock().await;
        assert_eq!(metrics_guard.successes, 1, "Should record one success");
        assert_eq!(metrics_guard.attempts, 1, "Should record one attempt");
        assert_eq!(metrics_guard.failures, 0, "Should have no failures");
        
        // Test with empty text
        let result = manager.inject("").await;
        assert!(result.is_ok(), "Should handle empty text gracefully");
        
        // Test with long text
        let long_text = "This is a longer text that should be injected successfully. ".repeat(10);
        let result = manager.inject(&long_text).await;
        assert!(result.is_ok(), "Should handle long text");
        
        // Verify metrics updated
        assert_eq!(metrics_guard.successes, 2, "Should have two successes");
        assert_eq!(metrics_guard.attempts, 2, "Should have two attempts");
    }

    #[tokio::test]
    async fn test_injection_with_failure_and_recovery() {
        // Create configuration
        let config = InjectionConfig::default();
        
        // Create shared metrics
        let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));
        
        // Create strategy manager
        let mut manager = StrategyManager::new(config, metrics.clone());
        
        // Force a failure by setting very short budget
        manager.config.max_total_latency_ms = 1;
        
        // This should fail due to budget exhaustion
        let result = manager.inject("Test text").await;
        assert!(result.is_err(), "Should fail due to budget exhaustion");
        
        // Verify metrics
        let metrics_guard = metrics.lock().await;
        assert_eq!(metrics_guard.failures, 1, "Should record one failure");
        assert_eq!(metrics_guard.attempts, 1, "Should record one attempt");
        
        // Reset budget to allow success
        manager.config.max_total_latency_ms = 800;
        
        // This should succeed
        let result = manager.inject("Test text").await;
        assert!(result.is_ok(), "Should succeed with adequate budget");
        
        // Verify metrics
        assert_eq!(metrics_guard.successes, 1, "Should record one success");
        assert_eq!(metrics_guard.attempts, 2, "Should have two attempts total");
    }

    #[tokio::test]
    async fn test_method_fallback_sequence() {
        // Create configuration
        let config = InjectionConfig::default();
        
        // Create shared metrics
        let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));
        
        // Create strategy manager
        let mut manager = StrategyManager::new(config, metrics.clone());
        
        // Temporarily disable all methods to force fallback sequence
        manager.config.allow_ydotool = false;
        manager.config.allow_kdotool = false;
        manager.config.allow_enigo = false;
        manager.config.allow_mki = false;
        
        // This should fail but try all methods in sequence
        let result = manager.inject("Test text").await;
        assert!(result.is_err(), "Should fail when no methods are available");
        
        // Verify that all methods were attempted
        let metrics_guard = metrics.lock().await;
        assert!(metrics_guard.attempts > 0, "Should attempt injection");
        
        // The specific number of attempts depends on available backends
        // but should be at least the base methods (AtspiInsert, ClipboardAndPaste, Clipboard)
        assert!(metrics_guard.attempts >= 3, "Should try at least 3 methods");
    }

    #[tokio::test]
    async fn test_cooldown_and_recovery() {
        // Create configuration
        let mut config = InjectionConfig::default();
        config.cooldown_initial_ms = 100; // Short cooldown for testing
        
        // Create shared metrics
        let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));
        
        // Create strategy manager
        let mut manager = StrategyManager::new(config, metrics.clone());
        
        // Force a failure
        manager.config.max_total_latency_ms = 1;
        let result = manager.inject("Test text").await;
        assert!(result.is_err(), "Should fail due to budget exhaustion");
        
        // Verify cooldown is active
    let method = manager.get_method_order_uncached()[0]; // First method should be in cooldown
        assert!(manager.is_in_cooldown(method), "Method should be in cooldown after failure");
        
        // Wait for cooldown to expire
        tokio::time::sleep(Duration::from_millis(150)).await;
        
        // Reset budget to allow success
        manager.config.max_total_latency_ms = 800;
        
        // This should succeed as cooldown has expired
        let result = manager.inject("Test text").await;
        assert!(result.is_ok(), "Should succeed after cooldown expires");
        
        // Verify cooldown is cleared after success
        assert!(!manager.is_in_cooldown(method), "Cooldown should be cleared after successful injection");
    }
}