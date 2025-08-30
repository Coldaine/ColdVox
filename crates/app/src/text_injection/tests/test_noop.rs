use super::super::*;
use crate::text_injection::noop_injector::NoOpInjector;

#[test]
fn noop_always_available_and_succeeds() {
    let config = InjectionConfig::default();
    let mut injector = NoOpInjector::new(config);
    assert!(injector.is_available());
    assert!(injector.inject("hello").is_ok());
}
