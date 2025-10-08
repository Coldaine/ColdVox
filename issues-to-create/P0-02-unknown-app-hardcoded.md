---
title: "[P0] \"unknown_app\" hardcoded in update_cooldown() and clear_cooldown()"
labels: ["bug", "priority:P0", "component:text-injection"]
---

## Problem

The `update_cooldown()` and `clear_cooldown()` methods hardcode `"unknown_app"` instead of using the actual current application ID. This causes all cooldowns to be tracked under a single fake app ID.

## Current Behavior

In `crates/coldvox-text-injection/src/manager.rs`:

```rust
fn update_cooldown(&mut self, method: InjectionMethod, error: &str) {
    // TODO(#38): This should use actual app_id from get_current_app_id()
    let app_id = "unknown_app";
    self.apply_cooldown(app_id, method, error);
}

fn clear_cooldown(&mut self, method: InjectionMethod) {
    let app_id = "unknown_app"; // Placeholder - would be from get_current_app_id
    let key = (app_id.to_string(), method);
    self.cooldowns.remove(&key);
}
```

## Expected Behavior

These methods should:
1. Call `get_current_app_id()` to obtain the real app ID
2. Use that app ID for cooldown tracking
3. Handle errors from `get_current_app_id()` appropriately

```rust
async fn update_cooldown(&mut self, method: InjectionMethod, error: &str) {
    let app_id = match self.get_current_app_id().await {
        Ok(id) => id,
        Err(e) => {
            warn!("Failed to get app_id for cooldown: {}", e);
            "unknown_app".to_string()
        }
    };
    self.apply_cooldown(&app_id, method, error);
}
```

## Impact

- **Critical**: Breaks per-app cooldown functionality completely
- **Data Integrity**: All apps share the same cooldown state
- **Reliability**: Success/failure tracking is meaningless when all apps are conflated

## Location

- File: `crates/coldvox-text-injection/src/manager.rs`
- Functions: `update_cooldown()`, `clear_cooldown()`

## Notes

There is a TODO comment referencing issue #38 for this fix.
