# coldvox-text-injection

A robust, resilient, and fast-failing text injection system for ColdVox.

## Core Architecture

This crate provides text injection capabilities designed for reliability, especially in CI and diverse desktop environments. The architecture is built on three pillars:

1.  **Fast Environment Probe**: Before any injection attempt, a quick, asynchronous probe runs to detect available backends (`AT-SPI`, `wl-paste`, `xclip`) and essential services (`D-Bus`). This probe uses strict timeouts to prevent hangs and allows the system to gracefully skip injection if the environment is not suitable.

2.  **Strict Timeouts & Structured Errors**: Every potentially blocking operation is wrapped in a timeout, from a single D-Bus call to an entire injection attempt. Failures are returned as a structured `InjectionError` enum, making it easy to diagnose the root cause (e.g., `Timeout`, `Unavailable`, `PreconditionNotMet`).

3.  **Resilient Orchestration**: The `StrategyManager` uses the probe's results to select a backend. It no longer relies on complex success-rate caching, opting for a simpler, more predictable flow with a single retry for specific transient errors (like a temporary loss of focus).

## Key Components

### Injection Backends
- **AT-SPI**: The preferred method for accessibility-compliant injection on Linux.
- **Clipboard (Wayland & X11)**: A reliable fallback that uses `wl-clipboard` or `xclip` via robust, timeout-wrapped subprocess calls.
- **Ydotool**: Can be used for synthetic input (feature-gated).
- **NoOp**: A fallback that does nothing, ensuring the injection call never fails catastrophically.

### Fail-Fast Testing
The test suite has been overhauled for robustness:
- All real-hardware tests are wrapped in a watchdog timeout to prevent hangs.
- Tests use the environment probe to skip gracefully if the required backends are not available, providing clear JSON-formatted skip reasons.
- Fixed `sleep()` calls have been replaced with asynchronous polling loops (`wait_ready`) for deterministic test setup.

## Features

- `default`: Core text injection functionality.
- `atspi`: Enables the Linux AT-SPI accessibility backend.
- `wl_clipboard`: Enables the `wl-clipboard-rs` dependency (though the new implementation uses the CLI tools for robustness).
- `ydotool`: Enables the `ydotool` injector.
- `real-injection-tests`: Compiles the real hardware tests, which can be run with the `cargo real-injection` alias.

## Usage

This crate is primarily used by the main ColdVox application.

To run the tests, including the real hardware tests (requires a graphical environment):
```bash
# This new alias runs all tests, including ignored ones, with output.
cargo real-injection
```

## System Requirements

### Linux
- **AT-SPI**: `at-spi2-core` and a running D-Bus session.
- **Clipboard**: `wl-clipboard` (for Wayland) or `xclip` (for X11).
- **Build**: `libgtk-3-dev` is required to build the test application.

## Dependencies

- `tokio`: For the async runtime.
- `async-trait`: For the `TextInjector` trait.
- `thiserror`: For structured error types.
- `heapless`: For lightweight, allocation-free metrics collection.
- `atspi` (optional): For the AT-SPI backend.
