---
title: "[P1] No cache cleanup: unbounded success/cooldown caches"
labels: ["bug", "priority:P1", "component:text-injection"]
---

## Problem

The `success_records`, `cooldowns`, and `cached_method_order` HashMaps have no size limits or cleanup mechanisms, allowing unbounded memory growth as users interact with different applications.

## Current Behavior

In `crates/coldvox-text-injection/src/manager.rs`:

```rust
pub struct StrategyManager {
    success_records: HashMap<AppMethodKey, SuccessRecord>,
    cooldowns: HashMap<AppMethodKey, CooldownState>,
    cached_method_order: HashMap<String, Vec<InjectionMethod>>,
    // ... no cleanup logic anywhere
}
```

## Expected Behavior

Should implement one or more cleanup strategies:

### Option 1: LRU Cache with Size Limit
```rust
use lru::LruCache;

pub struct StrategyManager {
    success_records: LruCache<AppMethodKey, SuccessRecord>,
    cooldowns: LruCache<AppMethodKey, CooldownState>,
    cached_method_order: LruCache<String, Vec<InjectionMethod>>,
}
```

### Option 2: Periodic Cleanup of Old Entries
```rust
fn cleanup_old_records(&mut self) {
    let cutoff = Instant::now() - Duration::from_secs(3600);
    
    self.success_records.retain(|_, record| {
        record.last_success.map_or(false, |t| t > cutoff) ||
        record.last_failure.map_or(false, |t| t > cutoff)
    });
    
    self.cooldowns.retain(|_, state| state.until > Instant::now());
}
```

### Option 3: Maximum Size with Eviction
```rust
const MAX_CACHE_ENTRIES: usize = 100;

fn maybe_evict_old_entries(&mut self) {
    if self.success_records.len() > MAX_CACHE_ENTRIES {
        // Evict entries with oldest activity
        // ...
    }
}
```

## Impact

**Memory Leak**:
- Each unique app creates entries in all three caches
- Entries never removed even after app closes
- With 100 apps × 5 methods = 500 entries in success_records
- Could grow to MB of memory over long sessions

**Example Growth**:
- 1 hour with 10 apps: ~50 entries
- 8 hour session with 50 apps: ~250 entries
- Week-long session: potentially 1000+ entries

## Location

- File: `crates/coldvox-text-injection/src/manager.rs`
- Struct: `StrategyManager`
- Fields: `success_records`, `cooldowns`, `cached_method_order`

## Recommendations

1. **Immediate**: Implement cooldown cleanup (remove expired entries)
2. **Short term**: Add periodic cleanup of old success records
3. **Medium term**: Migrate to LRU caches with reasonable limits
4. **Long term**: Add configuration for cache sizes

Suggested limits:
- `success_records`: 200 entries (20 apps × 10 methods)
- `cooldowns`: 100 entries (temporary, should be cleared)
- `cached_method_order`: 50 entries (one per app)
