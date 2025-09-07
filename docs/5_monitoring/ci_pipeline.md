# CI Pipeline

This document describes the current Continuous Integration (CI) and Release workflows implemented with GitHub Actions.

## Overview

- Primary workflow: `.github/workflows/ci.yml`
- Release workflow: `.github/workflows/release.yml`
- Triggers:
  - CI: pull requests targeting `main` (`opened`, `synchronize`, `reopened`) and manual `workflow_dispatch`.
  - Release: manual `workflow_dispatch` for preparing a release PR via `release-plz`, and automatic release when a release PR is merged to `main`.
- Concurrency:
  - CI groups by ref `ci-${{ github.ref }}`; cancels in-progress runs for the same ref.
  - Release groups by ref `release-${{ github.ref }}`; cancels in-progress runs for the same ref.
- Permissions:
  - CI: `contents: read`, `pull-requests: read`, `checks: write`.
  - Release: `contents: write`, `pull-requests: write`, `issues: write`.
- Global env (CI): `RUST_BACKTRACE=1`, `CARGO_TERM_COLOR=always`, `CARGO_INCREMENTAL=0`.

## CI Jobs (`.github/workflows/ci.yml`)

### validate-workflows
- Purpose: Server-side validation of all workflow files using the GitHub CLI (`gh`). This job is optional and will not fail the CI run if it fails.
- Runner: `ubuntu-latest`.
- Steps:
  - Checkout repository.
  - Enumerate `.github/workflows/*.yml|*.yaml` and validate each via `gh workflow view --ref $GITHUB_SHA --yaml`.
  - Skips cleanly if `gh` is not found or no workflow files are present.

### download-vosk-model
- Purpose: Centralized job to download and cache the Vosk speech recognition model.
- Runner: `ubuntu-latest`.
- Steps:
  - Caches the model directory `models/vosk-model-small-en-us-0.15` using `actions/cache`.
  - On cache miss, it attempts to download the model from the alphacephei.com server with retries.
  - **Graceful Failure**: If the download fails, the job continues without failing the pipeline, but downstream jobs that require the model will skip E2E tests.
- Outputs:
  - `model-path`: The absolute path to the model directory.
  - `download-outcome`: The outcome of the download step (`success` or `failure`), used by dependent jobs to conditionally run tests.

### build_and_check
- Purpose: Consolidated static checks, build, docs, and unit/integration tests.
- Runner: `ubuntu-latest`.
- Needs: `[download-vosk-model]`
- Toolchain and caches:
  - Rust toolchain via `dtolnay/rust-toolchain@v1` (stable) with components `rustfmt` and `clippy`.
  - Cargo build cache via `Swatinem/rust-cache@v2.8.0`.
- System dependencies installed via `apt`:
  - `libasound2-dev`, `libxdo-dev`, `libxtst-dev`, `wget`, `unzip`.
- Rust checks and build (default features):
  - All `cargo` commands are run with the `--locked` flag to ensure `Cargo.lock` is up-to-date.
  - Strict warnings (`-D warnings`) have been removed to prevent failures on harmless compiler warnings.
  - Format check: `cargo fmt --all -- --check`.
  - Lint: `cargo clippy --all-targets --locked`.
  - Typecheck: `cargo check --workspace --all-targets --locked`.
  - Build: `cargo build --workspace --locked`.
  - Docs: `cargo doc --workspace --no-deps --locked`.
- Tests (unit/integration):
  - Runs `cargo test --workspace --locked -- --skip test_end_to_end_wav_pipeline`. This step is conditional and only runs if the Vosk model was successfully downloaded in the `download-vosk-model` job.

### gui-groundwork
- Purpose: Checks that the GUI crate can be built if Qt 6 is present on the runner.
- Runner: `ubuntu-latest`.
- Steps:
  - Detects if Qt 6 is installed.
  - If found, runs `cargo check -p coldvox-gui --features qt-ui --locked`.
  - If not found, the job passes explicitly, acknowledging the missing dependency.

### text_injection_tests
- Purpose: Runs tests for the text injection functionality in a headless graphical environment.
- Runner: `ubuntu-latest`.
- Needs: `[download-vosk-model]`
- Environment:
  - A virtual X11 server (Xvfb) is started, along with the `fluxbox` window manager.
  - D-Bus session is configured.
  - Readiness checks are performed to ensure `dbus` and clipboard tools (`xclip`, `wl-paste`) are available.
- Steps:
  - Installs system dependencies like `xvfb`, `at-spi2-core`, `xclip`, `wl-clipboard`, etc.
  - Installs `libvosk` from the vendored bundle.
  - Tests the `coldvox-text-injection` crate with multiple feature flag combinations (default, no-default, regex-only).
  - Builds the main `coldvox-app` to ensure integration.
  - Runs the end-to-end WAV pipeline test (`test_end_to_end_wav_pipeline`), which is conditional on the successful download of the Vosk model.

### security
- Purpose: Security audit of dependencies (via `rustsec/audit-check` / `cargo audit`).
- Status: Disabled (`if: false`). Retained as a placeholder for future enablement.

### ci-success
- Purpose: Aggregate success marker. Always runs and fails if any of the required jobs fail.
- Needs: `validate-workflows`, `download-vosk-model`, `build_and_check`, `gui-groundwork`, `text_injection_tests`, `security`.

## Release Workflow (`.github/workflows/release.yml`)

### release-plz (manual)
- Trigger: Manual `workflow_dispatch`.
- Purpose: Prepare a release PR using `release-plz`.
- Steps: Checkout (full history), install stable Rust, cache builds, `cargo install release-plz`, then `release-plz release-pr` using `GITHUB_TOKEN`.

### release (auto on merge)
- Trigger: When a pull request merges into `main`.
- Purpose: Create a GitHub release via `release-plz` against the `main` branch.
- Steps: Checkout `main`, install stable Rust, `cargo install release-plz`, then `release-plz release` with `GITHUB_TOKEN`.

## Disabled/Archived Workflows

These are kept under `.github/workflows.disabled/` for reference and may be reintroduced later:
- `ci.yml.disabled`: Earlier multi-job CI with organization-reusable workflow, lockfile check, and feature smoke tests.
- `feature-matrix.yml.disabled`: Systematic feature-combination testing across selected crates.
- `cross-platform.yml.disabled`: Cross-platform matrix (Linux primary, Windows/macOS best-effort) with `cargo-nextest`.
- `vosk-integration.yml.disabled`: Dedicated Vosk integration tests, examples, and failure artifacts.
- `release.yml.disabled`: Previous variant of the release automation.

## Local Parity: Reproducing CI Steps

- Toolchain: `rustup toolchain install stable && rustup component add rustfmt clippy`.
- System packages (Ubuntu/Debian): `sudo apt-get install -y libasound2-dev libxdo-dev libxtst-dev wget unzip xvfb fluxbox dbus-x11 at-spi2-core wl-clipboard xclip ydotool x11-utils wmctrl`.
- Baseline checks (default features):
  - `cargo fmt --all -- --check`
  - `cargo clippy --all-targets --locked`
  - `cargo check --workspace --all-targets --locked`
  - `cargo build --workspace --locked`
  - `cargo doc --workspace --no-deps --locked`
- Run tests (skipping E2E): `cargo test --workspace --locked -- --skip test_end_to_end_wav_pipeline`.
- E2E WAV pipeline test with Vosk:
  - Ensure `libvosk.so` is available on the system library path.
  - Download model to `models/vosk-model-small-en-us-0.15` or set `VOSK_MODEL_PATH`.
  - Run: `VOSK_MODEL_PATH=models/vosk-model-small-en-us-0.15 cargo test -p coldvox-app --locked test_end_to_end_wav_pipeline -- --nocapture`.

## Notes and Limitations

- Pre-commit hooks: Configured for local development in `.pre-commit-config.yaml`. They are not executed in CI; equivalent checks are covered by `fmt`/`clippy`/`check` steps. A hook is also configured to automatically generate and commit dependency graphs to `docs/dependency-graphs/` when `Cargo.toml` or `Cargo.lock` files change.
- Artifacts: The active CI does not upload artifacts on failure.
- Timeouts: No explicit job-level timeouts are set.
- Feature scope: The main checks now run with default features. The `text_injection_tests` job still checks multiple feature flag combinations.

## Troubleshooting

- gh CLI not found in `validate-workflows`:
  - The hosted `ubuntu-latest` includes `gh`. If using a custom runner, install GitHub CLI. The job will be skipped if `gh` is not found.
- Vosk model not found:
  - The `download-vosk-model` job handles this. If the download fails, tests requiring the model will be skipped.
- libvosk loading error:
  - Verify `libvosk.so` is placed under `/usr/local/lib` (or another library path) and `ldconfig` has been run.
