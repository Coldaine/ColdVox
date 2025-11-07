
# Testing in ColdVox

This document provides a comprehensive overview of the testing strategy, organization, and best practices for the ColdVox project.

## Testing Philosophy

The ColdVox project prioritizes robust and reliable testing to ensure the quality and stability of the application. Our testing philosophy is guided by the following principles:

*   **Correctness:** Tests should be accurate and reliable, providing a high degree of confidence in the correctness of the code.
*   **Clarity:** Tests should be easy to understand and maintain, serving as living documentation for the behavior of the application.
*   **Comprehensiveness:** Tests should cover a wide range of scenarios, including edge cases and error conditions.
*   **Automation:** Tests should be automated and integrated into the development workflow to provide rapid feedback.

## Test Organization

The ColdVox project uses a hybrid approach to test organization, combining integration tests in dedicated `tests/` directories with unit tests embedded in source files.

### Integration Tests

Integration tests are located in `tests/` directories at the root of each crate. These tests are designed to verify the interaction between different components of the application and to test end-to-end functionality.

### Unit Tests

Unit tests are embedded in source files using `#[cfg(test)]` modules. These tests are designed to verify the functionality of individual functions and modules in isolation.

### Test Naming Conventions

To ensure consistency and clarity, all test files and functions should follow these naming conventions:

*   Test files should be named with a `_test.rs` or `_tests.rs` suffix (e.g., `chunker_tests.rs`).
*   Test functions should be named with a `test_` prefix (e.g., `test_chunker_can_chunk_audio`).

## Running Tests

Tests can be run using the standard `cargo test` command.

```bash
# Run all tests in the workspace
cargo test --workspace

# Run tests for a specific crate
cargo test -p coldvox-app

# Run a specific test
cargo test -p coldvox-app --test settings_test
```

### Conditional Testing

The ColdVox project uses a sophisticated environment detection system to conditionally run tests based on the availability of certain features or resources. The `coldvox_foundation::test_env` module provides a set of macros and utilities for this purpose.

For example, to skip a test unless a display server is available, you can use the `skip_test_unless!` macro:

```rust
use coldvox_foundation::test_env::*;

#[test]
fn my_gui_test() {
    skip_test_unless!(
        TestRequirements::new()
            .requires_gui()
    );

    // Test code here...
}
```

## Adding New Tests

When adding new tests, please follow these guidelines:

*   Place integration tests in the `tests/` directory of the corresponding crate.
*   Place unit tests in a `#[cfg(test)]` module at the bottom of the file they are testing.
*   Follow the naming conventions for test files and functions.
*   Use the `skip_test_unless!` macro to conditionally run tests that have specific requirements.
*   Ensure that all new code is accompanied by corresponding tests.

## Continuous Integration

The ColdVox project uses GitHub Actions for continuous integration. The CI pipeline is defined in the `.github/workflows/` directory and includes the following workflows:

*   `ci.yml`: The main CI workflow, which runs a comprehensive suite of tests on every push and pull request.
*   `ci-minimal.yml`: A minimal CI workflow that provides a faster feedback loop for common development scenarios.
*   `docs-ci.yml`: A workflow that validates documentation changes.
*   `release.yml`: A workflow that automates the release process.
*   `runner-diagnostic.yml`: A workflow for diagnosing issues with self-hosted runners.
*   `runner-test.yml`: A workflow for testing self-hosted runners.
*   `vosk-integration.yml`: A workflow for running Vosk integration tests.

## Code Coverage

The ColdVox project does not currently have a code coverage reporting mechanism in place. This is a known issue that we plan to address in the future.

## Testing Tools

The ColdVox project does not currently use any external testing frameworks beyond the standard Rust testing framework. However, we are considering the adoption of the following tools to improve our testing workflow:

*   **`cargo-nextest`:** A next-generation test runner for Rust that provides a more interactive and efficient testing experience.
*   **`tarpaulin` or `grcov`:** Tools for generating code coverage reports.

## Recommended Tooling

To address the current weaknesses in our testing setup and to improve the overall developer experience, we recommend the adoption of the following tools:

### `cargo-nextest` for Test Execution

`cargo-nextest` is a next-generation test runner for Rust that offers several advantages over the default `cargo test` runner, including:

*   **Faster test execution:** `cargo-nextest` runs tests in parallel by default, which can significantly reduce test execution time.
*   **More informative output:** `cargo-nextest` provides a more detailed and user-friendly output, making it easier to identify and debug test failures.
*   **Better test filtering:** `cargo-nextest` provides more powerful and flexible options for filtering tests, making it easier to run specific tests or groups of tests.

We recommend using `cargo-nextest` as the default test runner for both local development and CI. To integrate it into the CI pipeline, you can add the following step to your workflow files:

```yaml
- name: Install cargo-nextest
  uses: taiki-e/install-action@nextest

- name: Run tests with cargo-nextest
  run: cargo nextest run --workspace
```

### `tarpaulin` for Code Coverage

`tarpaulin` is a code coverage tool for Rust that provides detailed reports on the parts of your code that are covered by tests. We recommend using `tarpaulin` to generate code coverage reports and to enforce a minimum code coverage threshold for all new code.

To integrate `tarpaulin` into the CI pipeline, you can add the following step to your workflow files:

```yaml
- name: Install tarpaulin
  uses: taiki-e/install-action@tarpaulin

- name: Generate code coverage report
  run: cargo tarpaulin --workspace --out Xml
```

This will generate a code coverage report in Cobertura XML format, which can be easily integrated with CI platforms like GitHub Actions.

### `mockall` for Mocking Dependencies

`mockall` is a powerful mocking library for Rust that can be used to create mock objects for dependencies in unit tests. We recommend considering the use of `mockall` to improve test isolation and to reduce the reliance on real hardware.

By using `mockall`, you can create mock objects that simulate the behavior of real dependencies, allowing you to test your code in a more controlled and predictable environment. This can be particularly useful for testing code that interacts with external services or hardware.

## CI Enhancements

To improve the performance and maintainability of our CI pipeline, we recommend the following enhancements:

### Add a Code Coverage Job

We recommend adding a new job to the `ci.yml` workflow that generates and uploads a code coverage report. This will provide valuable insights into the parts of our code that are not covered by tests and will help us to improve our test coverage over time.

Here's an example of how you can add a code coverage job to your `ci.yml` workflow:

```yaml
jobs:
  # ... other jobs

  code-coverage:
    name: Code Coverage
    runs-on: [self-hosted, Linux, X64, fedora, nobara]
    steps:
      - uses: actions/checkout@v5
      - uses: dtolnay/rust-toolchain@stable
      - name: Install tarpaulin
        uses: taiki-e/install-action@tarpaulin
      - name: Generate code coverage report
        run: cargo tarpaulin --workspace --out Xml
      - name: Upload coverage to Codecov
        uses: codecov/codecov-action@v3
        with:
          files: ./cobertura.xml
          fail_ci_if_error: true
```

### Explore Test Parallelization

To speed up the CI pipeline, we recommend exploring test parallelization strategies. This could involve using `cargo-nextest`'s built-in parallelization features or sharding tests across multiple CI jobs.

By running tests in parallel, we can significantly reduce the time it takes to get feedback from the CI pipeline, which can help to improve developer productivity.

### Use GitHub-Hosted Runners

To reduce our reliance on self-hosted runners and to improve the overall reliability of our CI pipeline, we recommend considering the use of GitHub-hosted runners for certain jobs.

For example, the `docs-ci.yml` workflow, which only runs Python scripts and does not have any special hardware requirements, could be easily migrated to a GitHub-hosted runner. This would free up our self-hosted runners for jobs that have more specific requirements.
