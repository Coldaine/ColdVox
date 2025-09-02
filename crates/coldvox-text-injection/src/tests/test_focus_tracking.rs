#[cfg(test)]
mod tests {
    use crate::focus::{FocusStatus, FocusTracker};
    use crate::types::InjectionConfig;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_focus_detection() {
        let config = InjectionConfig::default();
        let mut tracker = FocusTracker::new(config);

        // Test focus detection
        let status = tracker.get_focus_status().await;
        assert!(status.is_ok());
    }

    #[tokio::test]
    async fn test_focus_cache_expiry() {
        let config = InjectionConfig {
            focus_cache_duration_ms: 50, // Very short cache
            ..Default::default()
        };
        let mut tracker = FocusTracker::new(config);

        // Get initial status
        let _status1 = tracker.get_focus_status().await.unwrap();

        // Wait for cache to expire
        sleep(Duration::from_millis(60)).await;

        // This should trigger a new check
        let _status2 = tracker.get_focus_status().await.unwrap();
    }

    #[test]
    fn test_focus_status_equality() {
        assert_eq!(FocusStatus::EditableText, FocusStatus::EditableText);
        assert_ne!(FocusStatus::EditableText, FocusStatus::NonEditable);
        assert_ne!(FocusStatus::Unknown, FocusStatus::EditableText);
    }
}