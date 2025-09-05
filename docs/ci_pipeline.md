# CI Pipeline Documentation

This document consolidates information about the CI pipeline.

## Overview

The CI pipeline is defined in `.github/workflows/ci.yml` and is designed to ensure code quality, correctness, and that the project builds successfully. The pipeline is triggered on pushes to the `main` and `develop` branches, as well as on pull requests targeting `main`.

## Pipeline Jobs

The pipeline consists of the following jobs:

### `validate-workflows`

This job validates the syntax of all GitHub Actions workflows in the repository to prevent syntax errors in workflow files.

### `build_and_check`

This job performs a series of checks and builds the project. It was recently consolidated from four separate jobs (`fmt`, `clippy`, `check`, and `docs`) to improve efficiency and reduce compute time. The steps in this job are:

1.  **Checkout**: Checks out the source code.
2.  **Install Toolchain**: Sets up the Rust toolchain with `rustfmt` and `clippy`.
3.  **Cache**: Caches dependencies to speed up subsequent runs.
4.  **Install Dependencies**: Installs system dependencies required for the build.
5.  **Format Check**: Checks that the code is formatted according to the project's style (`cargo fmt`).
6.  **Clippy Linting**: Lints the code for common errors and style issues (`cargo clippy`).
7.  **Type Check**: Performs a type check of the code (`cargo check`).
8.  **Build**: Builds the project (`cargo build`).
9.  **Build Documentation**: Builds the project documentation (`cargo doc`).

### `test`

This job runs the unit and integration tests. It depends on the `build_and_check` job to complete successfully before it starts. It runs a matrix of tests with different feature combinations to ensure all features work as expected.

### `security`

This job performs a security audit of the project's dependencies to check for known vulnerabilities. This job is currently disabled.

### `ci-success`

This job runs after all other jobs and marks the entire CI run as successful.

## Other CI Workflows

### Feature Matrix Testing

The `.github/workflows/feature-matrix.yml` workflow is used for systematic testing of feature combinations. It is described in more detail in the `docs/testing-framework.md` document.

### Pre-commit Hooks in CI

The same pre-commit hooks that are run locally are also run in the CI pipeline to ensure consistency. This is described in more detail in `docs/pre-commit-yaml-linting.md`.

## Planned Enhancements

### Enable Complete Pipeline Testing

A plan to enable complete pipeline testing in CI is outlined in `docs/tasks/enable_complete_pipeline_testing_ci.md`. The goal is to enable the `vosk-integration.yml` workflow to run on all pull requests.
