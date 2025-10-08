---
title: "[P1] Unbatched metrics: individual locks throughout code"
labels: ["performance", "priority:P1", "component:text-injection"]
---

## Problem

Metrics are updated via individual mutex locks scattered throughout the code, creating unnecessary lock contention and overhead. Each metric update acquires and releases the lock separately.

## Current Behavior

In `crates/coldvox-text-injection/src/manager.rs`:

```rust
// Lock #1
if let Ok(mut metrics) = self.metrics.lock() {
    metrics.record_focus_missing();
}

// ... other code ...

// Lock #2 (separate acquisition)
if let Ok(mut metrics) = self.metrics.lock() {
    metrics.record_injection_attempt();
}

// ... other code ...

// Lock #3
if let Ok(mut metrics) = self.metrics.lock() {
    metrics.record_method_used(method);
}
```

## Expected Behavior

Batch metrics updates to minimize lock acquisitions:

```rust
// Approach 1: Batch updates
{
    let mut metrics = self.metrics.lock().unwrap_or_else(|p| p.into_inner());
    metrics.record_focus_missing();
    metrics.record_injection_attempt();
    metrics.record_method_used(method);
}  // Lock released once

// Approach 2: Use channel-based metrics collector
self.metrics_tx.send(MetricEvent::FocusMissing).ok();
self.metrics_tx.send(MetricEvent::InjectionAttempt).ok();
self.metrics_tx.send(MetricEvent::MethodUsed(method)).ok();

// Approach 3: Use atomic counters for simple metrics
self.metrics.focus_missing_count.fetch_add(1, Ordering::Relaxed);
```

## Impact

**Performance**:
- Each lock acquisition has syscall overhead
- Lock contention when multiple threads access metrics
- Cache line bouncing on multi-core systems

**Scalability**:
- Becomes bottleneck with high injection rates
- Affects latency of injection operations

## Measurements

Typical injection path has 5-7 metrics updates:
- Current: 5-7 lock acquisitions = 5-7 syscalls
- Batched: 1 lock acquisition = 1 syscall
- Channel-based: 5-7 channel sends (lock-free)
- Atomic: 5-7 atomic increments (lock-free)

## Location

- File: `crates/coldvox-text-injection/src/manager.rs`
- Throughout: Multiple `self.metrics.lock()` calls

## Recommendations

1. **Short term**: Batch metrics in hot paths
2. **Medium term**: Use atomics for counters, mutex for complex metrics
3. **Long term**: Implement lock-free metrics collector with channel

Priority: Start with batching in the `inject()` method (main hot path)
