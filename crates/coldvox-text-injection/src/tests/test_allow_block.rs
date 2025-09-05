use std::sync::{Arc, Mutex};

use crate::{
    manager::StrategyManager,
    types::{InjectionConfig, InjectionMetrics},
};

#[tokio::test]
async fn allow_block_with_regex_feature_or_substring() {
    // Case 1: allowlist present -> only allowed when pattern matches
    let mut config = InjectionConfig::default();
    config.allowlist = vec!["^Code$".into(), "^Terminal$".into()];
    let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));
    let manager = StrategyManager::new(config, metrics).await;

    // With regex feature: exact match expected; without regex feature: substring match OK
    #[cfg(feature = "regex")]
    {
        assert!(manager.is_app_allowed("Code"));
        assert!(!manager.is_app_allowed("SomeCodeWindow"));
        assert!(!manager.is_app_allowed("Forbidden"));
    }

    #[cfg(not(feature = "regex"))]
    {
        assert!(manager.is_app_allowed("SomeCodeWindow"));
        assert!(manager.is_app_allowed("Terminal"));
        assert!(!manager.is_app_allowed("Forbidden"));
    }
}

#[tokio::test]
async fn blocklist_only_behavior() {
    let mut config = InjectionConfig::default();
    config.blocklist = vec!["^Forbidden$".into(), "Games".into()];
    let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));
    let manager = StrategyManager::new(config, metrics).await;

    #[cfg(feature = "regex")]
    {
        assert!(!manager.is_app_allowed("Forbidden"));
        assert!(manager.is_app_allowed("Notepad"));
    }

    #[cfg(not(feature = "regex"))]
    {
        assert!(!manager.is_app_allowed("ArcadeGames"));
        assert!(manager.is_app_allowed("Notepad"));
    }
}
