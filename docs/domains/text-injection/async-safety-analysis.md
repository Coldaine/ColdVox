---
doc_type: reference
subsystem: text-injection
version: 1.0.0
status: draft
owners: Documentation Working Group
last_reviewed: 2025-10-19
---

# Async Safety Analysis: Text Injection System

## Executive Summary

This document traces all execution paths through the text injection system, analyzing async correctness, lock ordering, and potential race conditions. The system has **two main entry points** and several critical async hazards that need attention.

---

## Entry Points

### 1. **StrategyOrchestrator::inject_text()** (`orchestrator.rs:337`)
```rust
pub async fn inject_text(&self, text: &str) -> InjectionResult<()>
```

### 2. **StrategyManager::inject()** (`manager.rs:867`)
```rust
pub async fn inject(&mut self, text: &str) -> Result<(), InjectionError>
```

---

## Critical Async Hazards Found

### 🔴 **HAZARD 1: Concurrent Write Access in Orchestrator**

**Location:** `orchestrator.rs:206-217`

```rust
async fn check_and_trigger_prewarm(&self) {
    let session = self.session.read().await;  // ← Acquires read lock
    if session.state() == SessionState::Buffering {
        let context = self.last_context.read().await;  // ← Acquires second read lock
        if let Some(ref ctx) = *context {
            let ctx_clone = ctx.clone();
            tokio::spawn(async move {  // ← Spawns concurrent task
                if let Err(e) = run(&ctx_clone).await {
                    warn!("Pre-warming failed: {}", e);
                }
            });
        }
    }
}
```

**Problem:**
1. Holds `session` read lock while acquiring `last_context` read lock
2. Spawns task that might write to shared state
3. No clear cancellation mechanism if parent task fails

**Impact:** Low (read-only access), but spawned task could race with updates

---

### 🔴 **HAZARD 2: State Mutation After Await in inject_text**

**Location:** `orchestrator.rs:343-350`

```rust
pub async fn inject_text(&self, text: &str) -> InjectionResult<()> {
    if text.is_empty() {
        return Ok(());
    }

    // Update context from pre-warmed data
    let context = self.prewarm_controller.get_atspi_context().await;  // ← AWAIT
    *self.last_context.write().await = Some(context.clone());         // ← WRITE

    // Check if we should trigger pre-warming
    self.check_and_trigger_prewarm().await;  // ← Another AWAIT

    // Execute fast-fail injection loop
    self.fast_fail_inject(text).await
}
```

**Problem:**
1. Between getting context and writing it, another task could modify `last_context`
2. `check_and_trigger_prewarm()` spawns a background task that might read stale context
3. Race condition: spawned task might use outdated context if injection fails quickly

**Impact:** Medium - Could lead to stale context being used for pre-warming

---

### 🔴 **HAZARD 3: Lock Ordering Issue in PrewarmController**

**Location:** `prewarm.rs:430-503`

```rust
pub async fn execute_all_prewarming(&self) {
    // ... futures::join! of all pre-warming tasks ...
    
    // Sequential write lock acquisitions - different order each time
    {
        let mut cached = self.atspi_data.write().await;
        // ... update ...
    }
    // Lock released
    
    {
        let mut cached = self.clipboard_data.write().await;
        // ... update ...
    }
    // Lock released
    
    {
        let mut cached = self.portal_data.write().await;
        // ... update ...
    }
    // Lock released
    
    {
        let mut cached = self.virtual_keyboard_data.write().await;
        // ... update ...
    }
}
```

**Problem:**
1. Four separate write locks acquired sequentially
2. If another caller reads in different order → potential for inconsistent state
3. At line `516-519`, four read locks acquired simultaneously:

```rust
async fn is_any_data_expired(&self) -> bool {
    let atsi = self.atspi_data.read().await;
    let clipboard = self.clipboard_data.read().await;
    let portal = self.portal_data.read().await;
    let vk = self.virtual_keyboard_data.read().await;
    // ... uses all four ...
}
```

**Impact:** Medium - Could observe partially updated state across multiple caches

---

### 🔴 **HAZARD 4: Manager State Access Pattern**

**Location:** `manager.rs:867-1090`

```rust
pub async fn inject(&mut self, text: &str) -> Result<(), InjectionError> {
    // ...
    
    // Get current focus status
    let focus_status = match self.focus_provider.get_focus_status().await {
        Ok(status) => status,
        Err(e) => {
            warn!("Failed to get focus status: {}", e);
            FocusStatus::Unknown
        }
    };
    
    // ... many checks and modifications to self ...
    
    // Get current application ID
    let app_id = self.get_current_app_id().await?;  // ← Another async call
    
    // ... more self modifications ...
    
    self.check_and_trigger_prewarm().await;  // ← Spawns background task
    
    // ... method iteration ...
    
    for method in method_order.clone() {
        // ...
        
        let result = {
            if let Some(injector) = self.injectors.get_mut(method) {  // ← Mutable borrow
                if use_paste {
                    injector.inject_text(text).await  // ← AWAIT while holding mut ref
                } else {
                    injector.inject_text(text).await
                }
            } else {
                continue;
            }
        };
        
        match result {
            Ok(()) => {
                // ... updates self.metrics, self.success_cache, self.cooldowns ...
            }
            Err(e) => {
                // ... updates self.metrics, self.success_cache, self.cooldowns ...
            }
        }
    }
}
```

**Problems:**
1. **Takes `&mut self`** - exclusive access for entire duration
2. Multiple `.await` points while holding `&mut self`
3. Spawns background task that might access same resources
4. Updates shared state (`metrics`, `success_cache`, `cooldowns`) after awaits
5. Between getting `app_id` and using it, app could have changed

**Impact:** High - Could deadlock if called concurrently, state mutations not atomic

---

### 🟡 **HAZARD 5: Metrics Lock Contention**

**Location:** Throughout `manager.rs`

```rust
if let Ok(mut m) = self.metrics.lock() {
    m.record_success(method, duration);
}
```

**Pattern found at:**
- Line 780, 808, 827, 859, 889, 907, 917, 972, 1012, 1051

**Problems:**
1. `Arc<Mutex<InjectionMetrics>>` used throughout
2. Multiple lock acquisitions during inject path
3. If lock is poisoned, silently fails with `if let Ok()`
4. No logging when lock fails
5. Could lose metrics data without notification

**Impact:** Low-Medium - Metrics might be incomplete, but won't crash

---

### 🟡 **HAZARD 6: Session Read Lock Held Across Spawn**

**Location:** `manager.rs:573-586`

```rust
async fn check_and_trigger_prewarm(&self) {
    if let Some(ref session) = self.session {
        let session_guard = session.read().await;  // ← Acquire read lock
        if session_guard.state() == SessionState::Buffering {
            let context = self.prewarm_controller.get_atspi_context().await;
            let ctx_clone = context.clone();
            tokio::spawn(async move {  // ← Spawn while holding lock
                if let Err(e) = crate::prewarm::run(&ctx_clone).await {
                    warn!("Pre-warming failed: {}", e);
                }
            });
        }
    }  // ← Lock released here
}
```

**Problem:**
1. Read lock held while spawning task
2. Spawned task has no synchronization with parent
3. If spawned task tries to write session → could block indefinitely

**Impact:** Low (read-only), but could cause unexpected blocking

---

## Execution Path Analysis

### Path 1: Orchestrator Flow

```
inject_text(text)
  └─> if empty → return Ok
  └─> prewarm_controller.get_atspi_context().await       [AWAIT 1]
      └─> atspi_data.read().await                        [READ LOCK]
  └─> last_context.write().await = context               [WRITE LOCK]
  └─> check_and_trigger_prewarm().await                  [AWAIT 2]
      └─> session.read().await                           [READ LOCK]
      └─> last_context.read().await                      [READ LOCK]
      └─> tokio::spawn(run(&ctx))                        [SPAWNED TASK]
  └─> fast_fail_inject(text).await                       [AWAIT 3]
      └─> For each strategy:
          └─> timeout(injector.inject_text(text))        [AWAIT 4]
          └─> timeout(text_changed(app, window))         [AWAIT 5]
```

**Await Points:** 5 minimum
**Lock Acquisitions:** 3 (2 read, 1 write)
**Background Tasks:** 1 spawned

### Path 2: Manager Flow

```
inject(text)                                             [Takes &mut self]
  └─> if empty → return Ok
  └─> focus_provider.get_focus_status().await            [AWAIT 1]
  └─> get_current_app_id().await                         [AWAIT 2]
      └─> AccessibilityConnection::new().await           [AWAIT nested]
      └─> collection.get_matches(...).await              [AWAIT nested]
  └─> check_and_trigger_prewarm().await                  [AWAIT 3]
      └─> session.read().await                           [READ LOCK]
      └─> prewarm_controller.get_atspi_context().await   [AWAIT nested]
          └─> atspi_data.read().await                    [READ LOCK]
      └─> tokio::spawn(run(&ctx))                        [SPAWNED TASK]
  └─> For each method in method_order:
      └─> injectors.get_mut(method)                      [MUT BORROW]
      └─> injector.inject_text(text).await               [AWAIT 4+]
      └─> metrics.lock() + update                        [MUTEX LOCK]
      └─> update success_cache, cooldowns                [SELF MUTATION]
```

**Await Points:** 4+ (more in loop)
**Lock Acquisitions:** 2+ read locks, 1+ mutex locks per attempt
**State Mutations:** success_cache, cooldowns, metrics (multiple times)
**Background Tasks:** 1 spawned

---

## Specific Race Conditions

### Race 1: Context Staleness

**Scenario:**
1. Thread A: `inject_text()` calls `get_atspi_context()` → gets Context v1
2. Thread B: Focus changes, another inject updates `last_context` → Context v2
3. Thread A: Writes Context v1 to `last_context` (overwrites v2)
4. Thread A: Spawns prewarm task with stale Context v1
5. Thread B: Injects with correct Context v2

**Result:** Pre-warming uses wrong context

### Race 2: Partial Pre-warming State

**Scenario:**
1. Task A: `execute_all_prewarming()` updates atspi_data
2. Task B: `is_any_data_expired()` reads atspi_data (valid)
3. Task A: Updates clipboard_data
4. Task B: Reads clipboard_data (valid)
5. Task A: Updates portal_data
6. Task B: Reads portal_data (valid)
7. Task A: About to update vk_data
8. Task B: Reads vk_data (EXPIRED!)

**Result:** Inconsistent view - thinks data is expired when 3/4 are fresh

### Race 3: Manager Reentrancy

**Scenario:**
1. Task A: Calls `manager.inject("text1")` → holds `&mut self`
2. Task A: Awaits on `focus_provider.get_focus_status()`
3. Task B: Tries to call `manager.inject("text2")`
4. Task B: **BLOCKS** waiting for `&mut self`

**Result:** Sequential execution forced, possible deadlock if A spawns B

---

## Lock Hierarchy

```
Level 1: session (Arc<RwLock<InjectionSession>>)
Level 2: last_context (Arc<RwLock<Option<AtspiContext>>>)
Level 3: prewarm caches (Arc<RwLock<CachedData<T>>>)
Level 4: metrics (Arc<Mutex<InjectionMetrics>>)
```

**Violations:**
- `check_and_trigger_prewarm`: Acquires Level 1, then Level 2 (OK)
- `inject_text`: Acquires Level 3, then Level 2 (INCONSISTENT ORDER)
- `execute_all_prewarming`: Acquires Level 3 locks in arbitrary order

---

## Recommendations

### 🔧 Fix 1: Atomic Context Update

**Current:**
```rust
let context = self.prewarm_controller.get_atspi_context().await;
*self.last_context.write().await = Some(context.clone());
```

**Fixed:**
```rust
// Ensure context is fresh and atomically updated
let context = {
    let mut guard = self.last_context.write().await;
    let fresh_context = self.prewarm_controller.get_atspi_context().await;
    *guard = Some(fresh_context.clone());
    fresh_context
};
```

### 🔧 Fix 2: Consistent Lock Ordering

**Current:**
```rust
// Four separate write acquisitions
let mut cached = self.atspi_data.write().await;
let mut cached = self.clipboard_data.write().await;
// etc.
```

**Fixed:**
```rust
// Acquire all locks at once in consistent order
let (mut atspi, mut clipboard, mut portal, mut vk) = tokio::join!(
    self.atspi_data.write(),
    self.clipboard_data.write(),
    self.portal_data.write(),
    self.virtual_keyboard_data.write(),
);

// Update all atomically
if let Ok(data) = atspy_result {
    atspi.update(data);
}
// etc.
```

### 🔧 Fix 3: Make Manager Take &self Instead of &mut self

**Problem:** `inject(&mut self)` prevents concurrent calls

**Solution:**
```rust
// Change internal state to use interior mutability
pub struct StrategyManager {
    success_cache: Arc<Mutex<HashMap<AppMethodKey, SuccessRecord>>>,
    cooldowns: Arc<Mutex<HashMap<InjectionMethod, CooldownState>>>,
    cached_method_order: Arc<RwLock<Option<Vec<InjectionMethod>>>>,
    // ...
}

pub async fn inject(&self, text: &str) -> Result<(), InjectionError> {
    // Now can be called concurrently
}
```

### 🔧 Fix 4: Handle Poisoned Locks

**Current:**
```rust
if let Ok(mut m) = self.metrics.lock() {
    m.record_success(method, duration);
}
```

**Fixed:**
```rust
match self.metrics.lock() {
    Ok(mut m) => m.record_success(method, duration),
    Err(e) => {
        error!("Metrics lock poisoned: {}", e);
        // Attempt recovery or clear poison
        let mut m = e.into_inner();
        m.record_success(method, duration);
    }
}
```

### 🔧 Fix 5: Add Cancellation Token for Spawned Tasks

**Current:**
```rust
tokio::spawn(async move {
    if let Err(e) = run(&ctx_clone).await {
        warn!("Pre-warming failed: {}", e);
    }
});
```

**Fixed:**
```rust
let cancel_token = self.prewarm_cancel_token.clone();
tokio::spawn(async move {
    tokio::select! {
        result = run(&ctx_clone) => {
            if let Err(e) = result {
                warn!("Pre-warming failed: {}", e);
            }
        }
        _ = cancel_token.cancelled() => {
            debug!("Pre-warming cancelled");
        }
    }
});
```

### 🔧 Fix 6: Add Timeout Guards

**Add to all injector calls:**
```rust
// Don't let individual injector calls block indefinitely
let timeout_duration = Duration::from_millis(self.config.per_method_timeout_ms);
let result = tokio::time::timeout(
    timeout_duration,
    injector.inject_text(text)
).await;

match result {
    Ok(Ok(())) => { /* success */ }
    Ok(Err(e)) => { /* injection error */ }
    Err(_) => { /* timeout */ }
}
```

---

## Testing Recommendations

### Test 1: Concurrent Injection Calls
```rust
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_injections() {
    let orchestrator = Arc::new(StrategyOrchestrator::new(config).await);
    
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let orch = orchestrator.clone();
            tokio::spawn(async move {
                orch.inject_text(&format!("text {}", i)).await
            })
        })
        .collect();
    
    for handle in handles {
        assert!(handle.await.is_ok());
    }
}
```

### Test 2: Lock Ordering Validation
```rust
#[tokio::test]
async fn test_no_deadlock_on_concurrent_prewarm() {
    let controller = Arc::new(PrewarmController::new(config));
    
    // Spawn multiple pre-warming tasks
    let handles: Vec<_> = (0..5)
        .map(|_| {
            let ctrl = controller.clone();
            tokio::spawn(async move {
                ctrl.execute_all_prewarming().await;
            })
        })
        .collect();
    
    // Should complete without deadlock
    tokio::time::timeout(
        Duration::from_secs(5),
        futures::future::join_all(handles)
    ).await.expect("Deadlock detected");
}
```

### Test 3: Context Consistency
```rust
#[tokio::test]
async fn test_context_consistency() {
    let orchestrator = StrategyOrchestrator::new(config).await;
    
    // Inject text
    orchestrator.inject_text("test").await.unwrap();
    
    // Context should be consistent across all readers
    let ctx1 = orchestrator.last_context.read().await.clone();
    let ctx2 = orchestrator.prewarm_controller.get_atspi_context().await;
    
    // They should match
    assert_eq!(ctx1.as_ref().unwrap().target_app, ctx2.target_app);
}
```

---

## Summary

**Total Hazards:** 6 (4 critical, 2 moderate)

**Critical Issues:**
1. ❌ State mutation after await (context staleness)
2. ❌ Lock ordering inconsistency (partial state reads)
3. ❌ Manager `&mut self` prevents concurrent calls
4. ❌ Metrics lock failures silently ignored

**Medium Issues:**
1. ⚠️ Background tasks spawned without cancellation
2. ⚠️ Read locks held across spawns

**Priority Fixes:**
1. Change Manager to use interior mutability (`&self` instead of `&mut self`)
2. Implement consistent lock ordering in PrewarmController
3. Add cancellation tokens for spawned tasks
4. Handle poisoned locks explicitly
5. Add comprehensive concurrent testing

**Estimated Effort:**
- Critical fixes: 2-3 days
- Testing: 1-2 days
- Validation: 1 day

---

## Appendix: Complete Call Graph

```
inject_text (orchestrator)
├─> get_atspi_context
│   └─> atspi_data.read() [RwLock]
├─> last_context.write() [RwLock]
├─> check_and_trigger_prewarm
│   ├─> session.read() [RwLock]
│   ├─> last_context.read() [RwLock]
│   └─> spawn(run)
│       └─> execute_all_prewarming
│           ├─> atspi_data.write() [RwLock]
│           ├─> clipboard_data.write() [RwLock]
│           ├─> portal_data.write() [RwLock]
│           └─> virtual_keyboard_data.write() [RwLock]
└─> fast_fail_inject
    └─> For each strategy:
        └─> timeout(injector.inject_text)

inject (manager) [&mut self]
├─> focus_provider.get_focus_status()
├─> get_current_app_id
│   └─> AT-SPI calls (multiple awaits)
├─> check_and_trigger_prewarm
│   ├─> session.read() [RwLock]
│   ├─> get_atspi_context
│   │   └─> atspi_data.read() [RwLock]
│   └─> spawn(run)
└─> For each method:
    ├─> injectors.get_mut [&mut borrow]
    ├─> injector.inject_text (await)
    ├─> metrics.lock() [Mutex]
    ├─> update success_cache [&mut self]
    └─> update cooldowns [&mut self]
```
