---
title: "[P2] CI knobs in production: cfg!(test) and CI checks in runtime code"
labels: ["refactor", "priority:P2", "component:text-injection"]
---

## Problem

Production runtime code contains `cfg!(test)` and environment variable checks for `CI`, mixing test/CI concerns with production logic. This creates confusing behavior, reduces code clarity, and can lead to different behavior in test vs production.

## Current Behavior

Common patterns in the codebase:

```rust
// In production code
if cfg!(test) {
    // Special behavior during tests
} else {
    // Normal behavior
}

// Environment checks in runtime
if std::env::var("CI").is_ok() {
    // Different behavior in CI
}
```

## Examples to Find

```bash
# Find test-conditional code
grep -r "cfg!(test)" crates/coldvox-text-injection/src/

# Find CI checks
grep -r "CI" crates/coldvox-text-injection/src/ | grep env::var

# Find test-specific compilation
grep -r "#\[cfg(test)\]" crates/coldvox-text-injection/src/
```

## Problems with Current Approach

1. **Behavior Divergence**: Tests may not test actual production behavior
2. **Code Clarity**: Production code polluted with test concerns
3. **Debugging**: Hard to understand what code actually runs in production
4. **Security**: Could leak test-only backdoors into production
5. **Maintainability**: Changes require considering test vs production paths

## Expected Behavior

### Pattern 1: Dependency Injection
Instead of runtime checks, inject dependencies:

```rust
// Bad
impl StrategyManager {
    fn get_time(&self) -> Instant {
        if cfg!(test) {
            MOCK_TIME.with(|t| *t)
        } else {
            Instant::now()
        }
    }
}

// Good
trait TimeSource {
    fn now(&self) -> Instant;
}

struct SystemTime;
impl TimeSource for SystemTime {
    fn now(&self) -> Instant { Instant::now() }
}

struct MockTime { /* ... */ }
impl TimeSource for MockTime {
    fn now(&self) -> Instant { /* mock */ }
}

impl StrategyManager {
    fn with_time_source(time_source: Arc<dyn TimeSource>) -> Self { /* ... */ }
}
```

### Pattern 2: Feature Flags for Test Helpers
```rust
// In lib.rs or test_helpers module
#[cfg(test)]
pub mod test_helpers {
    // All test-specific helpers
}
```

### Pattern 3: Compilation-Time Separation
```rust
// Good: Compile-time feature separation
#[cfg(test)]
mod tests {
    // Test code here
}

// Production code has no test conditionals
```

### Pattern 4: Test-Only Trait Implementations
```rust
#[cfg(test)]
impl StrategyManager {
    // Test-only constructor or methods
    pub fn with_mock_backends(/* ... */) -> Self { /* ... */ }
}
```

## Specific Issues to Address

### 1. Time-Based Testing
Current: Likely has `cfg!(test)` for time mocking
Solution: Inject `TimeSource` trait

### 2. Backend Availability
Current: May check CI env to skip backend detection
Solution: Accept backends as constructor parameter in tests

### 3. Timeout Values
Current: May reduce timeouts in CI
Solution: Make timeouts configurable via `InjectionConfig`

### 4. Logging Behavior
Current: May suppress logs in CI
Solution: Use proper test log configuration (tracing-test crate)

### 5. Process Spawning
Current: May mock process spawns differently in tests
Solution: Inject command executor trait

## Migration Strategy

1. **Identify All Occurrences**: Search for patterns above
2. **Categorize**: Group by what's being tested/mocked
3. **Design Abstractions**: Create traits for testability
4. **Refactor Incrementally**: One abstraction at a time
5. **Update Tests**: Use new test helpers
6. **Verify**: Ensure tests still pass and test real behavior

## Benefits

- **Production Confidence**: Production code is exactly what runs
- **Test Validity**: Tests actually test production behavior
- **Code Quality**: Clearer separation of concerns
- **Maintainability**: Easier to understand and modify
- **Type Safety**: Dependency injection caught at compile time

## Location

- Files: `crates/coldvox-text-injection/src/**/*.rs`
- Particularly check: `manager.rs`, `session.rs`, injector implementations

## Related Issues

- P2-17: Missing targeted tests (will need proper abstractions for mocking)
- P0-04: No timeouts (making timeouts configurable helps testing)

## Acceptance Criteria

After this issue is resolved:
- [ ] No `cfg!(test)` in non-test code paths
- [ ] No `env::var("CI")` checks in runtime code
- [ ] Test-only code is in `#[cfg(test)]` modules
- [ ] Testability achieved through dependency injection
- [ ] All tests still pass with same coverage
- [ ] Production behavior unchanged
