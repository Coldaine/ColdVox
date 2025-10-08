---
title: "[P0] No timeouts on awaited operations (e.g., get_focus_status)"
labels: ["bug", "priority:P0", "component:text-injection"]
---

## Problem

Async operations like `focus_provider.get_focus_status().await` have no timeout wrappers, which can cause the injection pipeline to hang indefinitely if AT-SPI or other system services become unresponsive.

## Current Behavior

In `crates/coldvox-text-injection/src/manager.rs`:

```rust
let focus_status = match self.focus_provider.get_focus_status().await {
    Ok(status) => status,
    Err(e) => {
        warn!("Failed to get focus status: {}", e);
        FocusStatus::Unknown
    }
};
```

## Expected Behavior

Should wrap with a timeout:

```rust
use tokio::time::{timeout, Duration};

let focus_status = match timeout(
    Duration::from_millis(500),
    self.focus_provider.get_focus_status()
).await {
    Ok(Ok(status)) => status,
    Ok(Err(e)) => {
        warn!("Failed to get focus status: {}", e);
        FocusStatus::Unknown
    }
    Err(_) => {
        warn!("Focus status check timed out");
        FocusStatus::Unknown
    }
};
```

## Impact

- **Reliability**: Can cause complete pipeline hang
- **User Experience**: Application becomes unresponsive during injection
- **Production Risk**: No way to recover from hung AT-SPI calls

## Location

- File: `crates/coldvox-text-injection/src/manager.rs`
- Primary: `inject()` method's call to `get_focus_status()`
- Also check: `get_current_app_id()` and other async calls

## Recommended Timeout Values

- `get_focus_status()`: 500ms (UI should be fast)
- `get_current_app_id()`: 1000ms (may involve process spawning)
- `inject()` operations: 2000ms (includes clipboard and typing)

## Configuration

Consider making timeout values configurable via `InjectionConfig`.
