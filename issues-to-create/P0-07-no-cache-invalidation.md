---
title: "[P0] No cache invalidation: update_success_record() doesn't clear cached_method_order"
labels: ["bug", "priority:P0", "component:text-injection"]
---

## Problem

When `update_success_record()` modifies success rates, it doesn't invalidate the `cached_method_order` cache. This means the method priority ordering becomes stale and doesn't reflect recent success/failure patterns.

## Current Behavior

In `crates/coldvox-text-injection/src/manager.rs`:

```rust
pub(crate) fn update_success_record(
    &mut self,
    app_id: &str,
    method: InjectionMethod,
    success: bool,
) {
    // Updates success_records
    // ...
    
    // BUG: Does NOT clear cached_method_order!
}
```

The cache is only populated, never invalidated:

```rust
pub(crate) fn get_method_order_cached(&mut self, app_id: &str) -> Vec<InjectionMethod> {
    if let Some(cached) = self.cached_method_order.get(app_id) {
        return cached.clone();  // Returns stale data!
    }
    
    let order = self.compute_method_order(app_id);
    self.cached_method_order.insert(app_id.to_string(), order.clone());
    order
}
```

## Expected Behavior

Should invalidate cache when success records change:

```rust
pub(crate) fn update_success_record(
    &mut self,
    app_id: &str,
    method: InjectionMethod,
    success: bool,
) {
    // Update success_records
    // ...
    
    // Invalidate cache for this app
    self.cached_method_order.remove(app_id);
}
```

Or use a more sophisticated approach:
1. Cache with TTL (time-to-live)
2. Cache version number that increments on updates
3. Lazy invalidation during method ordering

## Impact

- **Correctness**: Uses outdated method priority, leading to suboptimal injection strategy
- **Adaptive Behavior**: Defeats the purpose of adaptive method selection
- **Performance**: May repeatedly try methods that are known to fail recently

## Location

- File: `crates/coldvox-text-injection/src/manager.rs`
- Function: `update_success_record()`
- Related: `get_method_order_cached()`

## Recommendation

Option 1 (simplest): Remove cache entry on update
Option 2 (better): Implement TTL-based cache expiration
Option 3 (best): Use a proper cache invalidation strategy based on access patterns
