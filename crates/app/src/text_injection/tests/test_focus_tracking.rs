#[cfg(test)]
mod tests {
    use crate::text_injection::focus::{FocusStatus, FocusTracker};
    use crate::text_injection::types::InjectionConfig;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_focus_detection() {
        let config = InjectionConfig::default();
        let mut tracker = FocusTracker::new(config);

        // Test focus detection
        let status = tracker.get_focus_status().await;
        assert!(status.is_ok());

        // Test caching
        let cached = tracker.cached_focus_status();
        assert!(cached.is_some());
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
        assert!(tracker.cached_focus_status().is_some());

        // Wait for cache to expire
        sleep(Duration::from_millis(60)).await;

        // This should trigger a new check
        let _status2 = tracker.get_focus_status().await.unwrap();

        // Cache should be refreshed
        assert!(tracker.cached_focus_status().is_some());
    }

    #[test]
    fn test_focus_status_equality() {
        assert_eq!(FocusStatus::EditableText, FocusStatus::EditableText);
        assert_ne!(FocusStatus::EditableText, FocusStatus::NonEditable);
        assert_ne!(FocusStatus::Unknown, FocusStatus::EditableText);
    }
}
