---
title: "[P0] Blocking runtime: std::process::Command in get_active_window_class()"
labels: ["bug", "priority:P0", "component:text-injection"]
---

## Problem

The code uses `std::process::Command` in async contexts, which blocks the tokio runtime thread pool. This can cause performance degradation and deadlocks.

## Current Behavior

In `crates/coldvox-text-injection/src/manager.rs`:

```rust
async fn get_active_window_class(&self) -> Result<String, InjectionError> {
    // Uses std::process::Command::output() which blocks
    // ...
}
```

## Expected Behavior

Should use `tokio::process::Command` for non-blocking execution:

```rust
use tokio::process::Command;

async fn get_active_window_class(&self) -> Result<String, InjectionError> {
    let output = Command::new("xdotool")
        .arg("getactivewindow")
        .arg("getwindowclassname")
        .output()
        .await
        .map_err(|e| InjectionError::Other(format!("Failed to run xdotool: {}", e)))?;
    
    // ... process output
}
```

Or use `tokio::task::spawn_blocking` for std operations:

```rust
use tokio::task;

async fn get_active_window_class(&self) -> Result<String, InjectionError> {
    task::spawn_blocking(|| {
        std::process::Command::new("xdotool")
            .arg("getactivewindow")
            .arg("getwindowclassname")
            .output()
    })
    .await
    .map_err(|e| InjectionError::Other(format!("Join error: {}", e)))?
    .map_err(|e| InjectionError::Other(format!("Command failed: {}", e)))?;
    
    // ... process output
}
```

## Impact

- **Performance**: Blocks tokio worker threads, reducing concurrency
- **Deadlock Risk**: Can cause runtime starvation if many concurrent calls
- **Best Practices**: Violates async Rust conventions

## Location

- File: `crates/coldvox-text-injection/src/manager.rs`
- Function: `get_active_window_class()`
- Also check: `get_current_app_id()` and other methods spawning processes

## Recommendation

Prefer `tokio::process::Command` when available. Use `spawn_blocking` only when truly necessary (e.g., for CPU-bound work or blocking syscalls).
