use crate::types::{InjectionConfig, InjectionError, InjectionMetrics, TextInjector};
use async_trait::async_trait;

/// NoOp injector that always succeeds but does nothing
/// Used as a fallback when no other injectors are available
pub struct NoOpInjector {
    _config: InjectionConfig,
    metrics: InjectionMetrics,
}

impl NoOpInjector {
    /// Create a new NoOp injector
    pub fn new(config: InjectionConfig) -> Self {
        Self {
            _config: config,
            metrics: InjectionMetrics::default(),
        }
    }
}

#[async_trait]
impl TextInjector for NoOpInjector {
    fn name(&self) -> &'static str {
        "NoOp"
    }

    fn is_available(&self) -> bool {
        true // Always available as fallback
    }

    async fn inject(&mut self, text: &str) -> Result<(), InjectionError> {
        if text.is_empty() {
            return Ok(());
        }

        let start = std::time::Instant::now();

        // Record the operation but do nothing
        let duration = start.elapsed().as_millis() as u64;
        self.metrics
            .record_success(crate::types::InjectionMethod::NoOp, duration);

        tracing::debug!("NoOp injector: would inject {} characters", text.len());

        Ok(())
    }

    fn metrics(&self) -> &InjectionMetrics {
        &self.metrics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noop_injector_creation() {
        let config = InjectionConfig::default();
        let injector = NoOpInjector::new(config);

        assert_eq!(injector.name(), "NoOp");
        assert!(injector.is_available());
        assert_eq!(injector.metrics().attempts, 0);
    }

    #[tokio::test]
    async fn test_noop_inject_success() {
        let config = InjectionConfig::default();
        let mut injector = NoOpInjector::new(config);

        let result = injector.inject("test text").await;
        assert!(result.is_ok());

        // Check metrics
        let metrics = injector.metrics();
        assert_eq!(metrics.successes, 1);
        assert_eq!(metrics.attempts, 1);
        assert_eq!(metrics.failures, 0);
    }

    #[tokio::test]
    async fn test_noop_inject_empty_text() {
        let config = InjectionConfig::default();
        let mut injector = NoOpInjector::new(config);

        let result = injector.inject("").await;
        assert!(result.is_ok());

        // Should not record metrics for empty text
        let metrics = injector.metrics();
        assert_eq!(metrics.attempts, 0);
    }
}
