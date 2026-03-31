---
doc_type: session
subsystem: general
status: active
freshness: current
preservation: reference
last_reviewed: 2026-03-31
owners: Patrick MacLyman
version: 1.0.0
summary: Audit of ColdVox default STT profile, test truthfulness, CI contract drift, and agentic enforcement options
---

# Session Log: Default Profile, Test Truthfulness, and CI Contract Audit (2026-03-31)

## Objective

Determine what ColdVox currently treats as the product-default STT path, whether the default build and test paths validate that product path, what CI actually enforces, and what happened during the follow-up cleanup and documentation actions.

## Scope Reviewed

- Anchor docs:
  - `docs/northstar.md`
  - `docs/plans/current-status.md`
  - `docs/dev/CI/policy.md`
  - `docs/dev/CI/architecture.md`
  - `docs/dev/commands.md`
- Runtime and config:
  - `config/default.toml`
  - `justfile`
  - `.github/workflows/ci.yml`
- Cargo/test wiring:
  - `crates/app/Cargo.toml`
  - `crates/coldvox-stt/Cargo.toml`
  - `crates/app/tests/golden_master.rs`
  - `crates/app/tests/pipeline_integration.rs`
  - `crates/app/tests/integration/*`
  - `crates/app/tests/unit/*`
- STT profile behavior:
  - `crates/coldvox-stt/src/plugins/http_remote.rs`
  - `crates/app/src/stt/plugin_manager.rs`
- External reference:
  - arXiv:2601.22832, "Just-in-Time Catching Test Generation at Meta"

## Observed Facts

### 1. The product anchor and the runtime defaults drifted apart

- `docs/northstar.md` says the project should be CUDA-first on high-end NVIDIA GPUs, with Moonshine as the baseline fallback backend.
- `docs/northstar.md` also says:
  - supported STT now: Moonshine
  - planned later: Parakeet
  - no normal "no-STT" product mode
- `config/default.toml` currently sets:
  - `stt.preferred = "http-remote"`
  - remote base URL `http://localhost:5092`
  - model `parakeet-tdt-0.6b-v2`
- `justfile` also treats `http-remote,text-injection` as the run path.

Result: the runtime defaults and developer commands drifted away from the anchor document.

### 2. The default app build does not compile in any STT backend

- `crates/app/Cargo.toml` sets:
  - `autotests = false`
  - `default = ["silero", "text-injection"]`
- `crates/coldvox-stt/Cargo.toml` sets:
  - `default = []`

Result: the default `coldvox-app` build has VAD and text injection enabled, but no STT backend compiled in.

### 3. The default runtime path still fails today

Reproduced on 2026-03-31 with:

```powershell
cargo run -p coldvox-app --bin coldvox
```

Observed runtime result:

- audio capture initialized
- VAD initialized
- plugin discovery returned zero available STT plugins
- preferred plugin `http-remote` was not found
- startup terminated with:
  - `Validation failed: stt: No STT plugin available. Ensure required models and libraries are installed.`

This is not hypothetical. The current default app path fails at runtime.

### 4. The app test suite can report green while skipping the real STT path

Reproduced on 2026-03-31 with:

```powershell
cargo test -p coldvox-app --test golden_master --features http-remote,text-injection -- --nocapture
```

Observed result:

- test status: `ok`
- printed message:
  - `Skipping golden master test: no real STT backend feature enabled (requires moonshine or parakeet).`

This means a nominally passing "golden master" test can still skip the real STT execution path.

### 5. Cargo test discovery is intentionally restricted, and the repo is relying on manual registration

- `crates/app/Cargo.toml` uses `autotests = false`
- only explicitly listed `[[test]]` targets are executed
- `cargo test -p coldvox-app -- --list` confirmed the active registered targets
- `pipeline_integration` is still registered, but currently contributes `0 tests`
- `crates/app/tests/pipeline_integration.rs` is commented out and explicitly says it is temporarily disabled

Files present in tree but not part of the default `cargo test -p coldvox-app` integration surface include:

- `crates/app/tests/integration/capture_integration_test.rs`
- `crates/app/tests/integration/text_injection_integration_test.rs`
- `crates/app/tests/integration/mock_injection_tests.rs`
- `crates/app/tests/unit/watchdog_test.rs`
- `crates/app/tests/unit/silence_detector_test.rs`
- `crates/app/tests/tui_dashboard_test.rs`

Result: files can exist under `tests/` without contributing any protection unless someone manually keeps `Cargo.toml` in sync.

### 6. The hosted CI contract is narrower than the product-default contract

Documented preference:

- `docs/dev/CI/policy.md` and `docs/dev/CI/architecture.md` split CI between:
  - GitHub-hosted for fmt, clippy, build, unit tests
  - self-hosted Fedora/Nobara for hardware-dependent tests

Actual current workflow:

- `.github/workflows/ci.yml` hosted job runs:
  - `cargo check --workspace --all-targets --locked`
  - `cargo build --workspace --locked`
  - `cargo test --workspace --locked`
- self-hosted job runs hardware integration tests

What is missing:

- no mandatory "product-default must start" check
- no mandatory "default test path must exercise a real STT backend" contract
- no guard against skip-only "green" tests
- no guard that fails CI when files under `crates/app/tests/` are unregistered because of `autotests = false`

Result: CI is spread for a reasonable hardware-isolation reason, but it does not enforce the one contract that matters here:

> default green must mean the default product path works

### 7. The local and hosted service reality is different from the app-default reality

Direct service checks on 2026-03-31:

```powershell
Invoke-RestMethod http://localhost:5092/health
Invoke-RestMethod http://localhost:8200/healthz
curl -X POST http://localhost:5092/v1/audio/transcriptions ...
curl -X POST http://localhost:8200/audio/transcriptions ...
```

Observed results:

- `5092` CPU service health: `{"status":"ok"}`
- `8200` GPU service health: `{"status":"ok"}`
- CPU transcription succeeded on `crates/app/test_data/test_11.wav`
- GPU transcription succeeded on the same WAV

Result: the remote services themselves are up and functional. The failure is in build/profile/default wiring, not in the existence of available STT services.

### 8. Moonshine and Parakeet were not removed from the codebase

What is still present:

- `moonshine` feature in `crates/app/Cargo.toml`
- `parakeet` feature in `crates/app/Cargo.toml`
- `moonshine`, `parakeet`, `http-remote`, `parakeet-cuda`, and `parakeet-tensorrt` features in `crates/coldvox-stt/Cargo.toml`

What changed historically:

- other STT backends such as Whisper, Coqui, Leopard, and Silero-STT were removed or demoted

Result: the current false-green problem is not caused by Moonshine or Parakeet being deleted. It is caused by default feature selection, runtime configuration drift, and weak CI contracts.

## What Occurred During This Session

### 1. Review findings were expanded into a deeper audit

The initial review found two concrete regressions:

- default app startup could fail due to no built-in STT fallback
- `just ci` was broken because `scripts/local_ci.sh` had been removed while callers still referenced it

That review was then widened into:

- default profile audit
- runtime validation
- test inventory audit
- CI contract audit
- GPU-vs-CPU profile clarification

### 2. The repo anchor conflict was called out explicitly

`docs/northstar.md` was treated as the top-priority anchor doc. That made the current `http-remote` CPU-default posture look like documentation and command drift rather than the intended product truth.

### 3. `windows-multi-agent-recovery.md` was deleted at user request

Action taken:

- removed `docs/plans/windows-multi-agent-recovery.md`
- committed locally as:
  - `94f94aa docs: remove windows multi-agent recovery plan`

Push status:

- attempted push to `origin/main`
- push was rejected by branch protection because changes must land through a pull request

Result: the deletion is committed locally but not pushed.

## Why the Current CI Split Exists

The split itself is reasonable:

- hosted CI is for cheap, parallel, non-hardware checks
- self-hosted CI is for live display/audio/clipboard checks that only the real machine can perform

That spread is not the bug.

The bug is that the repo never enforced a product contract on top of the spread.

The missing contract is:

1. exactly one product-default STT profile must be authoritative
2. default build/run/test must align to that profile
3. CI must fail if that profile is absent, skipped, or silently replaced

## External Reference: arXiv 2601.22832

Paper reviewed:

- https://arxiv.org/abs/2601.22832

High-level conclusion:

- the paper is useful for *additional* change-aware bug catching
- it is not an acceptable replacement for deterministic product-default CI contracts

Relevant takeaways:

- Meta generates "catching tests" that are meant to fail on buggy diffs and pass on the parent revision
- they target high-risk diffs rather than every change
- they use rule-based and LLM-based assessors to reduce human review load
- their LLM layer is a triage and ranking system, not the sole source of truth

For ColdVox, that makes this paper appropriate only as a second-layer system:

- after deterministic default-path smoke tests
- after unit/build/hardware contracts are explicit
- after skip-based false greens are removed

It is not appropriate as the first fix for the current problem.

## Recommended Contract Changes

### Required

1. Define one authoritative product-default STT profile.
2. Make `cargo run -p coldvox-app --bin coldvox` honor that profile, or stop calling it the default path.
3. Add a mandatory CI smoke test that exercises the product-default path and fails hard if no real STT backend is active.
4. Remove skip-success behavior from the so-called golden path test, or rename it so it stops overstating coverage.
5. Remove `autotests = false`, or add a CI check that fails if any `crates/app/tests/**/*.rs` file is not explicitly registered.
6. Remove or restore dead registered tests like `pipeline_integration`.

### Optional but valuable

1. Add an agentic review layer that inspects diffs for test contract drift.
2. Add risk-based candidate test generation only after the deterministic contracts above are enforced.

## Practical Agentic Enforcement Model

An LLM system is appropriate here as a policy-enforcement and triage layer, not as the ground truth for correctness.

Recommended use:

- detect when `default` Cargo features diverge from product-default runtime config
- detect when `justfile`, docs, config, and CI disagree about the canonical path
- detect when tests under `tests/` are orphaned by `autotests = false`
- detect when a "golden" or "integration" test only skips
- propose targeted smoke tests for changed STT/plugin/CI files

Not recommended:

- letting an LLM decide whether a green build means the product works without deterministic checks
- using LLM judgments as the only gate for merge blocking

## Bottom Line

ColdVox currently has a false-green failure mode:

- the default app path fails at runtime
- the nominal golden test can pass while skipping the real STT path
- CI enforces build/unit/hardware separation, but not product-default truth

The right immediate fix is to harden the deterministic contract around one real default profile.
The Meta paper is useful later as an additive, risk-focused catching layer, not as a substitute for that contract.
