---
title: "[P2] Dead code preserved with #[allow(dead_code)] throughout"
labels: ["cleanup", "priority:P2", "component:text-injection"]
---

## Problem

Multiple functions and structs are marked with `#[allow(dead_code)]`, indicating they're not currently used but kept in the codebase. This creates maintenance burden, confusion, and may hide actually useful functionality that should be integrated.

## Current Occurrences

Based on the issue description and typical patterns:

### Known Dead Code
```rust
#[allow(dead_code)]
fn chunk_and_paste() { /* ... */ }

#[allow(dead_code)]
fn pace_type_text() { /* ... */ }

#[allow(dead_code)]
pub fn get_method_order_uncached(&self) -> Vec<InjectionMethod> { /* ... */ }
```

### Search Strategy
```bash
# Find all dead code annotations
grep -r "#\[allow(dead_code)\]" crates/coldvox-text-injection/src/

# Find unused functions (Rust compiler warnings)
cargo build 2>&1 | grep "never used"
```

## Decision Framework

For each piece of dead code, answer:

1. **Why was it added?**
   - Check git history: `git log -p --all -S "function_name"`
   - Read commit messages and PR discussions

2. **Is it incomplete functionality?**
   - Was it part of a partial implementation?
   - Is there a plan to complete it?

3. **Is it superseded?**
   - Has a better implementation replaced it?
   - Is it redundant with existing code?

4. **Is it future-proofing?**
   - Is it for planned features?
   - Should it be in a feature branch instead?

5. **Is it tested?**
   - Does it have unit tests?
   - Would removing it break tests?

## Actions by Category

### Category 1: Remove Completely
For genuinely unused code with no future plans:
```rust
// DELETE the function
// DELETE associated tests
// DELETE from documentation
```

### Category 2: Integrate and Activate
For useful code that should be part of the system:
```rust
// Remove #[allow(dead_code)]
// Add call sites
// Add/update tests
// Document usage
```

### Category 3: Move to Examples
For reference implementations or demonstrations:
```rust
// Move to examples/ directory
// Add documentation about purpose
// Keep as working example code
```

### Category 4: Convert to Conditional Compilation
For platform-specific or feature-gated code:
```rust
#[cfg(feature = "experimental")]
fn experimental_feature() { /* ... */ }
```

### Category 5: Document and Keep
For code that's genuinely meant for future use:
```rust
/// Reserved for future clipboard chunking implementation
/// See issue #XXX for tracking
#[allow(dead_code)]
fn chunk_and_paste() { /* ... */ }
```

## Specific Code to Evaluate

### From Issue Description
1. `chunk_and_paste()` - Clipboard chunking (P1-14)
2. `pace_type_text()` - Rate-limited typing (P1-14)
3. `get_method_order_uncached()` - Back-compat method (mentioned in manager.rs)

### To Find
4. Search for all `#[allow(dead_code)]` in text-injection crate
5. Run `cargo clippy` to find unused functions
6. Check for unused imports, structs, traits

## Expected Outcome

After this issue:
- No `#[allow(dead_code)]` annotations without documentation
- Clear plan for each piece of "future" code
- Reduced codebase size and complexity
- Better signal-to-noise ratio for developers

## Implementation Plan

1. **Audit**: List all dead code with annotations
2. **Categorize**: Classify each piece using decision framework
3. **Document Decisions**: Create decision log for reference
4. **Execute**: Remove, integrate, or document as appropriate
5. **Verify**: Ensure tests still pass
6. **Update**: Update documentation and comments

## Location

- Files: `crates/coldvox-text-injection/src/*.rs`
- Particularly: `manager.rs`, `session.rs`, individual injector files

## Related Issues

- P1-14: Dead paste/keystroke code (specific functions)
- P2-16: God method (may reveal more unused code after refactoring)

## Notes

- Be conservative: when in doubt, document rather than delete
- Check git history before removing to understand intent
- Consider creating feature flags for experimental code rather than dead code annotations
