---
title: "[P2] Missing targeted tests for cooldown per-app, cache invalidation, mocked time"
labels: ["testing", "priority:P2", "component:text-injection"]
---

## Problem

While some tests exist in `crates/coldvox-text-injection/src/tests/`, they don't cover critical scenarios and edge cases. Specifically missing are tests for per-app behavior, cache invalidation, time-dependent behavior, and failure scenarios.

## Current Test Coverage

In `crates/coldvox-text-injection/src/tests/test_adaptive_strategy.rs`:

```rust
#[tokio::test]
async fn test_cooldown_application() {
    // Basic test but doesn't verify per-app isolation
}

#[tokio::test]
async fn test_success_rate_calculation() {
    // Basic success rate test
}
```

## Missing Test Scenarios

### 1. Per-App Cooldown Isolation
```rust
#[tokio::test]
async fn test_cooldown_is_per_app() {
    let mut manager = /* ... */;
    
    // Method fails for app1
    manager.apply_cooldown("app1", method, "error");
    
    // Should still work for app2
    assert!(!manager.is_in_cooldown("app2", method));
    
    // Should be in cooldown for app1
    assert!(manager.is_in_cooldown("app1", method));
}
```

### 2. Cache Invalidation
```rust
#[tokio::test]
async fn test_cache_invalidated_on_success_update() {
    let mut manager = /* ... */;
    
    // Prime cache
    let order1 = manager.get_method_order_cached("app1");
    
    // Update success record
    manager.update_success_record("app1", method, false);
    
    // Cache should be invalidated
    let order2 = manager.get_method_order_cached("app1");
    assert_ne!(order1, order2);  // Should recompute
}
```

### 3. Mocked Time for Time-Dependent Behavior
```rust
#[tokio::test]
async fn test_cooldown_expires_after_duration() {
    // Need mock time to test without Thread::sleep
    let mut time_mock = MockTime::new();
    let mut manager = StrategyManager::with_time_source(config, time_mock.clone());
    
    manager.apply_cooldown("app1", method, "error");
    assert!(manager.is_in_cooldown("app1", method));
    
    // Advance time
    time_mock.advance(Duration::from_millis(1000));
    assert!(!manager.is_in_cooldown("app1", method));
}
```

### 4. App Detection Failure Handling
```rust
#[tokio::test]
async fn test_app_detection_failure_handling() {
    let mut manager = /* with mocked app detection */;
    
    // Inject when app detection fails
    let result = manager.inject("test").await;
    
    // Should handle gracefully, not panic
    assert!(result.is_err() || /* uses fallback */);
}
```

### 5. Focus Status Edge Cases
```rust
#[tokio::test]
async fn test_inject_on_unknown_focus_configurable() {
    let config = InjectionConfig {
        inject_on_unknown_focus: false,
        ..Default::default()
    };
    
    // Should reject injection when focus unknown
    let result = manager.inject("test").await;
    assert!(matches!(result, Err(InjectionError::Other(_))));
}
```

### 6. Method Ordering Stability
```rust
#[tokio::test]
async fn test_method_ordering_stable_across_calls() {
    let manager = /* ... */;
    
    let order1 = manager.get_method_priority("app1");
    let order2 = manager.get_method_priority("app1");
    
    // Without updates, order should be identical
    assert_eq!(order1, order2);
}
```

### 7. Exponential Backoff
```rust
#[tokio::test]
async fn test_exponential_backoff_increases_cooldown() {
    let mut manager = /* ... */;
    
    // First failure
    manager.apply_cooldown("app1", method, "error");
    let cooldown1 = get_cooldown_duration(&manager, "app1", method);
    
    // Second failure
    manager.apply_cooldown("app1", method, "error");
    let cooldown2 = get_cooldown_duration(&manager, "app1", method);
    
    assert!(cooldown2 > cooldown1);
}
```

### 8. Metrics Mutex Poisoning Recovery
```rust
#[tokio::test]
async fn test_metrics_recovery_from_poisoning() {
    // Poison the mutex
    // Verify system continues to function
}
```

### 9. Timeout Behavior
```rust
#[tokio::test]
async fn test_focus_check_timeout() {
    let mut manager = /* with slow focus provider */;
    
    let start = Instant::now();
    manager.inject("test").await;
    let elapsed = start.elapsed();
    
    // Should timeout, not hang forever
    assert!(elapsed < Duration::from_secs(2));
}
```

### 10. Cache Size Limits
```rust
#[tokio::test]
async fn test_cache_respects_size_limits() {
    let mut manager = /* ... */;
    
    // Add many entries
    for i in 0..1000 {
        let app_id = format!("app_{}", i);
        manager.update_success_record(&app_id, method, true);
    }
    
    // Cache should not grow unbounded
    assert!(manager.success_records.len() <= MAX_CACHE_SIZE);
}
```

## Test Infrastructure Needs

1. **Mock Time Source**: For testing time-dependent behavior without sleep
2. **Mock App Detection**: For testing app detection failures
3. **Mock Focus Provider**: For testing focus-related edge cases
4. **Test Helpers**: Builder pattern for test setup
5. **Property-Based Tests**: Using `proptest` for invariants

## Location

- Files: `crates/coldvox-text-injection/src/tests/*.rs`
- New file needed: `test_time_dependent.rs`, `test_per_app_isolation.rs`, `test_cache_behavior.rs`

## Priority

High priority tests:
1. Per-app cooldown isolation (critical for P0 fixes)
2. Cache invalidation (blocks performance work)
3. Timeout behavior (production reliability)

Medium priority:
4. Time-dependent behavior with mocks
5. Edge cases and error handling
6. Metrics behavior

## Impact

- **Regression Prevention**: Catch bugs during refactoring
- **Documentation**: Tests serve as usage examples
- **Confidence**: Enable safe refactoring of complex code
