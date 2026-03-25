---
doc_type: plan
subsystem: testing
status: proposed
summary: Scope integration tests by OS to unblock Windows compilation and testing
---

# Scoping Windows vs Linux Integration Tests

> **Status**: PROPOSED
> **Target OS**: Windows, Linux
> **Priority**: High (Unblocks local Windows testing)

## 1. The Problem
Currently, running `cargo test --workspace` or `cargo test -p coldvox-text-injection --features real-injection-tests` fails on Windows. 

The failures are caused by tests that implicitly assume a Linux/Unix environment:
- **Missing Shell Binaries**: `clipboard.rs` unit tests (`test_execute_command_with_stdin_success` and `test_execute_command_with_stdin_timeout`) shell out to `cat` and `sleep`. These binaries are not naturally present in a standard Windows `PATH`.
- **Linux-Specific Display Tools**: The `real-injection-tests` feature executes tests assuming availability of `xdotool`, `ydotool`, and `wl-copy`/`wl-paste` which only exist on X11/Wayland Linux setups.

These failures block developers from running a clean `cargo test --workspace` locally on Windows.

## 2. CI Architecture Context
As outlined in `docs/dev/CI/architecture.md`, we have a strict split:
- **GitHub Hosted**: Runs unit tests (`cargo test --workspace`). This must succeed on any OS.
- **Self-Hosted (Linux KDE)**: Runs the hardware/integration tests (`real-injection-tests`).

Because GitHub Actions (which might run on Ubuntu) has `cat` and `sleep`, it passes CI there but fails locally on Windows. The integration tests correctly run on the Self-Hosted Linux machine, but currently break local Windows development.

## 3. Action Plan

To fix this, we need to strictly scope tests to their supported target operating systems.

### Phase 1: Unit Test Cross-Platform Safety
For the standard unit tests in `clipboard.rs` that use `cat` and `sleep`:
- **Option A (Preferred)**: Gate these specific tests behind `#[cfg(unix)]` since they exist to test the fallback subprocess execution mechanism, which is primarily a Linux/Unix concern (Windows has native clipboard APIs).
- **Option B**: Rewrite the tests to use Rust's `std::thread::sleep` for timeouts and dummy Rust binaries instead of relying on OS-level `cat` and `sleep`.

### Phase 2: Integration Test OS Gating
For tests under `src/tests/real_injection.rs` and the `real-injection-tests` feature:
- Add `#[cfg(target_os = "linux")]` (or `#[cfg(unix)]`) to all tests that shell out to Linux windowing tools (`xdotool`, `at-spi`, `wl-clipboard`).
- Ensure that if `real-injection-tests` is accidentally enabled on Windows, it compiles and gracefully skips the Linux-only tests, rather than failing the entire test suite.

### Phase 3: Windows-Native Clipboard & Injection Tests (Future)
- Once the pure-Rust Windows STT (Parakeet) and Windows native text injection paths are finalized, introduce `#[cfg(target_os = "windows")]` specific integration tests.
- These will test the `SendInput` or Windows Clipboard API behaviors natively without requiring Linux tools.

## 4. Execution
Assign this plan to an agent to perform the safe `#[cfg]` gating on the `coldvox-text-injection` crate. After applying the gates, `cargo test --workspace` must pass on a clean Windows 11 machine without the `cat`/`sleep` assertion panics.
