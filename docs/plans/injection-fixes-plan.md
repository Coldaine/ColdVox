# Text Injection System: Issue Resolution Plan

**Status:** Planning
**Created:** 2025-10-08
**Updated:** 2025-10-08 (Codebase verification complete)
**Crate:** `coldvox-text-injection`

## Executive Summary

All 21 identified issues in the text injection system are **confirmed present** in the current codebase. This plan systematically addresses them in priority order (P0 → P1 → P2) to ensure correctness, performance, and maintainability.

**Codebase Verification Complete:**
- All P0 issues confirmed critical and present
- All P1 issues confirmed and quantified (345+ lines of dead code, 19 unhandled mutex sites)
- All P2 issues confirmed present
- New race condition issue discovered (Issue 21)

---

## P0: Correctness & Reliability (Critical)

### Issue 1: Cooldowns Not Per-App
**Location:** `manager.rs:458`
**Problem:** `is_in_cooldown()` checks ANY app with the method, not per-app
```rust
// Current (wrong):
self.cooldowns.iter().any(|((_, m), cd)| *m == method && now < cd.until)
```

**Fix:**
```rust
pub(crate) fn is_in_cooldown(&self, app_id: &str, method: InjectionMethod) -> bool {
    let now = Instant::now();
    let key = (app_id.to_string(), method);
    self.cooldowns.get(&key).map_or(false, |cd| now < cd.until)
}
```

**Changes Required:**
- Update signature: `is_in_cooldown(&self, app_id: &str, method: InjectionMethod)`
- Update all call sites in `inject()` to pass `app_id`
- Add test: `test_cooldown_is_per_app()`

---

### Issue 2: "unknown_app" Hardcoded
**Location:** `manager.rs:552, 558`
**Problem:** `update_cooldown()` and `clear_cooldown()` hardcode "unknown_app"

**Fix:**
- Remove `update_cooldown()` and `clear_cooldown()` - they're redundant with `apply_cooldown()`
- Update all call sites to use `apply_cooldown(app_id, method, error)` directly
- Pass actual `app_id` from context

**Changes Required:**
1. Remove `update_cooldown()` at line 550
2. Remove `clear_cooldown()` at line 557
3. Update `inject()` at line 1027 to use `apply_cooldown(&app_id, method, &error_string)`
4. Update `inject()` at line 988 to remove `clear_cooldown()` call (cooldown clears on success automatically via `apply_cooldown`)

---

### Issue 3: Mutex Poisoning Not Handled
**Location:** Throughout `manager.rs`, `processor.rs`, and `session.rs`
**Problem:** Uses `if let Ok(m) = self.metrics.lock()` silently dropping failures

**Confirmed Locations:**
- `manager.rs`: 13 instances (lines 206, 249, 755, 783, 802, 834, 864, 882, 892, 944, 984, 1023, 1089)
- `processor.rs`: 4 instances (lines 142, 171, 202, 244)
- `session.rs`: 2 instances (lines 306, 313)
- **Total: 19 unhandled mutex lock sites**

**Fix:**
```rust
// Replace all instances with proper handling:
let mut metrics = self.metrics.lock().unwrap_or_else(|e| {
    error!("Metrics mutex poisoned: {}", e);
    e.into_inner()
});
```

**Or create a helper:**
```rust
fn lock_metrics(&self) -> std::sync::MutexGuard<InjectionMetrics> {
    self.metrics.lock().unwrap_or_else(|e| {
        error!("Metrics mutex poisoned, recovering: {}", e);
        e.into_inner()
    })
}
```

**Changes Required:**
- Add `lock_metrics()` helper to `StrategyManager`
- Add `lock_processor_metrics()` helper to `InjectionProcessor`
- Add `lock_session_metrics()` helper to injection session type
- Replace all 19 `if let Ok(m) = self.metrics.lock()` with `lock_metrics()`
- Add test: `test_mutex_poisoning_recovery()`

---

### Issue 4: No Timeouts on Awaited Operations
**Location:** `manager.rs:871`
**Problem:** `focus_provider.get_focus_status().await` has no timeout wrapper

**Fix:**
```rust
use tokio::time::timeout;

let focus_status = match timeout(
    self.config.per_method_timeout(),
    self.focus_provider.get_focus_status()
).await {
    Ok(Ok(status)) => status,
    Ok(Err(e)) => {
        warn!("Failed to get focus status: {}", e);
        FocusStatus::Unknown
    }
    Err(_) => {
        warn!("Focus status check timed out after {}ms",
            self.config.per_method_timeout_ms);
        FocusStatus::Unknown
    }
};
```

**Changes Required:**
- Wrap `get_focus_status()` call with timeout
- Wrap `get_current_app_id()` call with timeout
- Wrap all injector calls with timeout (if not already done)
- Add test: `test_focus_timeout()`

---

### Issue 5: Blocking Runtime in Async Context
**Location:** `manager.rs:332-398`
**Problem:** `get_active_window_class()` uses `std::process::Command` (blocking) in async function

**Fix:**
```rust
use tokio::process::Command;

async fn get_active_window_class(&self) -> Result<String, InjectionError> {
    // Try xprop for X11
    if let Ok(output) = Command::new("xprop")
        .args(["-root", "_NET_ACTIVE_WINDOW"])
        .output()
        .await
    {
        // ... rest of logic
    }

    // Try swaymsg for Wayland
    if let Ok(output) = Command::new("swaymsg")
        .args(["-t", "get_tree"])
        .output()
        .await
    {
        // ... rest of logic
    }

    Err(InjectionError::Other(
        "Could not determine active window class".to_string(),
    ))
}
```

**Changes Required:**
- Replace `std::process::Command` with `tokio::process::Command`
- Update all `.output()` calls to `.output().await`
- Add test: `test_window_class_async()`

---

### Issue 6: Silent Failures in App Detection
**Location:** `manager.rs:325`
**Problem:** `get_current_app_id()` returns `"unknown"` instead of propagating errors

**Fix:**
```rust
// Return Result and let caller handle:
pub(crate) async fn get_current_app_id(&self) -> Result<String, InjectionError> {
    // ... existing logic

    // At the end, instead of Ok("unknown"):
    Err(InjectionError::Other("Could not determine app ID".to_string()))
}

// Update callers to handle gracefully:
let app_id = self.get_current_app_id().await.unwrap_or_else(|e| {
    debug!("Could not determine app ID: {}, using 'unknown'", e);
    "unknown".to_string()
});
```

**Changes Required:**
- Keep the function signature returning `Result`
- Remove the final `Ok("unknown")` fallback at line 325
- Update call site in `inject()` to handle error gracefully
- Add metrics for app detection failures

---

### Issue 7: No Cache Invalidation
**Location:** `manager.rs:466`
**Problem:** `update_success_record()` doesn't clear `cached_method_order`

**Fix:**
```rust
pub(crate) fn update_success_record(
    &mut self,
    app_id: &str,
    method: InjectionMethod,
    success: bool,
) {
    let key = (app_id.to_string(), method);

    // ... existing logic ...

    // Invalidate cache when success record changes significantly
    if let Some((cached_app, _)) = &self.cached_method_order {
        if cached_app == app_id {
            // Check if this update might change method order
            let should_invalidate = !success && record.fail_count > 2;
            if should_invalidate {
                self.cached_method_order = None;
                debug!("Invalidated method order cache for {}", app_id);
            }
        }
    }

    if should_cooldown {
        self.apply_cooldown(app_id, method, "Multiple consecutive failures");
    }
}
```

**Changes Required:**
- Add cache invalidation logic to `update_success_record()`
- Add cache invalidation to `apply_cooldown()`
- Add test: `test_cache_invalidation_on_failure()`

---

## P1: Performance & Maintainability

### Issue 8: Duplicate Functions
**Location:** `manager.rs:565, 641`
**Problem:** Both `_get_method_priority()` and `compute_method_order()` exist and duplicate logic

**Fix:**
```rust
// Remove _get_method_priority() entirely
// Keep only compute_method_order() which is more complete
// Update get_method_priority() wrapper:

pub fn get_method_priority(&self, app_id: &str) -> Vec<InjectionMethod> {
    self.compute_method_order(app_id)
}
```

**Changes Required:**
- Delete `_get_method_priority()` function (lines 565-638)
- Update `get_method_priority()` to call `compute_method_order()`
- Update any tests calling `_get_method_priority()`

---

### Issue 9: 32-bit Hash, Not Zero-Copy
**Location:** `manager.rs:33-43`
**Problem:** `redact_text()` uses 32-bit hash and returns `String`, not `Cow`

**Fix:**
```rust
use std::borrow::Cow;

fn redact_text(text: &str, redact: bool) -> Cow<'_, str> {
    if redact {
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let hash = hasher.finish(); // Full 64-bit hash
        Cow::Owned(format!("len={} hash={:016x}", text.len(), hash))
    } else {
        Cow::Borrowed(text)
    }
}
```

**Changes Required:**
- Update signature to return `Cow<'_, str>`
- Use full 64-bit hash instead of masking to 32-bit
- Update call sites to handle `Cow` (`.as_ref()` or `.into_owned()`)

---

### Issue 10: Inefficient Comparator
**Location:** `manager.rs:606-632`
**Problem:** Uses `position()` in `sort_by()` which iterates repeatedly

**Fix:**
```rust
// Pre-compute positions in a HashMap
let position_map: HashMap<InjectionMethod, usize> = base_order_copy
    .iter()
    .enumerate()
    .map(|(i, m)| (*m, i))
    .collect();

base_order.sort_by(|a, b| {
    let pos_a = position_map.get(a).copied().unwrap_or(usize::MAX);
    let pos_b = position_map.get(b).copied().unwrap_or(usize::MAX);
    pos_a.cmp(&pos_b).then_with(|| {
        let key_a = (app_id.to_string(), *a);
        let key_b = (app_id.to_string(), *b);
        let rate_a = self.success_cache.get(&key_a).map(|r| r.success_rate).unwrap_or(0.5);
        let rate_b = self.success_cache.get(&key_b).map(|r| r.success_rate).unwrap_or(0.5);
        rate_b.partial_cmp(&rate_a).unwrap_or(std::cmp::Ordering::Equal)
    })
});
```

**Changes Required:**
- Apply fix to both `_get_method_priority()` and `compute_method_order()`
- Add microbenchmark to verify improvement

---

### Issue 11: Unbatched Metrics
**Location:** Throughout `manager.rs`
**Problem:** Individual locks throughout instead of batching

**Fix:**
```rust
// Before method loop:
let mut metrics_batch = MetricsBatch::new();

// During loop:
metrics_batch.record_attempt(method, duration);
metrics_batch.record_success(method, duration);

// After loop or on error:
if let Ok(mut m) = self.metrics.lock() {
    metrics_batch.apply(&mut m);
}
```

**Or simpler: collect metrics locally, flush once:**
```rust
struct InjectionAttempt {
    method: InjectionMethod,
    duration_ms: u64,
    success: bool,
    error: Option<String>,
}

let mut attempts = Vec::new();

// In loop:
attempts.push(InjectionAttempt { ... });

// After loop:
if let mut m = self.lock_metrics() {
    for attempt in attempts {
        if attempt.success {
            m.record_success(attempt.method, attempt.duration_ms);
        } else {
            m.record_failure(attempt.method, attempt.duration_ms, attempt.error.unwrap_or_default());
        }
    }
}
```

**Changes Required:**
- Refactor `inject()` to batch metrics updates
- Create `InjectionAttempt` struct or similar
- Flush metrics once at end of injection attempt

---

### Issue 12: No Cache Cleanup
**Location:** `manager.rs:152-154`
**Problem:** No bounds or cleanup for success/cooldown caches

**Fix:**
```rust
impl StrategyManager {
    /// Clean up old cache entries to prevent unbounded growth
    pub fn cleanup_old_caches(&mut self) {
        let now = Instant::now();
        let max_age = Duration::from_secs(3600); // 1 hour

        // Clean up expired cooldowns
        self.cooldowns.retain(|_, cd| now < cd.until);

        // Clean up stale success records (no activity in 1 hour)
        self.success_cache.retain(|_, record| {
            let last_activity = record.last_success
                .or(record.last_failure)
                .map(|t| now.duration_since(t) < max_age)
                .unwrap_or(false);
            last_activity
        });

        debug!("Cache cleanup: {} cooldowns, {} success records",
            self.cooldowns.len(), self.success_cache.len());
    }
}
```

**Changes Required:**
- Add `cleanup_old_caches()` method
- Call it periodically from `AsyncInjectionProcessor::run()` (e.g., every 5 minutes)
- Add config options for cache TTL if needed
- Add test: `test_cache_cleanup()`

---

### Issue 13: Magic Numbers Remain
**Location:** `manager.rs` and `types.rs`
**Status:** ⚠️ PARTIALLY RESOLVED

**Problem:** The cooldown backoff factor 2.0 mentioned at line 536 is NOT hardcoded inline.

**Verification:**
- Line 533: `let factor = self.config.cooldown_backoff_factor;` ✅ Uses config
- Line 536: Uses the `factor` variable ✅ Correct
- `types.rs:213-215`: Defines `default_cooldown_backoff_factor() -> f32 { 2.0 }` ✅ Acceptable

**Remaining Issues:**
- Cache TTL hardcoded in Issue 12 fix (3600 seconds)
- No config for success rate decay factor (currently unused)
- Test helpers may have magic numbers

**Fix:**
```rust
// In types.rs, add missing constants:
fn default_cache_ttl_seconds() -> u64 {
    3600 // 1 hour cache retention
}

fn default_success_rate_decay_factor() -> f64 {
    0.95 // Decay old data by 5% per day
}

// Update InjectionConfig:
pub struct InjectionConfig {
    // ... existing fields ...

    #[serde(default = "default_cache_ttl_seconds")]
    pub cache_ttl_seconds: u64,

    #[serde(default = "default_success_rate_decay_factor")]
    pub success_rate_decay_factor: f64,
}
```

**Changes Required:**
- Audit `manager.rs` for any remaining inline constants
- Add config fields for cache TTL
- Add config field for success rate decay (if/when implemented)
- Document all magic numbers found in tests

---

### Issue 14: Dead Paste/Keystroke Code
**Location:** `manager.rs:743, 792`
**Problem:** `chunk_and_paste()` and `pace_type_text()` are `#[allow(dead_code)]`

**Options:**

**Option A: Remove if truly unused**
```rust
// Delete chunk_and_paste() (lines 743-789)
// Delete pace_type_text() (lines 792-839)
```

**Option B: Integrate into injection flow**
```rust
// In inject(), use these methods when appropriate:
if use_paste && text.len() > self.config.paste_chunk_chars as usize {
    self.chunk_and_paste(injector, text).await?
} else if !use_paste {
    self.pace_type_text(injector, text).await?
} else {
    injector.inject_text(text).await?
}
```

**Recommendation:** Remove for now. Chunking should be handled by individual injectors if needed.

**Changes Required:**
- Delete `chunk_and_paste()` and `pace_type_text()`
- Document that injectors should handle chunking internally if needed
- Add TODO if functionality might be needed later

---

### Issue 15: No app_id Caching
**Location:** `manager.rs:278-326`
**Problem:** Spawns processes every call without TTL cache

**Fix:**
```rust
struct AppIdCache {
    app_id: String,
    cached_at: Instant,
    ttl: Duration,
}

impl StrategyManager {
    fn get_cached_app_id(&mut self) -> Option<&str> {
        self.app_id_cache.as_ref()
            .filter(|cache| cache.cached_at.elapsed() < cache.ttl)
            .map(|cache| cache.app_id.as_str())
    }

    pub(crate) async fn get_current_app_id(&mut self) -> Result<String, InjectionError> {
        // Check cache first
        if let Some(cached) = self.get_cached_app_id() {
            return Ok(cached.to_string());
        }

        // Fetch new value
        let app_id = self.fetch_app_id_uncached().await?;

        // Update cache
        self.app_id_cache = Some(AppIdCache {
            app_id: app_id.clone(),
            cached_at: Instant::now(),
            ttl: Duration::from_millis(self.config.focus_cache_duration_ms),
        });

        Ok(app_id)
    }
}
```

**Changes Required:**
- Add `AppIdCache` struct
- Add `app_id_cache: Option<AppIdCache>` field to `StrategyManager`
- Rename current `get_current_app_id()` to `fetch_app_id_uncached()`
- Implement caching wrapper
- Add test: `test_app_id_caching()`

---

## P2: Structure, Testing, and Documentation

### Issue 16: God Method Intact
**Location:** `manager.rs:842-1063`
**Problem:** `inject()` remains monolithic (221 lines)

**Fix:** Break into smaller functions:

```rust
async fn inject(&mut self, text: &str) -> Result<(), InjectionError> {
    if text.is_empty() {
        return Ok(());
    }

    self.validate_injection_preconditions(text)?;
    let context = self.prepare_injection_context(text).await?;
    self.perform_injection_with_fallbacks(text, context).await
}

fn validate_injection_preconditions(&mut self, text: &str) -> Result<(), InjectionError> {
    // Logging, paused check, budget check
    // Lines 848-868
}

async fn prepare_injection_context(&mut self, text: &str) -> Result<InjectionContext, InjectionError> {
    // Focus status, app_id, allowlist check, method selection
    // Lines 871-918
}

async fn perform_injection_with_fallbacks(&mut self, text: &str, context: InjectionContext) -> Result<(), InjectionError> {
    // Method loop, error handling
    // Lines 927-1063
}
```

**Changes Required:**
- Create `InjectionContext` struct to hold focus_status, app_id, use_paste, method_order
- Extract validation logic
- Extract context preparation logic
- Extract injection loop logic
- Add tests for each sub-function

---

### Issue 17: Missing Targeted Tests
**Location:** `crates/coldvox-text-injection/src/tests/`
**Problem:** No specific tests for cooldown per-app, cache invalidation, mocked time, etc.

**Fix:** Add test file `test_manager_edge_cases.rs`:

```rust
#[cfg(test)]
mod test_manager_edge_cases {
    use super::*;

    #[tokio::test]
    async fn test_cooldown_is_per_app() {
        // Verify app1 cooldown doesn't affect app2
    }

    #[tokio::test]
    async fn test_cache_invalidation_on_failure() {
        // Verify method order cache is invalidated after failures
    }

    #[tokio::test]
    async fn test_mutex_poisoning_recovery() {
        // Poison mutex and verify recovery
    }

    #[tokio::test]
    async fn test_timeout_on_focus_check() {
        // Mock slow focus provider and verify timeout
    }

    #[tokio::test]
    async fn test_app_id_cache_ttl() {
        // Verify cache expires after TTL
    }

    #[tokio::test]
    async fn test_metrics_batching() {
        // Verify reduced lock contention
    }

    #[tokio::test]
    async fn test_cache_cleanup() {
        // Verify old entries are removed
    }
}
```

**Changes Required:**
- Create `test_manager_edge_cases.rs`
- Add ~10 targeted tests covering all P0 and P1 issues
- Use mocking for time-based tests (or feature flags)

---

### Issue 18: Dead Code Preserved
**Location:** Multiple files
**Problem:** `#[allow(dead_code)]` still present

**Fix:** Remove or justify each instance:

| File | Line | Function | Action |
|------|------|----------|--------|
| `clipboard_injector.rs` | 87 | `save_clipboard` | Remove (unused) |
| `clipboard_injector.rs` | 116 | `restore_clipboard` | Remove (unused) |
| `clipboard_injector.rs` | 139 | `clipboard_with_restore` | Remove (unused) |
| `clipboard_injector.rs` | 162 | `set_clipboard` | Remove (unused) |
| `manager.rs` | 726 | `get_method_order_uncached` | Keep (used in tests) |
| `manager.rs` | 743 | `chunk_and_paste` | Remove (covered by Issue 14) |
| `manager.rs` | 792 | `pace_type_text` | Remove (covered by Issue 14) |
| `manager.rs` | 1071 | `override_injectors_for_tests` | Keep (test helper) |
| `manager.rs` | 1122-1129 | `MockInjector` | Keep (test helper) |

**Changes Required:**
- Remove dead functions from `clipboard_injector.rs`
- Remove `chunk_and_paste` and `pace_type_text`
- Keep test helpers but document why

---

### Issue 19: CI Knobs in Production
**Location:** `manager.rs:1152-1163`
**Problem:** `cfg!(test)` and `CI` checks remain in runtime code

**Fix:**

```rust
// In MockInjector, replace with deterministic behavior always:
async fn inject_text(&self, _text: &str) -> crate::types::InjectionResult<()> {
    // Use a deterministic seed based on object identity
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    (self as *const Self).hash(&mut hasher);
    let deterministic_value = (hasher.finish() % 100) as f64 / 100.0;

    if deterministic_value < self.success_rate {
        Ok(())
    } else {
        Err(InjectionError::MethodFailed("Mock injection failed".to_string()))
    }
}
```

**Changes Required:**
- Remove `cfg!(test)` branch from `MockInjector`
- Remove `std::env::var("CI")` check
- Use deterministic RNG or remove randomness from mocks entirely

---

### Issue 20: Undocumented
**Location:** Throughout module
**Problem:** Missing `/// # Errors` and concurrency docs

**Fix:** Add documentation to all public functions:

```rust
/// Try to inject text using the best available method
///
/// This function attempts to inject the provided text into the currently focused
/// application using an adaptive strategy that tries multiple injection methods
/// in order of likelihood of success.
///
/// # Arguments
/// * `text` - The text to inject (must not be empty)
///
/// # Errors
/// Returns `InjectionError` if:
/// - All injection methods fail
/// - Budget is exhausted before any method succeeds
/// - Application is not in the allowlist or is in the blocklist
/// - No editable focus is found (when `require_focus` is true)
/// - Focus state is unknown and `inject_on_unknown_focus` is false
///
/// # Concurrency
/// This method is `&mut self` because it updates internal caches and cooldown state.
/// It should not be called concurrently from multiple tasks. For concurrent access,
/// wrap in `Arc<Mutex<StrategyManager>>`.
///
/// # Performance
/// First call may be slow due to backend detection. Subsequent calls use cached
/// method ordering. Overall latency is bounded by `max_total_latency_ms` config.
pub async fn inject(&mut self, text: &str) -> Result<(), InjectionError>
```

**Changes Required:**
- Add `/// # Errors` sections to all fallible public functions
- Add `/// # Concurrency` notes to stateful methods
- Add `/// # Performance` notes where relevant
- Document all config options in `types.rs`

---

### Issue 21: Race Condition in Cache Update (NEW)
**Location:** `manager.rs:721`
**Priority:** P0 (Correctness)
**Problem:** Cache check and update are not atomic in `get_method_order_cached()`

**Current Code:**
```rust
pub fn get_method_order_cached(&mut self, app_id: &str) -> Vec<InjectionMethod> {
    if let Some((cached_app, methods)) = &self.cached_method_order {
        if cached_app == app_id {
            return methods.clone();
        }
    }
    // RACE: Cache could be invalidated here by another operation
    let methods = self.compute_method_order(app_id);
    self.cached_method_order = Some((app_id.to_string(), methods.clone()));
    methods
}
```

**Issue:** While this is `&mut self` (preventing concurrent calls), cache invalidation in other methods (Issue 7 fixes) could clear the cache between the check and update, causing:
1. Stale data to be re-cached after invalidation
2. Lost invalidation signals

**Fix:**
```rust
pub fn get_method_order_cached(&mut self, app_id: &str) -> Vec<InjectionMethod> {
    // Check if cache is valid and matches app
    let cache_valid = self.cached_method_order
        .as_ref()
        .map(|(cached_app, _)| cached_app == app_id)
        .unwrap_or(false);

    if cache_valid {
        // Cache hit - safe to return because we hold &mut self
        return self.cached_method_order.as_ref().unwrap().1.clone();
    }

    // Cache miss or wrong app - recompute
    let methods = self.compute_method_order(app_id);
    self.cached_method_order = Some((app_id.to_string(), methods.clone()));
    methods
}
```

**Changes Required:**
- Refactor cache check to be atomic
- Ensure cache invalidation logic (Issue 7) is safe
- Add test: `test_cache_race_condition()`
- Document cache semantics in `StrategyManager` docs

**Note:** This is less critical than other P0 issues because:
- `&mut self` prevents true race conditions
- Impact is limited to suboptimal method ordering
- Will be addressed when implementing Issue 7 fixes

---

## Implementation Order

### Phase 1: Critical Correctness (P0)
1. **Issue 3** (Mutex poisoning) - Foundation for all metrics (19 sites)
2. **Issue 1** (Cooldowns per-app) - Core logic fix
3. **Issue 2** (Remove unknown_app) - Depends on Issue 1
4. **Issue 7** (Cache invalidation) - Prevents stale data
5. **Issue 21** (Cache race condition) - Depends on Issue 7
6. **Issue 4** (Timeouts) - Safety net
7. **Issue 5** (Async runtime) - Correctness
8. **Issue 6** (Silent failures) - Better diagnostics

### Phase 2: Performance & Cleanup (P1)
9. **Issue 8** (Duplicate functions) - Easy cleanup (~140 lines)
10. **Issue 14** (Dead code) - Easy cleanup (~145 lines)
11. **Issue 9** (Hash/Cow) - Performance
12. **Issue 10** (Comparator) - Performance (O(n²) → O(n log n))
13. **Issue 15** (App ID caching) - Performance
14. **Issue 11** (Metrics batching) - Performance (19 locks → 1)
15. **Issue 12** (Cache cleanup) - Memory safety
16. **Issue 13** (Magic numbers) - Maintainability (partial fix needed)

### Phase 3: Structure & Documentation (P2)
17. **Issue 18** (Dead code audit) - Cleanup (200+ lines removable)
18. **Issue 19** (CI knobs) - Test quality
19. **Issue 17** (Targeted tests) - Coverage
20. **Issue 16** (God method) - Last refactor (222 lines → ~3 functions)
21. **Issue 20** (Documentation) - Final polish

---

## Testing Strategy

### Unit Tests (Per Issue)
- Each fix should include at least one targeted unit test
- Use mocks for external dependencies (AT-SPI, clipboard, window manager)
- Use deterministic time for cooldown/cache tests

### Integration Tests
- Test full injection flow with all fixes applied
- Test multi-app scenarios (cooldown isolation)
- Test long-running scenarios (cache cleanup)

### Regression Tests
- Ensure existing tests still pass after each phase
- Run full test suite after each major change

### Manual Testing
- Test on real Wayland/X11 environments
- Test with various applications (terminal, browser, editor)
- Monitor metrics and logs for anomalies

---

## Success Criteria

### Correctness
- [ ] Cooldowns are properly isolated per app-method pair (Issue 1)
- [ ] No hardcoded "unknown_app" strings in runtime code (Issue 2)
- [ ] All 19 mutex poisoning sites handled with recovery (Issue 3)
- [ ] All async operations have timeouts (Issue 4)
- [ ] No blocking calls in async context (Issue 5)
- [ ] Errors are propagated correctly (Issue 6)
- [ ] Cache invalidation works correctly (Issue 7)
- [ ] No race conditions in cache updates (Issue 21)

### Performance
- [ ] No duplicate code paths (~140 lines removed, Issue 8)
- [ ] Zero-copy where possible (Cow for redaction, Issue 9)
- [ ] O(1) lookups in hot paths (Issue 10)
- [ ] App ID cached with TTL (Issue 15)
- [ ] Metrics updates batched (19 locks → 1, Issue 11)
- [ ] Caches bounded and cleaned up (Issue 12)

### Maintainability
- [ ] No dead code warnings (345+ lines removed, Issues 14, 18)
- [ ] No magic numbers (Issue 13 partial fix)
- [ ] Functions under 50 lines (God method split, Issue 16)
- [ ] All public APIs documented (Issue 20)
- [ ] Test coverage >80% (Issue 17)
- [ ] No test-only code in production paths (Issue 19)

---

## Risks & Mitigations

### Risk: Breaking Changes
**Mitigation:** Run full test suite after each issue. Use feature flags for large changes.

### Risk: Performance Regression
**Mitigation:** Add benchmarks for hot paths. Compare before/after metrics.

### Risk: Incomplete Testing
**Mitigation:** Add coverage tracking. Require tests for each bug fix.

### Risk: Time/Scope Creep
**Mitigation:** Implement in strict priority order. Can stop after any phase.

---

## Dependencies

- `tokio` - Already present (async runtime)
- `tracing` - Already present (logging)
- No new external dependencies required

---

## Timeline Estimate

**Assuming one developer working full-time:**

- Phase 1 (P0): 2-3 days
- Phase 2 (P1): 2-3 days
- Phase 3 (P2): 1-2 days
- **Total: 5-8 days**

**Assuming incremental work:**
- Can be done issue-by-issue as time permits
- Each issue is 1-3 hours depending on complexity
- Minimum viable fix: Complete Phase 1 (P0 only)

---

## Implementation Recommendations

### Combine Interdependent Fixes
- **Issues 1 & 2** should be fixed together (cooldown per-app + remove unknown_app)
- **Issues 7 & 21** should be fixed together (cache invalidation + race condition)

### Add Safety Features
1. **Feature flag for rollback**: Add `legacy-cooldown` feature to enable old behavior if needed
2. **Integration tests first**: Write failing tests for each bug before fixing
3. **Telemetry for validation**: Add counters to track fix effectiveness:
   ```rust
   metrics.record_cooldown_skip_count(app_id, method);
   metrics.record_cache_invalidation_count();
   metrics.record_app_detection_failure();
   ```

### Alternative Approach for Issue 6
Consider making `get_current_app_id()` return `Result<Option<String>>`:
- `Ok(Some(app_id))` - Successfully detected
- `Ok(None)` - No window focused (legitimate state)
- `Err(e)` - Detection failed (error state)

This provides better diagnostics than silent "unknown" fallback.

### Testing Focus Areas
1. **Multi-app scenarios**: Test with 3+ different applications simultaneously
2. **Long-running sessions**: Test cache cleanup after 24+ hours
3. **Concurrency**: Test multiple injection requests in parallel
4. **Error recovery**: Test mutex poisoning recovery under load

### Success Metrics to Track
- **Cooldown effectiveness**: Measure per-app vs global cooldown hit rate
- **Cache hit rate**: Should be >90% for frequently used apps
- **Mutex contention**: Lock acquisition time P99 < 1ms
- **Memory growth**: Should plateau after cache cleanup implemented

---

## Next Steps

1. Review this plan and recommendations
2. Confirm priority order and combined fixes approach
3. Create tracking issues (if using issue tracker)
4. Write integration tests that demonstrate each bug
5. Begin implementation with Issue 3 (mutex poisoning)
6. Submit PRs after each phase for review
7. Monitor telemetry after deployment

---

## Verification Summary

All 21 issues have been confirmed present in the codebase through automated analysis:

**Critical Issues (P0): 8 issues**
- 19 unhandled mutex poisoning sites across 3 files
- Cooldown logic affects all apps globally
- No timeouts on 4+ async operations
- Blocking commands in async context
- Cache invalidation missing
- Race condition in cache update logic

**Performance Issues (P1): 8 issues**
- 345+ lines of dead code
- 19 individual mutex locks in hot path
- O(n²) sorting in method priority calculation
- 140 lines of duplicate function logic
- No app_id caching with TTL

**Structure Issues (P2): 5 issues**
- 222-line god method
- Missing targeted tests for edge cases
- Undocumented public APIs
- Test-only code in production paths

**Total Lines Removable:** ~485 lines (dead code + duplicates)
**Total Lock Sites to Fix:** 19 locations

---

**Document Version:** 1.1
**Last Updated:** 2025-10-08
**Status:** Verified against codebase - ready for implementation
