---
doc_type: history
subsystem: text-injection
status: archived
freshness: historical
preservation: permanent
last_reviewed: 2026-02-12
owners: Coldaine
version: 1.0.0
---

# Platform-Specific Code Gating Strategy (2025-11-06)

## Context

Branch: `retro-plan-vs-actual`
Platform: Windows (Nobara Linux-targeted code failing on Windows dev machine)

## Problem

Text injection crate failed to compile on Windows due to Unix-only code:

```
error[E0433]: failed to resolve: could not find `unix` in `os`
  --> crates/coldvox-text-injection/src/ydotool_injector.rs:156:17
   |
156 |     use std::os::unix::fs::PermissionsExt;
   |                 ^^^^ could not find `unix` in `os`
```

**Root cause**: `ydotool_injector.rs` unconditionally used `std::os::unix::fs::Permissions`, which doesn't exist on Windows.

## Solution Applied

### File-level and Module-level Gating

**Pattern 1: Real module on Unix, stub module elsewhere**

```rust
// crates/coldvox-text-injection/src/lib.rs

#[cfg(all(unix, feature = "ydotool"))]
pub mod ydotool_injector;

#[cfg(any(not(unix), not(feature = "ydotool")))]
pub mod ydotool_injector {
    // Stub implementation with same public API
    pub struct YdotoolInjector;

    impl YdotoolInjector {
        pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
            Err("ydotool not available on this platform".into())
        }
    }
}
```

**Benefits**:
- Consistent API across platforms (call sites don't need cfg gates)
- Stub returns errors at runtime rather than compile-time failure
- Clear separation: Unix implementation vs cross-platform stub

### Test Gating

```rust
// tests/wl_copy_stdin_test.rs
#[cfg(all(unix, feature = "wl_clipboard"))]
mod tests {
    // Real tests
}

#[cfg(any(not(unix), not(feature = "wl_clipboard")))]
mod tests {
    #[test]
    fn skipped_on_non_unix() {
        println!("wl-clipboard tests only run on Unix");
    }
}
```

**Benefits**:
- Tests compile on all platforms (just print skip message on wrong platform)
- `cargo test --no-run` validates compilation on Windows without runtime failures
- Avoids littering test code with per-platform cfgs

## Secondary Fix: Lifetime Error in Clipboard Restore

While fixing platform issues, discovered lifetime error in `UnifiedClipboardInjector`:

```rust
// BROKEN: Captures &self in spawned task
tokio::spawn(async move {
    self.restore_clipboard(&backup).await  // ← &self escapes!
});
```

`tokio::spawn` requires `'static` future, but `&self` is only valid until method returns.

**Fix**: Don't capture `&self`, move owned data into task

```rust
// FIXED: Move data, use helper that doesn't need &self
let content = backup.content.clone();
tokio::spawn(async move {
    Self::restore_clipboard_direct(content).await
});

// New helper (no &self)
async fn restore_clipboard_direct(content: Vec<u8>) -> Result<()> {
    // Write to clipboard via wl-copy or xclip
}
```

## Platform Gating Playbook

### When to Gate

1. **OS-specific system calls**: Unix-only APIs (`std::os::unix`), Windows-only APIs (`std::os::windows`)
2. **Platform-specific tools**: `ydotool`, `kdotool`, `xdotool` (Linux), `SendInput` (Windows)
3. **Display server dependencies**: X11, Wayland (Linux), Quartz (macOS), Win32 (Windows)
4. **External binary dependencies**: Tools that only exist on one platform

### How to Gate

**Option 1: Module-level with stub fallback** (preferred for backends)

```rust
#[cfg(all(target_os = "linux", feature = "backend"))]
pub mod real_backend;

#[cfg(not(all(target_os = "linux", feature = "backend")))]
pub mod real_backend {
    // Stub that compiles but returns "unavailable" errors
}
```

**Option 2: Inline function bodies** (for small differences)

```rust
pub fn do_thing() -> Result<()> {
    #[cfg(unix)]
    {
        // Unix implementation
    }

    #[cfg(not(unix))]
    {
        Err("Not available on this platform")
    }
}
```

**Option 3: Separate files** (for large platform-specific modules)

```
src/
  backend_unix.rs
  backend_windows.rs
  lib.rs  ← pub mod backend { #[cfg(unix)] include!("backend_unix.rs"); ... }
```

### CI Strategy

**GitHub Actions matrix**:

```yaml
strategy:
  matrix:
    include:
      - os: ubuntu-latest
        features: linux-desktop,wl_clipboard,ydotool
      - os: windows-latest
        features: windows-desktop  # excludes Unix-only features
      - os: macos-latest
        features: macos-desktop
```

**Benefits**:
- Catch platform drift early
- Verify stubs compile
- Platform-appropriate feature sets

## Lessons Learned

1. **Never assume target platform**: Just because you develop on Linux doesn't mean code should fail on Windows

2. **Stub patterns prevent cfg proliferation**: Better to have a stub module with consistent API than scatter `#[cfg(unix)]` everywhere

3. **Lifetime errors surface when mixing sync/async**: `tokio::spawn` + `&self` = instant lifetime error. Extract owned data first.

4. **Test compilation != test execution**: Use `cargo test --no-run` to verify cross-platform compilation without needing test environment

5. **Feature flags + cfg gates = powerful combo**: Can gate both on OS and feature, allowing fine-grained control

## Build Verification Commands

```bash
# Verify Windows compilation (from Linux dev machine)
cargo check --target x86_64-pc-windows-msvc

# Verify Linux compilation (from Windows dev machine)
cargo check --target x86_64-unknown-linux-gnu

# Compile tests without running (cross-platform)
cargo test --no-run

# Check all targets in workspace
cargo check --workspace --all-targets
```

## Future Work

Consider automated platform testing:
- Cross-compilation tests in CI
- Windows VM for actual Windows testing (not just cross-compile)
- Document which features are platform-exclusive vs cross-platform

## References

- Commit introducing gating: git log on branch `retro-plan-vs-actual`
- Rust platform-specific code: https://doc.rust-lang.org/reference/conditional-compilation.html
