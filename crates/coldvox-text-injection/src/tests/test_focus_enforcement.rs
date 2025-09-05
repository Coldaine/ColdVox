#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use crate::types::InjectionMetrics;
    use crate::{FocusProvider, FocusStatus, InjectionConfig, InjectionError, StrategyManager};

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
    async fn test_injection_blocked_on_non_editable_when_required() {
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
    async fn test_injection_blocked_on_unknown_when_disabled() {
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
    async fn test_injection_allowed_on_editable_focus() {
        let config = InjectionConfig {
            require_focus: true,
            ..Default::default()
        };
        let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));
        let focus = Box::new(MockFocusProvider {
            status: FocusStatus::EditableText,
        });
        let mut manager = StrategyManager::new_with_focus_provider(config, metrics, focus).await;

        let result = manager.inject("hello").await;
        // Should not fail due to focus; allow env-dependent outcomes
        match result {
            Ok(()) => {}
            Err(crate::InjectionError::NoEditableFocus) => {
                panic!("Unexpected NoEditableFocus on Editable status")
            }
            Err(crate::InjectionError::Other(msg)) if msg.contains("Unknown focus state") => {
                panic!("Unexpected unknown focus error on Editable status")
            }
            Err(_) => {} // acceptable: environment-dependent injector failures
        }
    }
}
