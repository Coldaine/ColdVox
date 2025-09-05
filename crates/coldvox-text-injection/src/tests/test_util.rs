#[cfg(test)]
pub mod util {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use crate::types::InjectionMetrics;
    use crate::{
        FocusProvider, FocusStatus, InjectionConfig, InjectionError, InjectionMethod,
        StrategyManager, TextInjector,
    };
    use async_trait::async_trait;

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
        async fn inject_text(&self, _text: &str) -> crate::types::InjectionResult<()> {
            if self.latency_ms > 0 {
                tokio::time::sleep(Duration::from_millis(self.latency_ms)).await;
            }
            if self.should_succeed {
                Ok(())
            } else {
                Err(InjectionError::MethodFailed("mock fail".into()))
            }
        }
        async fn is_available(&self) -> bool {
            true
        }
        fn backend_name(&self) -> &'static str {
            self.name
        }
        fn backend_info(&self) -> Vec<(&'static str, String)> {
            vec![("type", "mock".into())]
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
        let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));
        let focus = Box::new(MockFocusProvider { status });
        StrategyManager::new_with_focus_provider(config, metrics, focus).await
    }
}
