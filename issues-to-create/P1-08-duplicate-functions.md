---
title: "[P1] Duplicate functions: _get_method_priority() and compute_method_order()"
labels: ["refactor", "priority:P1", "component:text-injection"]
---

## Problem

Two functions exist that compute method ordering with overlapping functionality:
- `_get_method_priority(app_id)` 
- `compute_method_order(app_id)`

This creates confusion, maintenance burden, and potential for divergent behavior.

## Current Behavior

In `crates/coldvox-text-injection/src/manager.rs`:

```rust
pub(crate) fn _get_method_priority(&self, app_id: &str) -> Vec<InjectionMethod> {
    // One implementation with sorting logic
    // ...
}

fn compute_method_order(&self, app_id: &str) -> Vec<InjectionMethod> {
    // Another implementation with similar sorting logic
    // ...
}
```

Both functions:
1. Build a base order of methods
2. Deduplicate with HashSet
3. Sort by position with `position()` iterator calls

## Expected Behavior

Should have a single, clear implementation:

```rust
/// Compute method order based on environment, config, and success rates
fn compute_method_order(&self, app_id: &str) -> Vec<InjectionMethod> {
    // Single source of truth
}

/// Public wrapper for external/test access
pub fn get_method_priority(&self, app_id: &str) -> Vec<InjectionMethod> {
    self.compute_method_order(app_id)
}
```

## Impact

- **Maintainability**: Changes must be made in two places
- **Correctness Risk**: Functions can diverge over time
- **Code Clarity**: Unclear which function should be used where
- **Testing**: Must test both functions even though they should be equivalent

## Location

- File: `crates/coldvox-text-injection/src/manager.rs`
- Functions: `_get_method_priority()`, `compute_method_order()`
- Related: `get_method_order_cached()`, `get_method_priority()` (public wrapper)

## Recommendation

1. Keep `compute_method_order()` as the single implementation
2. Remove `_get_method_priority()` 
3. Use `get_method_priority()` as public wrapper
4. Update all call sites
5. Ensure tests cover the unified function
