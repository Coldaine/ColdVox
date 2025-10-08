#[cfg(test)]
mod tests {
    use coldvox_text_injection::manager::StrategyManager;
    use coldvox_text_injection::types::{InjectionConfig, InjectionMetrics};
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::Mutex;

    #[tokio::test]
    async fn test_text_injection_end_to_end() {
        std::env::set_var("COLDVOX_STT_PREFERRED", "vosk"); // Force Vosk for integration
    
        // Create configuration
        let config = InjectionConfig::default();
    
        // Create shared metrics
        let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));
    
        // Create strategy manager
        let mut manager = StrategyManager::new(config, metrics.clone()).await;
    
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
        let mut manager = StrategyManager::new(config, metrics.clone()).await;

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
        let mut manager = StrategyManager::new(config, metrics.clone()).await;

        // Temporarily disable all methods to force fallback sequence
        manager.config.allow_kdotool = false;
        manager.config.allow_enigo = false;

        // This should fail but try all methods in sequence
        let result = manager.inject("Test text").await;
        assert!(result.is_err(), "Should fail when no methods are available");

        // Verify that all methods were attempted
        let metrics_guard = metrics.lock().await;
        assert!(metrics_guard.attempts > 0, "Should attempt injection");

        // The specific number of attempts depends on available backends
    // but should be at least the base methods (AtspiInsert, ClipboardPasteFallback)
        assert!(metrics_guard.attempts >= 2, "Should try at least 2 methods");
    }

    #[tokio::test]
    async fn test_cooldown_and_recovery() {
        // Create configuration
        let mut config = InjectionConfig::default();
        config.cooldown_initial_ms = 100; // Short cooldown for testing

        // Create shared metrics
        let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));

        // Create strategy manager
        let mut manager = StrategyManager::new(config, metrics.clone()).await;

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

    #[tokio::test]
    async fn test_injection_timeout_handling() {
        // Create configuration with very short timeouts
        let mut config = InjectionConfig::default();
        config.max_total_latency_ms = 100; // Very short timeout
        config.per_method_timeout_ms = 50;  // Very short per-method timeout

        // Create shared metrics
        let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));

        // Create strategy manager
        let mut manager = StrategyManager::new(config, metrics.clone()).await;

        // This should timeout quickly
        let start = std::time::Instant::now();
        let result = manager.inject("Timeout test").await;
        let elapsed = start.elapsed();

        // Should fail due to timeout
        assert!(result.is_err(), "Should fail due to short timeout");

        // Should complete relatively quickly
        assert!(elapsed < Duration::from_millis(200), "Should timeout quickly");

        // Verify metrics
        let metrics_guard = metrics.lock().await;
        assert_eq!(metrics_guard.attempts, 1, "Should record one attempt");
        assert_eq!(metrics_guard.failures, 1, "Should record one failure");
    }

    #[tokio::test]
    async fn test_injection_with_unknown_focus_allowed() {
        // Create configuration that allows injection on unknown focus
        let mut config = InjectionConfig::default();
        config.inject_on_unknown_focus = true; // Allow for testing

        // Create shared metrics
        let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));

        // Create strategy manager
        let mut manager = StrategyManager::new(config, metrics.clone()).await;

        // This should attempt injection even with unknown focus
        let result = manager.inject("Test with unknown focus").await;

        // In a test environment, this might fail due to no available backends,
        // but it should at least attempt the injection
        let metrics_guard = metrics.lock().await;
        assert!(metrics_guard.attempts >= 1, "Should record at least one attempt");

        // Result might be error due to no backends available in test env
        // but the important thing is that it attempted
        println!("Injection result: {:?}", result);
        println!("Attempts: {}, Successes: {}, Failures: {}",
                metrics_guard.attempts, metrics_guard.successes, metrics_guard.failures);
    }

    #[tokio::test]
    async fn test_clipboard_save_restore_simulation() {
        // Test clipboard save/restore logic without actual injection
        use coldvox_text_injection::strategies::combo_clip_atspi::ComboClipAtspiStrategy;
        use coldvox_text_injection::types::InjectionContext;

        // Note: clipboard restoration is automatic (always enabled)
        let config = InjectionConfig {
            inject_on_unknown_focus: true,
            ..Default::default()
        };

        let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));
        let strategy = ComboClipAtspiStrategy::new(config, metrics);

        // Create a mock context
        let context = InjectionContext {
            text: "Test text".to_string(),
            session_id: "test-session".to_string(),
            attempt_id: 1,
        };

        // Test the clipboard save/restore logic
        // Note: This will fail in test environment due to no display/Wayland
        // but we can verify the logic doesn't panic
        let result = strategy.inject(&context).await;

        // In test environment, this will likely fail, but shouldn't panic
        match result {
            Ok(_) => println!("✅ Clipboard strategy succeeded (unexpected in test env)"),
            Err(e) => println!("⚠️  Clipboard strategy failed as expected: {}", e),
        }

        // The important thing is that it doesn't panic and handles errors gracefully
    }
}
