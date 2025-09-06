# Testing the ColdVox Text Injection Crate

This document outlines the testing strategy for the `coldvox-text-injection` crate, covering both mock-based unit tests and real-world injection tests.

## Test Levels

There are two levels of tests available for this crate, controlled by a feature flag.

### 1. Mock Tests (Default)

These are fast, lightweight unit tests that use mock injectors to verify the internal logic of the `StrategyManager`, configuration parsing, and fallback mechanisms. They do not perform any real text injection and do not require a graphical environment.

**How to Run:**
```bash
cargo test -p coldvox-text-injection --lib
```
or simply:
```bash
cargo test -p coldvox-text-injection
```

These tests are executed by default and are designed to be run frequently by developers and in CI environments where a display server is not available.

### 2. Real Injection Tests

These are full integration tests that launch real (but lightweight) test applications and verify that text is correctly injected by each of the supported backends (`atspi`, `ydotool`, etc.).

**Requirements:**
*   A Linux environment with a running X11 or Wayland display server.
*   Required development libraries for the test applications: `build-essential`, `libgtk-3-dev`.
*   Required runtime dependencies for the injection backends: `at-spi2-core`, `ydotool` (with daemon running), etc. These are typically installed in the CI environment.

**How to Run:**
To enable and run these tests, use the `--features real-injection-tests` flag:
```bash
cargo test -p coldvox-text-injection --features real-injection-tests
```

**What it Does:**
When this feature is enabled, the `build.rs` script for this crate will:
1.  Compile a minimal GTK3 test application.
2.  Compile a minimal terminal test application.

The test suite will then:
1.  Detect if a display server is available. If not, the tests will be skipped with a message.
2.  Launch the test applications as needed for each test case.
3.  Perform text injection using a specific backend.
4.  Verify the injection by reading the content from a temporary file written by the test application.
5.  Automatically clean up all application processes and temporary files.

## Pre-commit Hook

This repository includes a pre-commit hook to ensure that the core logic of the text injection crate remains sound.

**What it Does:**
The pre-commit hook automatically runs the **mock-only tests** (`cargo test -p coldvox-text-injection --lib`). It is very fast and does not require a graphical environment. It serves as a quick sanity check before you commit your changes.

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
