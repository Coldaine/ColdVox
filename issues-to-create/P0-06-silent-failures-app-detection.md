---
title: "[P0] Silent failures in app detection: get_current_app_id() returns \"unknown\""
labels: ["bug", "priority:P0", "component:text-injection"]
---

## Problem

The `get_current_app_id()` method returns `Ok("unknown".to_string())` instead of propagating errors, making it impossible for callers to distinguish between "app is genuinely unknown" and "app detection failed".

## Current Behavior

In `crates/coldvox-text-injection/src/manager.rs`:

```rust
pub(crate) async fn get_current_app_id(&self) -> Result<String, InjectionError> {
    // ... AT-SPI code that may fail ...
    Ok("unknown".to_string())  // Always returns Ok, even on failure
}
```

## Expected Behavior

Should return proper errors:

```rust
pub(crate) async fn get_current_app_id(&self) -> Result<String, InjectionError> {
    #[cfg(feature = "atspi")]
    {
        match self.try_atspi_app_id().await {
            Ok(app_id) => return Ok(app_id),
            Err(e) => warn!("AT-SPI app detection failed: {}", e),
        }
    }
    
    #[cfg(feature = "x11")]
    {
        match self.try_x11_app_id().await {
            Ok(app_id) => return Ok(app_id),
            Err(e) => warn!("X11 app detection failed: {}", e),
        }
    }
    
    // Only return "unknown" when all methods have been tried
    Ok("unknown".to_string())
}
```

Or use a proper fallback strategy with detailed error types:

```rust
pub enum AppIdResult {
    Known(String),
    Unknown,
    DetectionFailed(InjectionError),
}
```

## Impact

- **Correctness**: Cannot distinguish real failures from unknown apps
- **Debugging**: Impossible to diagnose app detection issues
- **Metrics**: Cannot track app detection failure rates
- **Per-app Strategy**: Falls back to "unknown" even when detection should work

## Location

- File: `crates/coldvox-text-injection/src/manager.rs`
- Function: `get_current_app_id()`

## Related Issues

- Blocks proper implementation of per-app cooldowns (P0-01, P0-02)
- Affects success rate tracking accuracy
