---
title: "[P2] Undocumented: Missing /// # Errors and concurrency docs"
labels: ["documentation", "priority:P2", "component:text-injection"]
---

## Problem

Public and critical functions lack proper documentation, especially:
1. Missing `/// # Errors` sections for fallible functions
2. No concurrency/thread-safety documentation for `Arc<Mutex<_>>` fields
3. Unclear async function behavior and cancellation safety
4. Missing panic documentation where applicable

## Current State

Many functions lack documentation:

```rust
// Missing error documentation
pub async fn inject(&mut self, text: &str) -> Result<(), InjectionError> {
    // What errors can this return?
    // When does it return each error type?
}

// Missing concurrency docs
pub struct StrategyManager {
    metrics: Arc<Mutex<InjectionMetrics>>,  // Thread-safe? Lock order?
}

// Missing panic docs
pub(crate) fn get_method_priority(&self, app_id: &str) -> Vec<InjectionMethod> {
    // Can this panic? Under what conditions?
}
```

## Expected Documentation Standards

### 1. Error Documentation
```rust
/// Try to inject text using the best available method
///
/// # Errors
///
/// Returns an error in the following cases:
///
/// - `InjectionError::Paused`: Injection is currently paused
/// - `InjectionError::NotAllowed`: Current app is not in allowlist
/// - `InjectionError::FocusRequired`: Focus required but not on editable element
/// - `InjectionError::AllMethodsFailed`: All injection methods were tried and failed
/// - `InjectionError::Other`: Unexpected error during injection
///
/// # Examples
///
/// ```no_run
/// # use coldvox_text_injection::*;
/// # async fn example(manager: &mut StrategyManager) -> Result<(), InjectionError> {
/// manager.inject("Hello, world!").await?;
/// # Ok(())
/// # }
/// ```
pub async fn inject(&mut self, text: &str) -> Result<(), InjectionError> {
    // ...
}
```

### 2. Concurrency Documentation
```rust
/// Manages text injection strategies with adaptive method selection
///
/// # Thread Safety
///
/// `StrategyManager` is `!Send` but can be used from a single async task.
/// The internal `metrics` field uses `Arc<Mutex<_>>` for shared access.
///
/// ## Lock Order
///
/// When acquiring multiple locks, always acquire in this order to prevent deadlocks:
/// 1. `metrics` mutex
/// 2. Never hold `metrics` lock across await points
///
/// # Concurrent Access
///
/// The `metrics` field can be shared across threads via `Arc`, but the
/// `StrategyManager` itself should not be shared. Create separate instances
/// per thread/task if parallel injection is needed.
pub struct StrategyManager {
    /// Thread-safe metrics shared with other components
    metrics: Arc<Mutex<InjectionMetrics>>,
    // ...
}
```

### 3. Async Documentation
```rust
/// Get current application identifier
///
/// # Cancellation Safety
///
/// This function is cancellation-safe. If the future is dropped before
/// completion, no side effects occur and the next call will retry detection.
///
/// # Timeouts
///
/// This function should complete within 1 second under normal conditions.
/// Consider using `tokio::time::timeout` if guaranteed bounds are needed.
///
/// # Errors
///
/// Returns `InjectionError::Other` if:
/// - AT-SPI connection fails
/// - X11 window detection fails
/// - No detection method is available for the current platform
pub(crate) async fn get_current_app_id(&self) -> Result<String, InjectionError> {
    // ...
}
```

### 4. Panic Documentation
```rust
/// Compute method ordering based on success rates
///
/// # Panics
///
/// This function should never panic. If app_id is not found in caches,
/// it returns default ordering. If success rate calculation fails,
/// it defaults to base priority order.
///
/// If this function panics, it indicates a bug in the success rate
/// calculation logic. Please file an issue with the panic message.
pub(crate) fn compute_method_order(&self, app_id: &str) -> Vec<InjectionMethod> {
    // ...
}
```

### 5. Configuration Documentation
```rust
/// Configuration for text injection behavior
///
/// # Examples
///
/// ```
/// # use coldvox_text_injection::InjectionConfig;
/// let config = InjectionConfig {
///     require_focus: true,
///     inject_on_unknown_focus: false,
///     cooldown_initial_ms: 1000,
///     cooldown_backoff_factor: 2.0,
///     cooldown_max_ms: 60000,
///     ..Default::default()
/// };
/// ```
///
/// # Field Descriptions
///
/// - `require_focus`: If true, only inject when an editable element has focus
/// - `inject_on_unknown_focus`: If true, attempt injection even when focus state is unknown
/// - `cooldown_initial_ms`: Initial cooldown duration when a method fails
/// - `cooldown_backoff_factor`: Multiplier for exponential backoff (typically 2.0)
/// - `cooldown_max_ms`: Maximum cooldown duration
#[derive(Debug, Clone)]
pub struct InjectionConfig {
    // ...
}
```

## Documentation Checklist

For each public function:
- [ ] Summary line describing what it does
- [ ] `# Errors` section if it returns `Result`
- [ ] `# Panics` section if it can panic
- [ ] `# Safety` section if it's unsafe
- [ ] `# Examples` showing typical usage
- [ ] Parameter descriptions if not obvious
- [ ] Return value description if not obvious

For structs:
- [ ] Summary describing purpose
- [ ] Thread safety documentation if applicable
- [ ] Field descriptions (or make fields private)
- [ ] Examples of construction and usage

For async functions:
- [ ] Cancellation safety documentation
- [ ] Expected duration/timeout info
- [ ] Await point considerations

## Tools to Help

1. **cargo doc**: Generate and review documentation
   ```bash
   cargo doc --open --no-deps --document-private-items
   ```

2. **missing_docs lint**: Enable in lib.rs
   ```rust
   #![warn(missing_docs)]
   ```

3. **cargo-rdme**: Sync README with module docs
   ```bash
   cargo install cargo-rdme
   cargo rdme --check
   ```

## Priority Functions to Document

### High Priority (Public API)
1. `StrategyManager::new()`
2. `StrategyManager::inject()`
3. `InjectionConfig` struct and all fields
4. `InjectionError` enum and all variants
5. `InjectionMethod` enum

### Medium Priority (Internal but Complex)
6. `get_current_app_id()`
7. `apply_cooldown()`
8. `update_success_record()`
9. `compute_method_order()`
10. `get_method_order_cached()`

### Low Priority (Simple Helpers)
11. Getter methods
12. Simple predicates like `is_paused()`
13. Internal helper functions

## Location

- Files: `crates/coldvox-text-injection/src/**/*.rs`
- Particularly: `manager.rs`, `types.rs`, `lib.rs`

## Acceptance Criteria

After this issue:
- [ ] All public functions have doc comments
- [ ] All fallible functions document their errors
- [ ] All async functions document cancellation safety
- [ ] Struct concurrency properties are documented
- [ ] `cargo doc` produces no warnings
- [ ] Documentation examples compile and run
- [ ] README matches module-level docs

## Related Standards

Follow Rust API Guidelines:
- https://rust-lang.github.io/api-guidelines/documentation.html
