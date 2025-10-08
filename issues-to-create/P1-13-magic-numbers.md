---
title: "[P1] Magic numbers remain: hardcoded values like 2.0 for backoff factor"
labels: ["refactor", "priority:P1", "component:text-injection"]
---

## Problem

Despite having some configuration constants, magic numbers still appear in the code, making behavior harder to understand and modify. The most notable is the hardcoded `2.0` factor in cooldown calculations.

## Current Occurrences

### 1. Cooldown Factor in Success Rate Calculation

In `crates/coldvox-text-injection/src/manager.rs`:

```rust
fn compute_method_order(&self, app_id: &str) -> Vec<InjectionMethod> {
    // ...
    if rate_a == rate_b {
        pos_a.cmp(&pos_b)
    } else {
        rate_b.partial_cmp(&rate_a).unwrap_or(std::cmp::Ordering::Equal)
    }
}
```

### 2. Decay Factors for Success Records

```rust
pub(crate) fn update_success_record(&mut self, app_id: &str, method: InjectionMethod, success: bool) {
    // Hardcoded decay thresholds
    // ...
}
```

### 3. String Length Constants

```rust
// Various string operations without named constants
```

## Expected Behavior

Define constants at module or config level:

```rust
// At module level
const SUCCESS_RATE_DECAY_FACTOR: f64 = 0.95;
const MIN_SAMPLE_SIZE_FOR_CONFIDENCE: u32 = 5;
const CACHE_TTL_SECONDS: u64 = 300;
const DEFAULT_PRIORITY_WEIGHT: f64 = 1.0;

// Or in InjectionConfig
pub struct InjectionConfig {
    // ... existing fields ...
    
    pub success_rate_decay_factor: f64,
    pub min_sample_size: u32,
    pub cache_ttl_secs: u64,
}
```

## Specific Magic Numbers to Address

1. **2.0** - Used for exponential factors (should use `cooldown_backoff_factor` from config)
2. **0.95** - Decay factor for old success records
3. **5** - Minimum sample size thresholds
4. **300** - Various timeout/duration values
5. **100** - Cache size limits (see P1-12)

## Impact

- **Maintainability**: Hard to find and update behavior thresholds
- **Documentation**: Unclear why specific values were chosen
- **Testing**: Can't easily test with different parameters
- **Tuning**: Requires code changes to adjust behavior

## Location

- File: `crates/coldvox-text-injection/src/manager.rs`
- Multiple functions: `update_success_record()`, `compute_method_order()`, etc.

## Recommendation

1. Audit code for all magic numbers
2. Extract to named constants with documentation
3. Consider which should be configurable vs. fixed
4. Add tests that verify behavior with different values
