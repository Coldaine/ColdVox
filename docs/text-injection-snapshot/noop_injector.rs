use crate::types::{InjectionConfig, InjectionResult};
use crate::TextInjector;
use async_trait::async_trait;

/// NoOp injector that always succeeds but does nothing
/// Used as a fallback when no other injectors are available
pub struct NoOpInjector {
    _config: InjectionConfig,
}

impl NoOpInjector {
    /// Create a new NoOp injector
    pub fn new(config: InjectionConfig) -> Self {
        Self { _config: config }
    }
}

#[async_trait]
impl TextInjector for NoOpInjector {
    async fn inject_text(&self, text: &str) -> InjectionResult<()> {
        if text.is_empty() {
            return Ok(());
        }

        tracing::debug!("NoOp injector: would inject {} characters", text.len());
        Ok(())
    }

    async fn is_available(&self) -> bool {
        true // Always available as fallback
    }

    fn backend_name(&self) -> &'static str {
        "NoOp"
    }

    fn backend_info(&self) -> Vec<(&'static str, String)> {
        vec![
            ("type", "fallback".to_string()),
            (
                "description",
                "No-op injector that always succeeds".to_string(),
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_noop_injector_creation() {
        let config = InjectionConfig::default();
        let injector = NoOpInjector::new(config);

        assert_eq!(injector.backend_name(), "NoOp");
        assert!(injector.is_available().await);
        assert!(!injector.backend_info().is_empty());
    }

    #[tokio::test]
    async fn test_noop_inject_success() {
        let config = InjectionConfig::default();
        let injector = NoOpInjector::new(config);

        let result = injector.inject_text("test text").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_noop_inject_empty_text() {
        let config = InjectionConfig::default();
        let injector = NoOpInjector::new(config);

        let result = injector.inject_text("").await;
        assert!(result.is_ok());
    }
}
