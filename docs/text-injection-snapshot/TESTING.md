---
doc_type: testing-guide
subsystem: text-injection
version: 2.0.0
status: active
owners: ColdVox Team
last_reviewed: 2025-09-19
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

**Test Execution Process:**
The `build.rs` script automatically:
1.  Compiles minimal GTK3 test applications
2.  Compiles minimal terminal test applications

The test suite:
1.  Launches test applications for each test case
2.  Performs text injection using specific backends
3.  Verifies injection by reading content from temporary files
4.  Automatically cleans up processes and temporary files

## Clipboard Behavior Tests

Because clipboard-based injection modifies system clipboard contents during injection, the crate implements an automatic restore mechanism: clipboard injectors save the prior clipboard contents and restore them after a configurable delay (default 500ms). Tests that validate clipboard-based injection should:

- Verify that the injected text appears in the target application.
- Verify that the system clipboard is returned to its prior value after the configured delay (use `clipboard_restore_delay_ms` to shorten delays in CI).

When running tests in CI, prefer a short `clipboard_restore_delay_ms` to reduce timing-related flakiness.

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
