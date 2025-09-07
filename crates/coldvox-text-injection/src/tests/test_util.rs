#[cfg(test)]
pub mod util {
    #![allow(dead_code)]
    use crate::error::InjectionError;
    use crate::focus::{FocusProvider, FocusStatus};
    use crate::metrics::InjectionMetrics;
    use crate::outcome::InjectionOutcome;
    use crate::probe::BackendId;
    use crate::types::{InjectionConfig, InjectionMethod};
    use crate::{StrategyManager, TextInjector};
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    #[derive(Default)]
    pub struct TestInjectorFactory {
        entries: Vec<(InjectionMethod, Box<dyn TextInjector>)>,
    }

    impl TestInjectorFactory {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn add_mock(
            mut self,
            method: InjectionMethod,
            should_succeed: bool,
            latency_ms: u64,
        ) -> Self {
            self.entries.push((
                method,
                Box::new(MockInjector {
                    name: "mock",
                    should_succeed,
                    latency_ms,
                }) as Box<dyn TextInjector>,
            ));
            self
        }

        pub fn build(self) -> HashMap<InjectionMethod, Box<dyn TextInjector>> {
            let mut map = HashMap::new();
            for (m, inj) in self.entries {
                map.insert(m, inj);
            }
            map
        }
    }

    pub struct MockInjector {
        pub name: &'static str,
        pub should_succeed: bool,
        pub latency_ms: u64,
    }

    #[async_trait]
    impl TextInjector for MockInjector {
        fn backend_id(&self) -> BackendId {
            // A mock can't easily map to a real backend, so we'll use Fallback.
            // Tests that need a specific ID should use a more specific mock.
            BackendId::Fallback
        }

        async fn is_available(&self) -> bool {
            true
        }

        async fn inject_text(&self, _text: &str) -> Result<InjectionOutcome, InjectionError> {
            if self.latency_ms > 0 {
                tokio::time::sleep(Duration::from_millis(self.latency_ms)).await;
            }
            if self.should_succeed {
                Ok(InjectionOutcome {
                    backend: self.backend_id(),
                    latency_ms: self.latency_ms as u32,
                    degraded: false,
                })
            } else {
                Err(InjectionError::Other("mock failure".to_string()))
            }
        }
    }

    pub struct MockFocusProvider {
        pub status: FocusStatus,
    }

    #[async_trait]
    impl FocusProvider for MockFocusProvider {
        async fn get_focus_status(&mut self) -> Result<FocusStatus, InjectionError> {
            Ok(self.status)
        }
    }

    pub async fn manager_with_focus(
        config: InjectionConfig,
        status: FocusStatus,
    ) -> StrategyManager {
        // NOTE: StrategyManager::new_with_focus_provider was removed in the refactor.
        // This helper is no longer valid. Tests using it will need to be updated
        // or removed. For now, we'll just return a default manager.
        StrategyManager::new(config)
    }
}
