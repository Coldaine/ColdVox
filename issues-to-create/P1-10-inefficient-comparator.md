---
title: "[P1] Inefficient comparator: sort_by uses position() which iterates"
labels: ["performance", "priority:P1", "component:text-injection"]
---

## Problem

The sorting comparator in method ordering uses `.position()` inside the comparison function, causing O(n²) iterations instead of O(n) with a position map.

## Current Behavior

In `crates/coldvox-text-injection/src/manager.rs`:

```rust
let base_order_copy = base_order.clone();
base_order.sort_by(|a, b| {
    let pos_a = base_order_copy.iter()
        .position(|m| m == a)
        .unwrap_or(usize::MAX);
    let pos_b = base_order_copy.iter()
        .position(|m| m == b)
        .unwrap_or(usize::MAX);
    // ... comparison logic
});
```

**Complexity**: O(n²) - for each of n elements, we search through n elements

## Expected Behavior

Build a position map once, then use O(1) lookups:

```rust
use std::collections::HashMap;

// Build position map once: O(n)
let position_map: HashMap<InjectionMethod, usize> = base_order
    .iter()
    .enumerate()
    .map(|(i, m)| (*m, i))
    .collect();

// Sort using O(1) lookups per comparison: O(n log n)
base_order.sort_by(|a, b| {
    let pos_a = position_map.get(a).copied().unwrap_or(usize::MAX);
    let pos_b = position_map.get(b).copied().unwrap_or(usize::MAX);
    // ... comparison logic
});
```

**Complexity**: O(n log n) overall

## Impact

**Performance**:
- With 8 methods: 64 position searches vs 8 map builds + 24 lookups
- Worst case: O(n²) vs O(n log n)
- Matters when method list grows or function is called frequently

**Call Frequency**:
- Called on every injection when cache is cold
- Called after success record updates (if cache invalidation is fixed)

## Measurements

Assuming 8 methods and 10 comparisons during sort:
- Current: ~80 linear searches through 8 elements = 320 comparisons
- Proposed: 8 map insertions + 20 map lookups = 28 operations

**Speedup**: ~11x faster

## Location

- File: `crates/coldvox-text-injection/src/manager.rs`
- Functions: `_get_method_priority()`, `compute_method_order()`

## Notes

- This optimization becomes more valuable as the method list grows
- Can be done in conjunction with fixing duplicate functions (P1-08)
