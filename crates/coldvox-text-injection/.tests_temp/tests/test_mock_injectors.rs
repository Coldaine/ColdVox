#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use async_trait::async_trait;
    use crate::{InjectionConfig, InjectionError, InjectionMethod, StrategyManager, TextInjector};
    use crate::types::InjectionMetrics;

    struct MockInjector {
        name: &'static str,
        should_succeed: bool,
        latency_ms: u64,
        // track count of injections if needed in future
    }

    impl MockInjector {
        fn new(name: &'static str, should_succeed: bool, latency_ms: u64) -> Self {
            Self { name, should_succeed, latency_ms }
        }
    }

    #[async_trait]
    impl TextInjector for MockInjector {
        async fn inject_text(&self, _text: &str) -> crate::types::InjectionResult<()> {
            if self.latency_ms > 0 { tokio::time::sleep(Duration::from_millis(self.latency_ms)).await; }
            if self.should_succeed { Ok(()) } else { Err(InjectionError::MethodFailed("mock fail".into())) }
        }

        async fn is_available(&self) -> bool { true }
        fn backend_name(&self) -> &'static str { self.name }
        fn backend_info(&self) -> Vec<(&'static str, String)> { vec![("type", "mock".into())] }
    }

    #[tokio::test]
    async fn test_fallback_succeeds_on_second_injector() {
        let config = InjectionConfig::default();
        let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));
        let mut manager = StrategyManager::new(config, metrics.clone()).await;

        // First method fails, second succeeds
        let mut map: std::collections::HashMap<InjectionMethod, Box<dyn TextInjector>> = std::collections::HashMap::new();
    map.insert(InjectionMethod::AtspiInsert, Box::new(MockInjector::new("m1", false, 5)));
    map.insert(InjectionMethod::ClipboardPasteFallback, Box::new(MockInjector::new("m2", true, 0)));
        manager.override_injectors_for_tests(map);

        let result = manager.inject("hello world").await;
        assert!(result.is_ok());

        // Verify metrics recorded both attempts
        let m = metrics.lock().unwrap().clone();
        assert_eq!(m.attempts, 2);
        assert_eq!(m.successes, 1);
        assert_eq!(m.failures, 1);
    }

    #[tokio::test]
    async fn test_all_methods_fail() {
        let config = InjectionConfig::default();
        let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));
        let mut manager = StrategyManager::new(config, metrics).await;

        let mut map: std::collections::HashMap<InjectionMethod, Box<dyn TextInjector>> = std::collections::HashMap::new();
    map.insert(InjectionMethod::AtspiInsert, Box::new(MockInjector::new("m1", false, 0)));
    map.insert(InjectionMethod::ClipboardPasteFallback, Box::new(MockInjector::new("m2", false, 0)));
        manager.override_injectors_for_tests(map);

        let result = manager.inject("hello").await;
        assert!(matches!(result, Err(InjectionError::MethodFailed(_))));
    }
}
