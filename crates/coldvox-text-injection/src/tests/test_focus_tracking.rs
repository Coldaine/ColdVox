#[cfg(test)]
mod tests {
    use crate::focus::{FocusStatus, FocusTracker};
    use crate::tests::test_util::util::skip_if_headless_ci;
    use crate::types::InjectionConfig;
    use serial_test::serial;
    use std::time::Duration;
    use tokio::time::sleep;
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

    #[tokio::test]
    #[serial]
    async fn test_focus_detection() {
        if skip_if_headless_ci() {
            eprintln!("Skipping test_focus_detection: headless CI environment detected");
            return;
        }

        init_test_tracing();
        info!("Starting test_focus_detection");
        let config = InjectionConfig::default();
        let mut tracker = FocusTracker::new(config);
        debug!("FocusTracker created successfully");

        // Test focus detection
        info!("Attempting to get focus status...");
        let status = tracker.get_focus_status().await;
        debug!("get_focus_status completed, result: {:?}", status);
        assert!(status.is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_focus_cache_expiry() {
        if skip_if_headless_ci() {
            eprintln!("Skipping test_focus_cache_expiry: headless CI environment detected");
            return;
        }

        init_test_tracing();
        info!("Starting test_focus_cache_expiry");
        let config = InjectionConfig {
            focus_cache_duration_ms: 50, // Very short cache
            ..Default::default()
        };
        let mut tracker = FocusTracker::new(config);
        debug!("FocusTracker created successfully with 50ms cache");

        // Get initial status
        info!("Getting initial focus status...");
        let _status1 = tracker.get_focus_status().await.unwrap();
        debug!("Initial focus status retrieved");

        // Wait for cache to expire
        info!("Waiting for cache to expire (60ms)...");
        sleep(Duration::from_millis(60)).await;
        debug!("Cache expiry sleep completed");

        // This should trigger a new check
        info!("Getting focus status after cache expiry...");
        let _status2 = tracker.get_focus_status().await.unwrap();
        debug!("Focus status after cache expiry retrieved");
    }

    #[test]
    fn test_focus_status_equality() {
        assert_eq!(FocusStatus::EditableText, FocusStatus::EditableText);
        assert_ne!(FocusStatus::EditableText, FocusStatus::NonEditable);
        assert_ne!(FocusStatus::Unknown, FocusStatus::EditableText);
    }
}
