---
doc_type: reference
subsystem: text-injection
status: draft
freshness: stale
preservation: preserve
last_reviewed: 2025-11-06
owners: Text Injection Maintainers
version: 1.0.0
---

# Testing the ColdVox Text Injection Crate

This document outlines the testing strategy for the `coldvox-text-injection` crate, using real text injection testing with actual desktop applications.

## Test Architecture

All test runs include both unit tests (with mocks for coordination logic) and real injection tests with actual desktop applications. **No mock-only test paths are permitted** - any test execution must include corresponding real hardware validation.

### Test Coverage Requirements

**Every test run includes:**
1. **Unit tests**: Mock-based testing of strategy management and coordination logic
2. **Real injection tests**: Actual text injection using real desktop applications and injection backends

**How to Run:**
```bash
cargo test -p coldvox-text-injection
```

This runs both mock-based unit tests AND real injection tests with actual desktop applications. All environments (development and self-hosted CI) have full desktop environments available for complete validation.

### Test Environment Requirements

**All environments have the following available:**
*   Linux environment with running X11 or Wayland display server
*   Development libraries: `build-essential`, `libgtk-3-dev`
*   Runtime dependencies: `at-spi2-core`, `ydotool` (with daemon), etc.
*   Full desktop environment for comprehensive testing

**ydotool daemon setup (Wayland clipboard fallback):**
- Install `ydotool` and run `./scripts/setup_text_injection.sh` to generate the user service unit
- Ensure the `ydotoold` user service is enabled (`systemctl --user status ydotool.service`)
- Confirm the environment exports `YDOTOOL_SOCKET=$HOME/.ydotool/socket`
- Verify that the socket (`~/.ydotool/socket`) exists before running real injection tests

**Test Execution Process:**
The `build.rs` script automatically:
1.  Compiles minimal GTK3 test applications
2.  Compiles minimal terminal test applications

The test suite:
1.  Launches test applications for each test case
2.  Performs text injection using specific backends
3.  Verifies injection by reading content from temporary files
4.  Automatically cleans up processes and temporary files

## Pre-commit Hook

This repository includes a pre-commit hook to ensure text injection functionality remains sound.

**What it Does:**
The pre-commit hook automatically runs the full text injection tests (`cargo test -p coldvox-text-injection`) with real desktop applications. Since all environments have desktop environments available, this provides comprehensive validation.

**Installation:**
To install the hook, run the following script from the repository root:
```bash
./scripts/setup_hooks.sh
```

This will create a symlink from `.git/hooks/pre-commit` to the script in the repository.

**Opting Out:**
You can skip the hook installation by setting the `COLDVOX_SKIP_HOOKS` environment variable:
```bash
COLDVOX_SKIP_HOOKS=1 ./scripts/setup_hooks.sh
```
You can also temporarily bypass the hook for a single commit using the `--no-verify` git flag:
```bash
git commit --no-verify -m "Your commit message"
```

## Testing Benefits

The real hardware testing approach provides comprehensive validation:

### Complete Coverage
All tests use real desktop applications and injection backends, ensuring:
- Full validation of production behavior
- Real error handling and fallback scenarios
- Complete multi-backend integration testing
- Actual desktop environment compatibility

### Reliable Execution
Tests run consistently across all environments:
- Development environments have full desktop setup
- Self-hosted CI runners have complete desktop environments
- No environment-specific skipping or mocking needed
- Consistent behavior validation across all platforms

### Comprehensive Validation
Real injection testing covers:
- Actual text injection with GTK and terminal applications
- Real-world backend compatibility (`atspi`, `ydotool`, etc.)
- Complete fallback chain testing
- Production-accurate error conditions and handling

This approach ensures that all tests validate actual production functionality without the limitations of mocked dependencies.

## Known Failure Scenarios

The following failure modes are documented for awareness and test coverage planning. Each describes an expected runtime condition and the system's designed response.

| Scenario | Detection | Expected Behavior | User-Facing Message |
|----------|-----------|-------------------|---------------------|
| AT-SPI bus not running | D-Bus connection timeout | Fall back to clipboard-based injection | (silent; fallback is automatic) |
| No application has focus | AT-SPI returns no focused object | Skip injection; wait for focus | "Please click on the target application" |
| Focused element is not editable | No EditableText interface on focused object | Fall back to clipboard path | (silent; handled internally) |
| Clipboard locked by another app | Clipboard operation times out | Skip clipboard methods; use remaining methods | (silent) |
| Unicode character has no keycode | Keymap lookup returns nothing | Use clipboard for that text chunk | (silent) |
| Rate-limited (rapid injections) | Multiple injections within tight window | Queue and batch via session buffering | (silent) |
| All methods exhausted | Every method in the chain failed | Return `AllMethodsFailed` error | "No injection method available in this session" |
| Budget exhausted | Total latency budget exceeded before all methods tried | Return `BudgetExhausted` error | (silent; logged) |

## Performance Budgets

The injection system enforces timing budgets at multiple levels to prevent runaway attempts:

| Operation | Default Budget | Notes |
|-----------|---------------|-------|
| Per-method attempt | 250 ms | Configurable via `per_method_timeout_ms` |
| Total injection | 800 ms | Configurable via `max_total_latency_ms` |
| Confirmation check | 75 ms | Fixed; 7 polls at 10ms intervals |
| Paste action | 200 ms | Configurable via `paste_action_timeout_ms` |
| Focus cache validity | 200 ms | Configurable via `focus_cache_duration_ms` |
| Clipboard restore delay | 500 ms | Configurable via `clipboard_restore_delay_ms` |

These budgets ensure the system fails fast and moves to the next method rather than blocking indefinitely on an unresponsive backend.
