use crate::error::InjectionError;
use crate::outcome::InjectionOutcome;
use crate::probe::BackendId;
use crate::types::InjectionConfig;
use crate::TextInjector;
use async_trait::async_trait;
use std::time::Instant;
use tracing::trace;

/// A fallback injector that does nothing and always succeeds.
pub struct NoOpInjector {
    _config: InjectionConfig,
}

impl NoOpInjector {
    pub fn new(config: InjectionConfig) -> Self {
        Self { _config: config }
    }
}

#[async_trait]
impl TextInjector for NoOpInjector {
    fn backend_id(&self) -> BackendId {
        BackendId::Fallback
    }

    async fn is_available(&self) -> bool {
        // The NoOp injector is always available.
        true
    }

    async fn inject_text(&self, text: &str) -> Result<InjectionOutcome, InjectionError> {
        let start = Instant::now();
        trace!(
            "NoOpInjector: pretending to inject {} characters.",
            text.len()
        );
        // Simulate a tiny amount of work.
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        Ok(InjectionOutcome {
            backend: self.backend_id(),
            latency_ms: start.elapsed().as_millis() as u32,
            degraded: false,
        })
    }
}
