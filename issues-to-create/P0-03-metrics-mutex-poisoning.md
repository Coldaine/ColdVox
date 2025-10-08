---
title: "[P0] Metrics mutex poisoning not handled properly"
labels: ["bug", "priority:P0", "component:text-injection"]
---

## Problem

The code uses `if let Ok(m) = self.metrics.lock()` to handle mutex poisoning, which silently ignores poisoned mutex errors instead of handling them explicitly. This can hide critical threading issues.

## Current Behavior

Throughout `crates/coldvox-text-injection/src/manager.rs`:

```rust
if let Ok(mut metrics) = self.metrics.lock() {
    metrics.record_focus_missing();
}
// Silently continues if mutex is poisoned
```

## Expected Behavior

Should use explicit error handling:

```rust
match self.metrics.lock() {
    Ok(mut metrics) => {
        metrics.record_focus_missing();
    }
    Err(poisoned) => {
        warn!("Metrics mutex poisoned, recovering: {}", poisoned);
        let mut metrics = poisoned.into_inner();
        metrics.record_focus_missing();
    }
}
```

Or at minimum, log when mutex acquisition fails:

```rust
let mut metrics = self.metrics.lock().unwrap_or_else(|poisoned| {
    warn!("Metrics mutex poisoned, recovering");
    poisoned.into_inner()
});
metrics.record_focus_missing();
```

## Impact

- **Observability**: Silently loses metrics when mutex is poisoned
- **Debugging**: Makes it harder to detect threading issues
- **Reliability**: Hides serious bugs that could indicate wider problems

## Location

- File: `crates/coldvox-text-injection/src/manager.rs`
- Multiple locations where `self.metrics.lock()` is called

## Recommendation

Consider one of:
1. Recover from poisoned mutex and log warning
2. Convert to RwLock if read-heavy
3. Use atomic counters for simple metrics
4. At minimum: log when mutex lock fails
