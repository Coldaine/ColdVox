---
title: "[P0] Cooldowns not per-app: is_in_cooldown() checks any app"
labels: ["bug", "priority:P0", "component:text-injection"]
---

## Problem

The `is_in_cooldown()` method checks if a method is in cooldown globally, not per-app. This breaks the intended per-app cooldown behavior where different apps should have independent cooldown states.

## Current Behavior

In `crates/coldvox-text-injection/src/manager.rs`, the `is_in_cooldown()` method checks cooldowns across all apps:

```rust
pub(crate) fn is_in_cooldown(&self, method: InjectionMethod) -> bool {
    // Currently checks any app with the method, not per-app
    // ...
}
```

## Expected Behavior

The method should accept an `app_id` parameter and only check cooldowns for that specific application:

```rust
pub(crate) fn is_in_cooldown(&self, app_id: &str, method: InjectionMethod) -> bool {
    let key = (app_id.to_string(), method);
    if let Some(cooldown) = self.cooldowns.get(&key) {
        cooldown.until > Instant::now()
    } else {
        false
    }
}
```

## Impact

- **Correctness**: Apps that should be able to inject are blocked due to failures in other apps
- **User Experience**: Injection failures cascade across applications unnecessarily
- **Design Intent**: Violates the per-app adaptive strategy design

## Location

- File: `crates/coldvox-text-injection/src/manager.rs`
- Function: `is_in_cooldown()`

## Related Issues

- Part of the larger text-injection refactoring effort
- Depends on proper app_id detection (see related issue)
