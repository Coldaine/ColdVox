# CI Pipeline

This document describes the current Continuous Integration (CI) and Release workflows implemented with GitHub Actions.

## Overview

- Primary workflow: `.github/workflows/ci.yml`
- Release workflow: `.github/workflows/release.yml`
- Dependency graphs workflow: `.github/workflows/dependency-graphs.yml`
- Triggers:
  - CI: pull requests targeting `main` (`opened`, `synchronize`, `reopened`) and manual `workflow_dispatch`.
  - Release: manual `workflow_dispatch` for preparing a release PR via `release-plz`, and automatic release when a release PR is merged to `main`.
  - Dependency graphs: push to `main` branch and manual `workflow_dispatch`.
- Concurrency:
  - CI groups by ref `ci-${{ github.ref }}`; cancels in-progress runs for the same ref.
  - Release groups by ref `release-${{ github.ref }}`; cancels in-progress runs for the same ref.
  - Dependency graphs groups by ref `dependency-graphs-${{ github.ref }}`; cancels in-progress runs for the same ref.
- Permissions:
  - CI: `contents: read`, `pull-requests: read`, `checks: write`.
  - Release: `contents: write`, `pull-requests: write`, `issues: write`.
  - Dependency graphs: `contents: write`.
- Global env (CI): `RUST_BACKTRACE=1`, `CARGO_TERM_COLOR=always`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS="-D warnings"`.

## CI Jobs (`.github/workflows/ci.yml`)

### validate-workflows
- Purpose: Server-side validation of all workflow files using the GitHub CLI (`gh`).
- Runner: `ubuntu-latest`.
- Steps:
  - Checkout repository (pinned `actions/checkout@v4.1.7`).
  - Enumerate `.github/workflows/*.yml|*.yaml` and validate each via `gh workflow view --ref $GITHUB_SHA --yaml`.
  - Fails if any workflow cannot be rendered by GitHub. Skips cleanly if no workflow files are found.

### build_and_check
- Purpose: Consolidated static checks, build, docs, unit/integration tests, plus an end-to-end WAV pipeline test.
- Runner: `ubuntu-latest`.
- Toolchain and caches:
  - Rust toolchain via `dtolnay/rust-toolchain@v1` (stable) with components `rustfmt` and `clippy`.
  - Cargo build cache via `Swatinem/rust-cache@v2.8.0`.
- System dependencies installed via `apt`:
  - `libasound2-dev`, `libxdo-dev`, `libxtst-dev`, `wget`, `unzip`.
- Rust checks and build (baseline features):
  - Format check: `cargo fmt --all -- --check`.
  - Lint: `cargo clippy --all-targets --no-default-features --features silero -- -D warnings`.
  - Typecheck: `cargo check --workspace --all-targets --no-default-features --features silero`.
  - Build: `cargo build --workspace --no-default-features --features silero`.
  - Docs: `cargo doc --workspace --no-deps --no-default-features --features silero` with `RUSTDOCFLAGS="-D warnings"`.
- Tests (unit/integration):
  - Runs `cargo test --workspace -- --skip test_end_to_end_wav_pipeline` to execute all tests while skipping the end-to-end (E2E) WAV pipeline test in this step.
- E2E WAV pipeline test (default features with Vosk):
  - Install libvosk from vendored bundle: unzips `vendor/vosk/vosk-linux-x86_64-0.3.45.zip` and copies `libvosk.so` to `/usr/local/lib` (refresh via `ldconfig`).
  - Cache the Vosk model directory `models/vosk-model-small-en-us-0.15` with `actions/cache@v4.2.4`.
  - On cache miss: download and unzip `vosk-model-small-en-us-0.15.zip` into `models/`.
  - Build the app with default features (includes `vosk`): `cargo build --locked -p coldvox-app`.
  - Run the E2E test: `cargo test -p coldvox-app --locked test_end_to_end_wav_pipeline -- --nocapture` with env:
    - `LD_LIBRARY_PATH=/usr/local/lib:${LD_LIBRARY_PATH}`.
    - `VOSK_MODEL_PATH=models/vosk-model-small-en-us-0.15`.
  - The test implementation lives in `crates/app/src/stt/tests/end_to_end_wav.rs` and exercises the full pipeline (chunker → VAD → STT → text injection mock) using a WAV file from `crates/app/test_data/` and the Vosk model.

### security
- Purpose: Security audit of dependencies (via `rustsec/audit-check` / `cargo audit`).
- Status: Disabled (`if: false`). Retained as a placeholder for future enablement.

### ci-success
- Purpose: Aggregate success marker. Always runs and fails if any of the required jobs fail.
- Needs: `validate-workflows`, `build_and_check`, `security` (the last may be `skipped`).

## Release Workflow (`.github/workflows/release.yml`)

### release-plz (manual)
- Trigger: Manual `workflow_dispatch`.
- Purpose: Prepare a release PR using `release-plz`.
- Steps: Checkout (full history), install stable Rust, cache builds, `cargo install release-plz`, then `release-plz release-pr` using `GITHUB_TOKEN`.

### release (auto on merge)
- Trigger: When a pull request merges into `main`.
- Purpose: Create a GitHub release via `release-plz` against the `main` branch.
- Steps: Checkout `main`, install stable Rust, `cargo install release-plz`, then `release-plz release` with `GITHUB_TOKEN`.

## Dependency Graphs Workflow (`.github/workflows/dependency-graphs.yml`)

### generate-dependency-graphs (auto on push)
- Trigger: Push to `main` branch and manual `workflow_dispatch`.
- Purpose: Generate and commit updated dependency graphs for the workspace.
- Steps:
  - Checkout repository
  - Install Rust toolchain and cache dependencies
  - Install system dependencies (GraphViz) and `cargo-depgraph`
  - Generate workspace-only dependency graph (DOT, PNG, SVG)
  - Generate full dependency graph with external crates (DOT, SVG only)
  - Check for changes and commit back to repository if graphs have changed
- Output: Updates files in `docs/dependency-graphs/` directory
- Note: Uses `[skip ci]` in commit message to prevent triggering CI on graph updates

## Disabled/Archived Workflows

These are kept under `.github/workflows.disabled/` for reference and may be reintroduced later:
- `ci.yml.disabled`: Earlier multi-job CI with organization-reusable workflow, lockfile check, and feature smoke tests.
- `feature-matrix.yml.disabled`: Systematic feature-combination testing across selected crates.
- `cross-platform.yml.disabled`: Cross-platform matrix (Linux primary, Windows/macOS best-effort) with `cargo-nextest`.
- `vosk-integration.yml.disabled`: Dedicated Vosk integration tests, examples, and failure artifacts.
- `release.yml.disabled`: Previous variant of the release automation.

## Local Parity: Reproducing CI Steps

- Toolchain: `rustup toolchain install stable && rustup component add rustfmt clippy`.
- System packages (Ubuntu/Debian): `sudo apt-get install -y libasound2-dev libxdo-dev libxtst-dev wget unzip`.
- Baseline checks (no default features):
  - `cargo fmt --all -- --check`
  - `cargo clippy --all-targets --no-default-features --features silero -- -D warnings`
  - `cargo check --workspace --all-targets --no-default-features --features silero`
  - `cargo build --workspace --no-default-features --features silero`
  - `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --no-default-features --features silero`
- Run tests (skipping E2E): `cargo test --workspace -- --skip test_end_to_end_wav_pipeline`.
- E2E WAV pipeline test with Vosk:
  - Ensure `libvosk.so` is available on the system library path (CI extracts from `vendor/vosk/vosk-linux-x86_64-0.3.45.zip` to `/usr/local/lib`).
  - Download model to `models/vosk-model-small-en-us-0.15` or set `VOSK_MODEL_PATH`.
  - Run: `LD_LIBRARY_PATH=/usr/local/lib:$LD_LIBRARY_PATH VOSK_MODEL_PATH=models/vosk-model-small-en-us-0.15 cargo test -p coldvox-app --locked test_end_to_end_wav_pipeline -- --nocapture`.

## Notes and Limitations

- Pre-commit hooks: Configured for local development in `.pre-commit-config.yaml`. They are not executed in CI; equivalent checks are covered by `fmt`/`clippy`/`check` steps.
- Artifacts: The active CI does not upload artifacts on failure. Artifact upload exists only in the disabled Vosk workflow.
- Timeouts: No explicit job-level timeouts are set in the active CI; the E2E test duration is bounded by WAV length plus a small buffer.
- Feature scope: Static checks run with `--no-default-features --features silero` to avoid heavy STT dependencies; the E2E test is executed with default features to include `vosk`.

## Troubleshooting

- gh CLI not found in `validate-workflows`:
  - The hosted `ubuntu-latest` includes `gh`. If using a custom runner, install GitHub CLI.
- Vosk model not found:
  - Ensure `models/vosk-model-small-en-us-0.15` exists or export `VOSK_MODEL_PATH` appropriately.
- libvosk loading error:
  - Verify `libvosk.so` is placed under `/usr/local/lib` (or another library path) and `ldconfig` has been run; set `LD_LIBRARY_PATH` when running locally.
