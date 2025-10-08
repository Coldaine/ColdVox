---
title: "[P1] Dead paste/keystroke code: chunk_and_paste() and pace_type_text()"
labels: ["cleanup", "priority:P1", "component:text-injection"]
---

## Problem

Functions marked with `#[allow(dead_code)]` exist but are never called, adding maintenance burden and confusion about their purpose. These include clipboard chunking and paced typing functions.

## Current Behavior

In `crates/coldvox-text-injection/src/manager.rs` or related files:

```rust
#[allow(dead_code)]
fn chunk_and_paste() {
    // Implementation that's never used
}

#[allow(dead_code)]
fn pace_type_text() {
    // Implementation that's never used
}
```

## Expected Behavior

Should either:

### Option 1: Remove Dead Code
```rust
// Delete functions that are not used
// Remove associated tests
// Remove documentation
```

### Option 2: Integrate into System
```rust
// If the functions are useful, integrate them:
// - Remove #[allow(dead_code)]
// - Add call sites in appropriate injectors
// - Add tests for the integrated functionality
```

### Option 3: Move to Examples/Tests
```rust
// If they're reference implementations:
// Move to examples or test utilities
// Document their purpose clearly
```

## Analysis Needed

For each dead code function, determine:

1. **Why was it written?** - Check git history and comments
2. **Is it still relevant?** - Could it solve current problems?
3. **Is it tested?** - Does removing it break tests?
4. **Is it documented?** - Are there plans to use it?

## Specific Functions to Evaluate

Based on the issue description:
- `chunk_and_paste()` - Clipboard text chunking
- `pace_type_text()` - Rate-limited text typing

Questions:
- Could these solve issues with large text injection?
- Were they experimental features?
- Are they superseded by other implementations?

## Impact

- **Code Clarity**: Reduces confusion about what code is active
- **Maintenance**: Less code to maintain and update
- **Binary Size**: Slightly smaller binaries (though likely optimized out)
- **Cognitive Load**: Developers don't wonder if they should use these functions

## Location

- File: `crates/coldvox-text-injection/src/manager.rs` (and possibly other files)
- Functions: `chunk_and_paste()`, `pace_type_text()`, and any others with `#[allow(dead_code)]`

## Recommendation

1. Search codebase for all `#[allow(dead_code)]` in text-injection crate
2. For each function, make a decision: delete, integrate, or move
3. Prefer deletion unless there's clear value in keeping
4. If keeping, document why and remove the allow attribute
