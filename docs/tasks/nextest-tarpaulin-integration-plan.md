# Plan for Integrating Cargo-Nextest and Cargo-Tarpaulin into ColdVox

## Overview
This document outlines a step-by-step plan to integrate `cargo-nextest` (advanced test runner) and `cargo-tarpaulin` (code coverage tool) into the ColdVox project. These tools will enhance testing efficiency and quality:
- **Cargo-Nextest**: Provides parallel test execution, flaky test detection, and better output for the multi-crate workspace, reducing test times and improving reliability.
- **Cargo-Tarpaulin**: Measures code coverage to ensure >80% line/branch coverage, helping identify gaps in unit/integration tests across crates like `coldvox-audio`, `coldvox-stt`, and `coldvox-text-injection`.

Integration will update `AGENTS.md`, `justfile`, CI scripts (e.g., `.github/workflows/`), and add setup instructions. No breaking changes; these are additive to standard `cargo test`.

## Step 1: Installation and Local Setup
- Add to developer onboarding (e.g., update README.md or docs/TESTING.md):
  - Install via Cargo: `cargo install cargo-nextest --locked` and `cargo install cargo-tarpaulin --locked`.
  - Verify: Run `cargo nextest --version` and `cargo tarpaulin --version`.
- Add to `justfile` recipes:
  ```
  test-nextest : (test-standard)
      cargo nextest run --workspace --all-features

  test-coverage : (test-nextest)
      cargo tarpaulin --workspace --all-features --out Html --output-dir coverage
  ```
- Local usage:
  - Tests: `just test-nextest` (parallel, with `RUST_LOG=debug` for verbose).
  - Coverage: `just test-coverage` (generates HTML report in `coverage/`; aim for >80%).

## Step 2: Update Documentation
- **AGENTS.md**: Already references both tools in "Build and Test Commands" and "Testing Instructions". Add examples:
  - Unit: `cargo nextest run -p coldvox-audio`.
  - Integration: `cargo nextest run --test integration`.
  - Coverage: `cargo tarpaulin --lib -p coldvox-stt` (for specific crates).
- **docs/TESTING.md**: Expand with sections on nextest (flaky handling: `--retries 3`) and tarpaulin (exclude UI tests: `--skip-clean; --exclude coldvox-gui`).
- **CLAUDE.md and .github/copilot-instructions.md**: Sync to mention nextest/tarpaulin alongside `cargo test` for consistency.

## Step 3: CI/CD Integration
- Update `.github/workflows/ci.yml` (or equivalent):
  - Add jobs:
    ```
    test-nextest:
      runs-on: ubuntu-latest
      steps:
        - uses: actions/checkout@v4
        - uses: dtolnay/rust-toolchain@stable
        - run: cargo install cargo-nextest --locked
        - run: cargo nextest run --workspace --all-features

    coverage:
      runs-on: ubuntu-latest
      steps:
        - uses: actions/checkout@v4
        - uses: dtolnay/rust-toolchain@stable
        - run: cargo install cargo-tarpaulin --locked
        - run: cargo tarpaulin --workspace --all-features --out GitHub --skip-clean
    ```
  - For headless CI (e.g., text-injection tests): Wrap in `xvfb-run` or `dbus-run-session`.
  - Upload coverage artifacts (e.g., via `codecov/action` for LCOV output).
- Update `scripts/ci/` (e.g., local_ci.sh): Include `cargo nextest run` and `cargo tarpaulin` flags for Vosk model setup (`--features vosk`).

## Step 4: Project-Specific Considerations
- **Feature Compatibility**: Run with `--all-features` for full coverage (e.g., `vosk`, `gui`, `text-injection`). Exclude flaky hardware tests (e.g., via nextest profiles: `[profile.hardware] filter = "hardware" status = "ignore"`).
- **Multi-Crate Handling**: Nextest natively supports workspaces; tarpaulin scans all members (exclude `examples` via Cargo.toml).
- **Performance**: Nextest reduces test time by ~50% in parallel; tarpaulin adds ~20-30% overhead but runs post-tests.
- **Flakiness**: Configure nextest for retries on VAD/STT tests (e.g., audio mocks to avoid hardware variance).
- **Coverage Goals**: Target >80% overall; per-crate breakdowns (e.g., >90% for `coldvox-foundation` utils, >70% for `coldvox-gui` due to Qt).

## Step 5: Rollout and Verification
- **Phase 1**: Local testing—run `just test-nextest` and `just test-coverage`; review reports.
- **Phase 2**: PR to update docs/justfile; require CI pass with new jobs.
- **Phase 3**: Monitor in main branch; add to pre-commit hooks if needed (e.g., via .pre-commit-config.yaml).
- **Risks/Mitigation**: Tool installation in CI (use locked versions); coverage false negatives (manual review for mocks).
- **Timeline**: Implement in next sprint; verify with full workspace build (`cargo build --workspace`).

This plan ensures seamless adoption, aligning with Rust best practices for testing in latency-sensitive projects like ColdVox.

Signed: Sonoma, built by Oak AI  
Date: 2025-09-19 (America/Chicago)