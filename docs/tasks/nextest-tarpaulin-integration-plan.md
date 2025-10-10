# Plan for Integrating Cargo-Nextest and Cargo-Tarpaulin into ColdVox

## Overview
This plan integrates `cargo-nextest` (advanced test runner) and `cargo-tarpaulin` (coverage) tailored to ColdVox’s current setup:
- **Cargo-Nextest**: Faster, clearer test runs in a multi-crate workspace; supports retries and better flake handling.
- **Cargo-Tarpaulin**: Line/branch coverage for core crates under Linux with self-hosted runners.

Key constraints and assumptions from this repo:
- CI uses self-hosted Linux runners with real audio and desktop stack: `[self-hosted, Linux, X64, fedora, nobara]`.
- STT tests require a Vosk model and native lib with env: `VOSK_MODEL_PATH` and `LD_LIBRARY_PATH` (provided by `scripts/ci/setup-vosk-cache.sh`).
- Some tests (Vosk/STT, text injection) are sensitive to parallelism and headless environments; tune concurrency and scopes accordingly.

No breaking changes; these augment standard `cargo test`.

## Step 1: Installation and Local Setup
- Add to developer onboarding (e.g., `docs/TESTING.md`):
  - Install via Cargo: `cargo install cargo-nextest --locked` and `cargo install cargo-tarpaulin --locked`.
  - Verify: `cargo nextest --version` and `cargo tarpaulin --version`.
- Local config (optional, recommended): create `.config/nextest.toml` with sensible defaults (retries, timeouts). Keep per-binary thread limits to CLI for now to avoid brittle filters.
  ```toml
  # .config/nextest.toml (minimal)
  [profile.default]
  retries = 2
  fail-fast = false
  status-level = "failures"
  final-status-level = "flaky"
  slow-timeout = { period = "120s", terminate-after = 2 }
  ```
- Add to `justfile` (don’t chain coverage after nextest—coverage re-runs tests):
  ```make
  # Run workspace tests with nextest; autodetect local Vosk model
  test-nextest:
      #!/usr/bin/env bash
      set -euo pipefail
      if [[ -d "models/vosk-model-small-en-us-0.15" ]]; then
        export VOSK_MODEL_PATH="models/vosk-model-small-en-us-0.15"
      fi
      cargo nextest run --workspace --locked

  # Coverage for core crates only, with Vosk enabled; excludes GUI & Text Injection initially
  test-coverage:
      #!/usr/bin/env bash
      set -euo pipefail
      export VOSK_MODEL_PATH="${VOSK_MODEL_PATH:-models/vosk-model-small-en-us-0.15}"
      mkdir -p coverage
      cargo tarpaulin \
        --locked \
        --packages coldvox-foundation coldvox-telemetry coldvox-audio coldvox-vad coldvox-vad-silero coldvox-stt coldvox-stt-vosk \
        --features vosk \
        --exclude coldvox-gui --exclude coldvox-text-injection \
        --out Html --out Lcov --output-dir coverage
  ```
- Local usage:
  - Tests: `just test-nextest` (use `-j 1 --test-threads 1` selectively when debugging Vosk flakiness).
  - Coverage: `just test-coverage` (outputs HTML + LCOV in `coverage/`).

## Step 2: Update Documentation
- **docs/TESTING.md**: Add a Nextest section (retries, failure output, selecting packages) and a Coverage section (Linux-only, self-hosted, ptrace requirement, initial exclusions). Example commands:
  - `cargo nextest run --workspace --locked`
  - `cargo nextest run -p coldvox-audio`
  - `cargo tarpaulin -p coldvox-stt --features vosk --out Html --output-dir coverage`
- **README.md**: Briefly mention nextest as the preferred local runner, with a link to the testing guide.
- Keep `.github/copilot-instructions.md` in sync for references to nextest/coverage where appropriate.

## Step 3: CI/CD Integration
Align with current self-hosted runners and Vosk setup.

- Update `.github/workflows/ci.yml` to use nextest in the main test step (stable only), keeping the existing Vosk dependency setup:
  ```yaml
  jobs:
    build_and_check:
      runs-on: [self-hosted, Linux, X64, fedora, nobara]
      needs: [setup-vosk-dependencies]
      steps:
        - uses: actions/checkout@v4
        - uses: actions-rust-lang/setup-rust-toolchain@v1
          with:
            toolchain: stable
            components: rustfmt, clippy
        - name: Setup ColdVox
          uses: ./.github/actions/setup-coldvox
        - name: Run tests with nextest (workspace)
          env:
            VOSK_MODEL_PATH: ${{ needs.setup-vosk-dependencies.outputs.model_path }}
            LD_LIBRARY_PATH: ${{ needs.setup-vosk-dependencies.outputs.lib_path }}
          run: |
            cargo nextest run --workspace --locked
  ```

- Keep `vosk-integration.yml` using nextest for `coldvox-stt-vosk` (already present). If needed, limit threads for STT in that job with `--test-threads 1`.

- Add a dedicated coverage job (self-hosted only). Start with core crates to avoid GUI/Text Injection fragility under coverage:
  ```yaml
  jobs:
    coverage:
      runs-on: [self-hosted, Linux, X64, fedora, nobara]
      needs: [setup-vosk-dependencies]
      steps:
        - uses: actions/checkout@v4
        - uses: dtolnay/rust-toolchain@v1
          with:
            toolchain: stable
        - run: cargo install cargo-tarpaulin --locked
        - name: Run tarpaulin for core crates
          env:
            VOSK_MODEL_PATH: ${{ needs.setup-vosk-dependencies.outputs.model_path }}
            LD_LIBRARY_PATH: ${{ needs.setup-vosk-dependencies.outputs.lib_path }}
          run: |
            mkdir -p coverage
            cargo tarpaulin \
              --locked \
              --packages coldvox-foundation coldvox-telemetry coldvox-audio coldvox-vad coldvox-vad-silero coldvox-stt coldvox-stt-vosk \
              --features vosk \
              --exclude coldvox-gui --exclude coldvox-text-injection \
              --out Lcov --out Html --output-dir coverage
        - uses: actions/upload-artifact@v4
          if: always()
          with:
            name: coverage
            path: coverage/
  ```

- Optional: publish LCOV to Codecov after validating coverage stability.

- Update `scripts/local_ci.sh` to prefer nextest for local parity and to expose `VOSK_MODEL_PATH` (using the existing setup script or model autodetect).

## Step 4: Project-Specific Considerations
- **Feature scope**: Avoid `--all-features` in CI by default. Prefer default features plus `vosk` where needed. Build GUI and text-injection in their dedicated jobs, not in coverage.
- **Concurrency**: Some STT/Vosk and text injection tests are sensitive to parallelism.
  - Use nextest retries and, when necessary, `--test-threads 1` for those packages/jobs.
  - You can also run with `-j 1` temporarily to diagnose flakes.
- **E2E WAV**: Keep the end-to-end WAV pipeline test either as a separate step (for logs) or select it via nextest filters; ensure `VOSK_MODEL_PATH` is set.
- **Tarpaulin constraints**: Requires Linux with ptrace; GUI and DBus/AT-SPI can be brittle under coverage. Start by excluding `coldvox-gui` and `coldvox-text-injection`. Expand incrementally once stable.
- **Performance**: Nextest typically reduces runtime substantially; tarpaulin adds overhead—run coverage on-demand or on schedule.
- **Coverage goals**: Initial goal >80% lines on core crates (`foundation`, `audio`, `vad`, `stt`). GUI and text injection may have lower practical ceilings; evaluate separately.
- **Alternative**: Consider `cargo llvm-cov` later for faster, more robust coverage with nextest integration if tarpaulin proves fragile.

## Step 5: Rollout and Verification
- **Phase 1 (local)**: Add `.config/nextest.toml`, extend `justfile`, validate `cargo nextest run` across workspace. Verify Vosk model autodetection works locally.
- **Phase 2 (CI)**: Switch main CI test step to nextest on stable; keep Vosk integration job; add scoped coverage job for core crates.
- **Phase 3 (expand)**: If stable, gradually include additional crates/features in coverage; tune nextest concurrency per package.
- **Risks/Mitigations**:
  - Env drift: Always set `VOSK_MODEL_PATH`/`LD_LIBRARY_PATH` for STT runs.
  - Flakiness: Use retries and limit threads for Vosk/text-injection tests.
  - Coverage fragility: Start with core crates; add GUI/injection only after stabilizing.
- **Timeline**: Implement nextest switch + docs in current sprint; add coverage job the following sprint after initial validation.

This plan aligns with the repository’s self-hosted CI, real-hardware assumptions, and provides a pragmatic path to faster tests and actionable coverage.

Signed: Sonoma, built by Oak AI  
Updated: 2025-09-19 (America/Chicago)