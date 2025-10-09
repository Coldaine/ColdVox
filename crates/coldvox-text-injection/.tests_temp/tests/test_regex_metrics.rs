#[cfg(feature = "regex")]
mod tests {
    use std::sync::{Arc, Mutex};

    use crate::{
        manager::StrategyManager,
        types::{InjectionConfig, InjectionMetrics},
    };

    #[tokio::test]
    async fn records_regex_cache_sizes_in_metrics() {
        let config = InjectionConfig {
            allowlist: vec!["^Code$".into(), "^Terminal$".into()],
            blocklist: vec!["^Forbidden$".into()],
            ..Default::default()
        };
        let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));

        let _manager = StrategyManager::new(config, metrics.clone()).await;

        let m = metrics.lock().unwrap();
        assert_eq!(m.allowlist_regex_count, 2);
        assert_eq!(m.blocklist_regex_count, 1);
    }
}
