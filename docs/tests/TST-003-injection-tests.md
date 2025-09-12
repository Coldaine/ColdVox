---
id: TST-003
title: Injection Tests
level: test
status: drafting
owners:
  - CDIS
criticality: 3
parent: null
pillar_trace:
  - PIL-003
  - DOM-003
  - SUB-003
  - SYS-003
verifies:
  - "SPEC-003"
---

# Injection Tests [TST-003]

## 1. Overview

This document describes the test suite for the Injection Manager System, which verifies the functionality defined in [SPEC-003](SPEC-003-injection-api.md). Testing for this crate is complex due to its interactions with the desktop environment.

## 2. Code-level Traceability

The tests for this crate are located in inline `#[cfg(test)]` modules within the `src/` directory. A key testing document is also available:

-   **Testing Documentation**: `CODE:repo://crates/coldvox-text-injection/TESTING.md`
-   *(Conceptual: Specific test modules for each injector exist within their respective source files.)*

## 3. Test Cases

The test suite relies heavily on mock objects and a controlled environment to validate behavior without needing a full desktop session.

-   **Mock Injectors**: The test suite includes mock implementations of the `Injector` trait that can be configured to succeed or fail, allowing for testing of the `StrategyManager`'s fallback logic.
-   **Mock Focus Provider**: A mock `FocusProvider` is used to simulate different applications having focus, ensuring the manager selects the correct strategy.

## 4. Test Strategy

-   **Headless CI Environment**: The tests are designed to run in a headless CI environment. This is achieved by using `dbus-run-session` to create a session bus, and running the tests within a minimal window manager like `fluxbox` inside an `Xvfb` virtual framebuffer.
-   **Test Command**: The standard command for running these tests is `dbus-run-session -- cargo test -p coldvox-text-injection`.
-   **Feature-gated Tests**: Different test configurations are run with and without default features to ensure all backends and logic paths are tested.

For more detailed information, refer to the `TESTING.md` file within the `coldvox-text-injection` crate.
