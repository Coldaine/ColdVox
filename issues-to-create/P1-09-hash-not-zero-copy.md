---
title: "[P1] 32-bit hash and non-zero-copy: redact_text() returns String not Cow"
labels: ["performance", "priority:P1", "component:text-injection"]
---

## Problem

The `redact_text()` function has two efficiency issues:
1. Uses only 32 bits of hash (`hash & 0xFFFFFFFF`) which increases collision risk
2. Returns `String` instead of `Cow<str>`, forcing allocation even for non-redacted text

## Current Behavior

In `crates/coldvox-text-injection/src/manager.rs`:

```rust
fn redact_text(text: &str, redact: bool) -> String {
    if redact {
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let hash = hasher.finish();
        format!("len={} hash={:08x}", text.len(), (hash & 0xFFFFFFFF))
    } else {
        text.to_string()  // Unnecessary allocation!
    }
}
```

## Expected Behavior

Use zero-copy with `Cow` and full 64-bit hash:

```rust
use std::borrow::Cow;

fn redact_text(text: &str, redact: bool) -> Cow<'_, str> {
    if redact {
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let hash = hasher.finish();
        Cow::Owned(format!("len={} hash={:016x}", text.len(), hash))
    } else {
        Cow::Borrowed(text)
    }
}
```

## Impact

- **Performance**: Unnecessary allocations on every log call when not redacting
- **Hash Collisions**: 32-bit hash increases collision probability from 1/2^64 to 1/2^32
- **Memory**: Extra allocations add GC pressure

## Measurements

For a typical injection with 100 chars:
- Current: Allocates 100 bytes even when `redact=false`
- Proposed: Zero allocation when `redact=false`

At 1000 injections/sec:
- Current: ~100 KB/sec extra allocations
- Proposed: 0 extra allocations

## Location

- File: `crates/coldvox-text-injection/src/manager.rs`
- Function: `redact_text()`

## Notes

- This is called on hot paths in the injection pipeline
- The function is used for privacy-preserving logging
- Full 64-bit hash maintains privacy while reducing collision risk
