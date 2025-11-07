
---
doc_type: standard
subsystem: general
version: 1.0.0
status: draft
owners: [Gemini]
last_reviewed: 2025-11-05
---

# Testing Standard

This document outlines the testing strategy, organization, and best practices for the ColdVox project. It is based on a new testing paradigm for the age of LLMs that emphasizes E2E testing, the use of LLM agents for debugging, and a critical reliance on high-quality, verifiable logging.

## The Three Pillars of Our Testing Paradigm

Our testing paradigm is based on the following three pillars:

1.  **A strong emphasis on E2E and integration tests:** These are the most important tests, as they are the only way to be sure that the application works in the real world.
2.  **The use of LLM agents to debug test failures:** The cost of debugging E2E test failures has been dramatically reduced, so we can afford to run more of them.
3.  **A critical reliance on high-quality, verifiable logging:** The logs are the primary source of information for the LLM agents to use when they are investigating a failure.

## Guidelines for Writing and Reviewing Tests

### Test Organization

*   **Integration and E2E Tests:** The majority of our tests should be integration and E2E tests. These tests should be located in the `tests/` directory at the root of each crate.
*   **Unit Tests:** Unit tests should be used sparingly, and only for the most complex and isolated pieces of logic. They should be located in a `#[cfg(test)]` module at the bottom of the file they are testing.

### Test Naming Conventions

*   Test files should be named with a `_test.rs` or `_tests.rs` suffix (e.g., `chunker_tests.rs`).
*   Test functions should be named with a `test_` prefix (e.g., `test_chunker_can_chunk_audio`).

### Conditional Testing

The `coldvox_foundation::test_env` module provides a set of macros and utilities for conditionally run tests based on the availability of certain features or resources. Use the `skip_test_unless!` macro to conditionally run tests that have specific requirements.

## Guidelines for Writing and Reviewing Logs

### Verifiable Logging

Our logging should be "verifiable," meaning that a log message that says "initialization successful" must be a "true statement." It must mean that the initialization was *actually* successful, not just that the code ran to the point where the log message was emitted.

### Structured Logging

We should use a structured logging format like JSON, which makes it easier for machines (and LLM agents) to parse and analyze the logs.

### Context-Aware Logging

Log messages should include not just a message, but also relevant data and state. For example, instead of just logging "initialization successful," we should log "initialization successful" along with the configuration that was used for the initialization.

### Compile-Time Log Level Filtering

To manage the performance cost of logging, we will use compile-time log level filtering. The `tracing` crate should be configured to have different log levels for debug and release builds.

## Recommended Tooling

*   **`cargo-nextest`:** We will use `cargo-nextest` as the default test runner for both local development and CI.
*   **`tarpaulin`:** We will use `tarpaulin` to generate code coverage reports and to enforce a minimum code coverage threshold for all new code.
*   **`mockall`:** We will use `mockall` sparingly for mocking dependencies in unit tests, with the understanding that all LLM-generated tests must be reviewed by a human developer.

## CI Enhancements

*   **Add a Code Coverage Job:** We will add a new job to the `ci.yml` workflow that generates and uploads a code coverage report.
*   **Explore Test Parallelization:** We will explore test parallelization strategies to speed up the CI pipeline.
*   **Use GitHub-Hosted Runners:** We will consider the use of GitHub-hosted runners for certain jobs to reduce our reliance on self-hosted runners.
