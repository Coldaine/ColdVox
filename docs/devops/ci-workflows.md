# CI Workflows in ColdVox

**Last Updated:** September 16, 2025  

## Overview

CI is handled via GitHub Actions in `.github/workflows/`. All workflows run exclusively on self-hosted runners (`runs-on: [self-hosted, Linux, X64, fedora, nobara]`), triggered by push/PR/workflow_dispatch/schedule. No cloud fallbacks, so runner availability is critical. Local mirror: `scripts/local_ci.sh` (fmt/clippy/check/build/test) and `just ci`.

### Workflows Summary

1. **ci.yml** (Main Pipeline):  
   - **Triggers/Env/Permissions/Concurrency:** Push to main/release/feature/fix branches; PRs to main; manual/scheduled (daily). `RUSTFLAGS="-D warnings"`, Vosk model basename, disk/load thresholds. Read contents/actions; write security-events. Cancels in-progress for same ref.  
   - **Jobs:**  
     - `validate-workflows`: Optional; checks syntax with `gh` (skips if missing, validates all .yml).  
     - `setup-vosk-dependencies`: Outputs paths via `setup-vosk-cache.sh` (persistent cache `/home/coldaine/ActionRunnerCache/vosk/`, SHA256 verify, symlink to vendor).  
     - `build_and_check`: Needs Vosk; matrix `rust-version: [stable, "1.75"]`. Steps: Checkout, Rust toolchain (components: rustfmt/clippy), setup-coldvox, fmt/clippy/check (stable only), build, docs (stable), tests (stable with Vosk env; unit/integration, skips E2E), Qt6 GUI (detect-qt6.sh, build if present), artifacts on failure.  
     - `text_injection_tests` (30min): Needs Vosk; headless env (DISPLAY=:99, start-headless.sh with Xvfb/fluxbox/D-Bus). Steps: Checkout, Rust stable + cache (Swatinem/rust-cache@v2), setup-coldvox, validate backends (xdotool etc.), real-injection tests (cargo test -p coldvox-text-injection --features real-injection-tests --nocapture --test-threads=1 --timeout 600 in dbus-session), build variants (default/no-default/regex), app build, E2E pipeline (cargo test -p coldvox-app test_end_to_end_wav_pipeline --nocapture), cleanup (kill processes).  
     - `ci_success`: Needs all; generates report.md (job results), uploads artifact; fails on criticals.  
   - **Strengths:** MSRV matrix; caching in text_injection; real E2E (Vosk WAV, injection headless); nextest in Vosk; per-test timeouts (10s unit, 30s integration).  
   - **Weaknesses:** No cache in build_and_check (add Swatinem); no nightly (add to matrix); no coverage (add tarpaulin >80%).  

2. **release.yml** (Automation):  
   - **Triggers/Jobs:** PR close (merged to main); manual. Checkout (depth 0), Rust stable, cache (`Swatinem/rust-cache@v2`), install `release-plz`, run `release-plz release-pr`. Auto on main merge: Checkout (ref main, depth 0), Rust stable, install `release-plz`, create GH release (`release-plz release`).  
   - **Strengths:** Caching; release-plz for PRs/tags.  
   - **Weaknesses:** No publish (add crates.io if needed).  

3. **runner-diagnostic.yml** (Manual Diagnostics):  
   - **Triggers:** Manual.  
   - **Jobs:** `diagnose`: Prints env (user/PATH/OS), network (DNS/HTTPS), sudo/pkgs (install `xdotool`).  
   - **Strengths:** Proactive health checks.  
   - **Weaknesses:** Manual; basic (no GPU/disk).  

4. **runner-test.yml** (Runner Validation):  
   - **Triggers:** Manual; push to `fedora-runner-test` (workflow changes).  
   - **Jobs:** `test-runner` (5min): Echo basics, diagnostics (OS/CPU/memory/Rust/Git), Rust setup, Cargo.lock test (clone/check).  
   - **Strengths:** Validates toolchain/lockfile.  
   - **Weaknesses:** Narrow (no build/hardware).  

5. **vosk-integration.yml** (STT Tests):  
   - **Triggers:** PRs to STT crates/examples/workflow; weekly; manual.  
   - **Jobs:**  
     - `setup-vosk-dependencies`: Checkout, `setup-vosk-cache.sh` (local persistent cache: download/verify/extract to `/home/coldaine/ActionRunnerCache/vosk/`, symlink).  
     - `vosk-tests` (30min): Needs setup; checkout (depth 0), setup-coldvox, install nextest, Vosk build (-p coldvox-stt-vosk --features vosk), tests (cargo nextest run -p coldvox-stt-vosk), E2E WAV (cargo test -p coldvox-app test_end_to_end_wav_pipeline --ignored --nocapture), examples (cargo run --example vosk_*), artifacts on failure.  
   - **Strengths:** Path-filtered; local Vosk cache (no re-download); nextest speed; real E2E (WAV, examples).  
   - **Weaknesses:** No Rust cache (add Swatinem); reduce timeout to 15min.  

## Strengths
- **Modularity:** Separate workflows; path filters; matrix in main CI.  
- **Reproducibility:** Local `local_ci.sh` (fmt/clippy/check/build/test); persistent Vosk cache.  
- **Niche Optimization:** Vosk script (SHA256, symlink); real E2E (Vosk WAV, injection headless); nextest.  

## Weaknesses and Improvements
- **Caching:** Selective (Rust in release/text_injection; Vosk local). Add universal: `Swatinem/rust-cache@v2` in build_and_check/vosk-tests.  
- **Coverage:** MSRV matrix good; add nightly (`rust-version: [stable, 1.75, nightly]`); tarpaulin for reports.  
- **Efficiency:** Conditionals in Vosk (if features); parallel jobs; reduce timeouts.  
- **Enforcement:** Pre-commit in CI (`pre-commit/action@v5`).  

## Usage
- Trigger manual: GitHub UI > Actions > Workflow > Run.  
- Local: `just ci` or `./scripts/local_ci.sh`.  
- Monitor: GitHub Actions tab; add badges to README.md.