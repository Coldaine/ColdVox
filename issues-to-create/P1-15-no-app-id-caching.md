---
title: "[P1] No app_id caching: spawns processes every call without TTL cache"
labels: ["performance", "priority:P1", "component:text-injection"]
---

## Problem

`get_current_app_id()` spawns system processes (xdotool, wmctrl, etc.) on every call without any caching, causing unnecessary overhead. Window focus doesn't change rapidly enough to justify per-call detection.

## Current Behavior

In `crates/coldvox-text-injection/src/manager.rs`:

```rust
pub(crate) async fn get_current_app_id(&self) -> Result<String, InjectionError> {
    // Spawns process every single call
    #[cfg(feature = "atspi")]
    {
        // AT-SPI queries every time
    }
    
    // Falls back to process spawning
    let output = Command::new("xdotool")
        .arg("getactivewindow")
        .output()
        .await?;
    // ...
}
```

## Expected Behavior

Implement TTL cache for app_id:

```rust
pub struct StrategyManager {
    // ... existing fields ...
    
    app_id_cache: Option<(String, Instant)>,
    app_id_cache_duration: Duration,
}

pub(crate) async fn get_current_app_id(&self) -> Result<String, InjectionError> {
    // Check cache
    if let Some((cached_id, cached_at)) = &self.app_id_cache {
        if cached_at.elapsed() < self.app_id_cache_duration {
            return Ok(cached_id.clone());
        }
    }
    
    // Cache miss - detect app
    let app_id = self.detect_current_app_id().await?;
    
    // Update cache
    self.app_id_cache = Some((app_id.clone(), Instant::now()));
    
    Ok(app_id)
}
```

## Impact

**Performance Costs**:
- Process spawn: ~5-20ms overhead per call
- AT-SPI queries: ~2-10ms overhead per call
- Typical injection might call this 2-3 times

**Call Frequency**:
- Called in `inject()` for app-specific strategies
- Called in cooldown checks (after P0 fixes)
- Called for metrics/logging

At 10 injections/second:
- Current: 20-60 process spawns/sec = 100-1200ms CPU time
- With cache: 0-1 spawns/sec = 0-20ms CPU time

## Recommended Cache Duration

Balance freshness vs performance:
- **100ms**: Very fresh, still saves 90% of calls
- **250ms**: Good balance, saves 98% at 10 Hz injection
- **500ms**: More aggressive, acceptable for most use cases
- **1000ms**: Maximum reasonable value

Recommended: **250ms** (configurable)

## Configuration

Add to `InjectionConfig`:
```rust
pub struct InjectionConfig {
    // ... existing fields ...
    
    /// How long to cache the current app ID (milliseconds)
    pub app_id_cache_duration_ms: u64,  // default: 250
}
```

## Location

- File: `crates/coldvox-text-injection/src/manager.rs`
- Function: `get_current_app_id()`
- Struct: `StrategyManager` (needs cache fields)

## Notes

- Cache should be per-manager instance
- Consider invalidating cache on certain events (if detectable)
- Should handle concurrent access properly in async context
- Related to focus status caching in `focus.rs` (which already has TTL cache)
