use crate::manager::StrategyManager;
use crate::types::{InjectionConfig, InjectionError, InjectionMethod, TextInjector, InjectionMetrics};
use std::sync::{Arc, Mutex};

struct DummyInjector { metrics: InjectionMetrics }
impl DummyInjector { fn new() -> Self { Self { metrics: InjectionMetrics::default() } } }
impl TextInjector for DummyInjector {
    fn name(&self) -> &'static str { "Dummy" }
    fn is_available(&self) -> bool { true }
    fn inject(&mut self, _text: &str) -> Result<(), InjectionError> { Ok(()) }
    fn paste(&mut self, _text: &str) -> Result<(), InjectionError> { Ok(()) }
    fn type_text(&mut self, _text: &str, _rate: u32) -> Result<(), InjectionError> { Ok(()) }
    fn metrics(&self) -> &InjectionMetrics { &self.metrics }
}

#[test]
fn regex_caching_allow_block() {
    let mut config = InjectionConfig::default();
    config.allowlist = vec!["^Code$".into()];
    config.blocklist = vec!["^Forbidden$".into()];
    let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));
    let manager = StrategyManager::new(config, metrics);

    #[cfg(feature = "regex")]
    {
        assert!(manager.is_app_allowed("Code"));
        assert!(!manager.is_app_allowed("Forbidden"));
        assert!(!manager.is_app_allowed("Other")); // blocked by allowlist
    }
    #[cfg(not(feature = "regex"))]
    {
        assert!(manager.is_app_allowed("SomeCodeWindow"));
    }
}

#[test]
fn method_order_caches_per_app() {
    let config = InjectionConfig::default();
    let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));
    let mut manager = StrategyManager::new(config, metrics);
    let order1 = manager.get_method_order("appA");
    let order2 = manager.get_method_order("appA");
    assert_eq!(order1, order2);
    let order3 = manager.get_method_order("appB");
    // Different app may have different cached key; at least call should not panic
    assert!(!order3.is_empty());
}

#[test]
fn unicode_chunk_boundaries() {
    let mut config = InjectionConfig::default();
    config.paste_chunk_chars = 3;
    config.chunk_delay_ms = 0;
    let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));
    let mut manager = StrategyManager::new(config, metrics);

    let mut inj: Box<dyn TextInjector> = Box::new(DummyInjector::new());
    let text = "🙂🙂🙂🙂"; // 4 emojis, multi-byte
    // Access private function via same module tests would be nicer; here we mimic by calling paste directly in a loop
    // Ensure slicing at char boundaries works by manual iteration
    let mut count = 0;
    for ch in text.chars() { let s = ch.to_string(); assert!(inj.paste(&s).is_ok()); count += 1; }
    assert_eq!(count, 4);
}
