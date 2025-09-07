# Testing the ColdVox Text Injection Crate

This document outlines the testing strategy for `coldvox-text-injection`, which has been redesigned for robustness and reliability.

## Test Philosophy: Fail Fast, No Hangs

The entire test suite is built around a core philosophy: tests must never hang, even if the underlying system services are slow, misconfigured, or missing. This is achieved through several mechanisms:

1.  **Environment Probe**: Before running any real injection logic, tests use the `probe_environment` function to check for necessary components. If the environment is not suitable, tests are skipped gracefully with a clear, structured JSON message indicating the reason.
2.  **Watchdog Timeouts**: Every `#[tokio::test]` that performs real I/O is wrapped in a global timeout (e.g., 10 seconds). This acts as a watchdog to prevent any single test from hanging the entire suite.
3.  **Deterministic Harness**: A new `TestApp` harness is used to launch a GTK test application. Instead of using fixed `sleep()` calls, tests use an async `wait_ready()` method that polls until the application is initialized, making the setup much more reliable.
4.  **Robust Subprocess Calls**: All interactions with external command-line tools (like `wl-paste` or `xclip`) are wrapped in their own short timeouts to prevent hangs.

## Test Levels

### 1. Unit & Smoke Tests (Always On)

These are fast, lightweight tests that run on every `cargo test` command. They do not require a graphical environment and are safe for any CI pipeline.

-   **Unit Tests**: Verify the internal logic of individual components.
-   **Smoke Test**: A crucial, always-on test (`smoke_test_manager_init_and_probe`) that initializes the `StrategyManager` and runs the `probe_environment` function. This ensures the core machinery is sound and that the probing logic itself doesn't hang, even in a minimal environment.

**How to Run:**
```bash
cargo test -p coldvox-text-injection
```

### 2. Real Injection Tests (Feature-Gated)

These are full integration tests that perform real text injection into a live GTK application. They are gated behind the `real-injection-tests` feature.

**Requirements:**
- A Linux environment with a running X11 or Wayland display server.
- `at-spi2-core`, `wl-clipboard`, `xclip` must be installed for the tests to pass.
- `build-essential` and `libgtk-3-dev` are needed to compile the GTK test app.

**How to Run:**
A convenient `cargo` alias has been created for running these tests.

```bash
# This is the recommended way to run the full test suite.
cargo real-injection
```

This alias expands to `cargo test -p coldvox-text-injection --features real-injection-tests -- --ignored --nocapture`, which ensures all tests run and their output is visible.

## Pre-commit Hook

The pre-commit hook (`.git-hooks/pre-commit-injection-tests`) has been updated.

- By default, it runs only the fast, mock-only unit tests.
- You can optionally run the **full real injection suite** by setting an environment variable before committing:
  ```bash
  RUN_REAL_INJECTION_TESTS=1 git commit -m "Your message"
  ```
This provides an extra layer of validation for developers working directly on the text injection logic, while keeping the default pre-commit check fast for everyone else.
