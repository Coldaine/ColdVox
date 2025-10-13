#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use crate::types::InjectionMetrics;
    use crate::{FocusProvider, FocusStatus, InjectionConfig, InjectionError, StrategyManager};
    use serial_test::serial;
    use tracing::{debug, info};

    /// Initialize tracing for tests with debug level - resilient to multiple calls
    fn init_test_tracing() {
        use std::sync::Once;
        use tracing_subscriber::{fmt, EnvFilter};

        static INIT: Once = Once::new();
        INIT.call_once(|| {
            let filter =
                EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug"));

            // Try to init, but ignore if already set to avoid panic
            let _ = fmt().with_env_filter(filter).with_test_writer().try_init();
        });
    }

    struct MockFocusProvider {
        status: FocusStatus,
    }

    #[async_trait::async_trait]
    impl FocusProvider for MockFocusProvider {
        async fn get_focus_status(&mut self) -> Result<FocusStatus, InjectionError> {
            Ok(self.status)
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_injection_blocked_on_non_editable_when_required() {
        init_test_tracing();
        let config = InjectionConfig {
            require_focus: true,
            ..Default::default()
        };
        let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));
        let focus = Box::new(MockFocusProvider {
            status: FocusStatus::NonEditable,
        });
        let mut manager = StrategyManager::new_with_focus_provider(config, metrics, focus).await;

        let result = manager.inject("hello").await;
        match result {
            Err(InjectionError::NoEditableFocus) => {}
            other => panic!("Expected NoEditableFocus, got {:?}", other),
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_injection_blocked_on_unknown_when_disabled() {
        init_test_tracing();
        let config = InjectionConfig {
            inject_on_unknown_focus: false,
            ..Default::default()
        };
        let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));
        let focus = Box::new(MockFocusProvider {
            status: FocusStatus::Unknown,
        });
        let mut manager = StrategyManager::new_with_focus_provider(config, metrics, focus).await;

        let result = manager.inject("hello").await;
        match result {
            Err(InjectionError::Other(msg)) => assert!(msg.contains("Unknown focus state")),
            other => panic!("Expected Other(Unknown focus...), got {:?}", other),
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_injection_allowed_on_editable_focus() {
        init_test_tracing();
        info!("Starting test_injection_allowed_on_editable_focus");
        let config = InjectionConfig {
            require_focus: true,
            ..Default::default()
        };
        let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));
        let focus = Box::new(MockFocusProvider {
            status: FocusStatus::EditableText,
        });
        debug!("MockFocusProvider created with EditableText status");

        info!("Creating StrategyManager with focus provider...");
        let mut manager = StrategyManager::new_with_focus_provider(config, metrics, focus).await;
        debug!("StrategyManager created successfully");

        info!("Attempting to inject 'hello'...");
        let result = manager.inject("hello").await;
        debug!("Injection completed, result: {:?}", result);
        // Should not fail due to focus; allow env-dependent outcomes
        match result {
            Ok(()) => {
                debug!("Injection successful");
            }
            Err(crate::InjectionError::NoEditableFocus) => {
                panic!("Unexpected NoEditableFocus on Editable status")
            }
            Err(crate::InjectionError::Other(msg)) if msg.contains("Unknown focus state") => {
                panic!("Unexpected unknown focus error on Editable status")
            }
            Err(e) => {
                debug!("Acceptable environment-dependent failure: {:?}", e);
            } // acceptable: environment-dependent injector failures
        }
    }
}
