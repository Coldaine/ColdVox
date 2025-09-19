# ColdVox PR Consolidation Follow-up: Remediation Strategy

## Executive Summary

The `merge-prs-112-113-114` branch pulled in most of the desired refactors, but several reviewer-flagged defects and architectural mismatches remain. Until these gaps are closed, the consolidation is **graded C**: it compiles in some feature sets, but diverges from the documented design and leaves CI red. This follow-up captures the remediation backlog based on source review, bot feedback, and repo inspection.

## Priority Remediation Items

### 1. STT Pipeline Must Consume `SharedAudioFrame` (Priority: High)
- **Issue**: `crates/coldvox-stt/src/processor.rs` still defines its own `AudioFrame { Vec<i16> }`, buffering copies even though the upstream audio path now broadcasts `SharedAudioFrame` (`Arc<[i16]>`). This regresses the zero-copy goal from PR #114 and contradicts the merge strategy.
- **Plan**:
  - Replace the local `AudioFrame` type with `SharedAudioFrame` (`pub use`d from `coldvox_audio`), or introduce a thin wrapper that borrows the shared slice instead of owning `Vec<i16>`.
  - Update `buffer_audio_frame_if_speech_active` to pass `&frame.samples` directly into `AudioBufferManager::add_frame` so we do not allocate new vectors.
  - Adjust tests/fakes to construct `SharedAudioFrame` (e.g. by wrapping `Arc::from(Vec<i16>)`).
- **Validation**: Ensure `cargo nextest run -p coldvox-stt --features vosk` passes and memory profile shows no new allocations on the hot path.

### 2. Consolidate STT Constants in `constants.rs` (Priority: Medium)
- **Issue**: `SAMPLE_RATE_HZ`, frame duration, and send timeout constants are duplicated between `constants.rs` and `processor.rs`.
- **Plan**:
  - Move any missing values (e.g. `SEND_TIMEOUT_SECONDS`, `DEFAULT_BUFFER_DURATION_SECONDS`, `DEFAULT_CHUNK_SIZE_SAMPLES`) into `crates/coldvox-stt/src/constants.rs`.
  - Import them via `use crate::constants::{...};` in `processor.rs` and `helpers.rs` to guarantee a single source of truth.
  - Follow up by regenerating docs/tests that reference the constants.

### 3. Event Emission Path Consistency (Priority: Medium)
- **Issue**: `SttProcessor::handle_speech_end` streams chunks directly and calls `self.emitter.emit`, while legacy helpers (`send_event`, `handle_finalization_result`) remain but are unused. The emitter still holds a `RwLock` write guard across `await`, which a review comment flagged as a P1 deadlock risk.
- **Plan**:
  - Either delete `send_event` / `handle_finalization_result` or route `handle_speech_end` through them so there is exactly one path.
  - Scope the metrics write lock in `EventEmitter::emit` so the guard is dropped before awaiting `tokio::time::timeout`; capture stats into locals or wrap in a block.
  - Verify no fields become unused (we still need `event_tx` for the emitter and `stt_metrics` for metrics).

### 4. Wire `AudioBufferManager::process_chunks` (Priority: Medium)
- **Issue**: `process_chunks` already batches chunk iteration but `handle_speech_end` reimplements the loop. Tests also collect chunks into temporary vectors, undercutting the helper design.
- **Plan**:
  - Call `mgr.process_chunks(|chunk| async { self.stt_engine.on_speech_frame(chunk).await })` and collect emitted events as needed.
  - Update the helper test to iterate directly over `mgr.buffer.chunks(...)` without `collect()` as the reviewer suggested.

### 5. Text-Injection Follow-ups from PR #112 (Priority: Medium)
Incorporate all reviewer comments before the merge freezes:
- Change `plan_backends` signature to accept `&dyn BackendDetector` to avoid bare-trait warnings (`crates/coldvox-text-injection/src/backend_plan.rs:32`).
- Extract the duplicated success-rate sorting into a helper (`sort_plan_by_success`) used by both the manager cache and test helper (`.../manager.rs:300` and cached order path).
- Fix keystroke throttling math (`keystroke_delay * burst.len() as u32` truncates): prefer `keystroke_delay.mul_f64(burst.len() as f64)`.
- Only append `InjectionMethod::NoOp` when not already present to keep dedup logic simple.

### 6. Audio Capture Regression from PR #113 (Priority: High)
- Restore the missing semicolon in the `thread_local!` macro inside `crates/coldvox-audio/src/capture.rs:511`. Without it, the workspace does not compile; this must be fixed before any CI run will go green.

### 7. STT Review Corrections from PR #114 (Priority: High)
Address reviewer findings already visible in the merged branch:
- Correct the default `buffer_size_ms` to a real millisecond value (`FRAME_SIZE_SAMPLES` was inserted by mistake).
- Reuse the imported `SAMPLE_RATE_HZ` constant in `AudioBufferManager::log_processing_info` instead of qualifying the module.
- Remove the invalid `Ordering` import (`std::cmp::Ordering`) from `crates/app/src/runtime.rs` and ensure the module builds.
- Mark `parakeet-onnx` as `optional` in `crates/coldvox-stt/Cargo.toml` before referencing it in a feature (`dep:parakeet-onnx`).
- Limit the metrics write guard scope in `EventEmitter::emit` to avoid holding it across the awaited send (matches the reviewer’s P1).

## Additional Clean-up Tasks

- Audit `AudioBufferManager` for unused fields (`started_at`). If telemetry does not read it, drop the field; otherwise expose a helper (e.g. `pub fn buffered_duration(&self) -> Duration`).
- Replace `use crate::helpers::*;` glob import in `processor.rs` with explicit items to follow reviewer guidance.
- Confirm `gh api repos/.../pulls/<n>/comments --paginate` yields no additional unresolved discussions before sign-off.
- Run `cargo fmt`, `cargo clippy --all-targets --locked`, and `cargo nextest run --workspace --locked` under both default and `--features vosk` to validate fixes.

## Quality Gates & Tooling

### Pre-commit Hooks
The repository already contains `.pre-commit-config.yaml`, but installation/enforcement is ad-hoc. Adopt one (or more) of the following approaches to ensure hooks run before commits:
1. **Developer Onboarding Script (recommended)** – Ship `scripts/dev-setup.sh` that verifies required tools, installs `pre-commit`, and installs hooks (`pre-commit install --install-hooks`, plus commit-msg/pre-push). Document this in `README.md` and `docs/DEVELOPMENT.md`.
2. **Justfile Target** – Add a `dev-setup` recipe invoking `pre-commit install --install-hooks` for teams already using `just`.
3. **Git Template** – Provide `scripts/setup-git-hooks.sh` that sets `init.templatedir` with a wrapper hook which installs `pre-commit` on first run. Useful for company-wide bootstrap.
4. **CI Enforcement** – Add a `validate-pre-commit` job to `.github/workflows/ci.yml` that runs `pre-commit run --all-files`. Reject commits containing `skip-hooks` markers.

Pick the combination that suits team workflow; at minimum, the onboarding script plus CI enforcement closes the gap.

### Suggested Workflow Enhancements
- Document a `cargo safe-commit` alias (wrapper that runs hooks before committing) to encourage consistent usage.
- Add a `just lint` target bundling `fmt`, `clippy`, `nextest`, and `pre-commit run --all-files` so contributors know the expected green path.
- Update `docs/TESTING.md` to mention the STT feature matrix (`vosk`, `parakeet`, `whisper`, etc.) and how to run `cargo check` with matching features.

## Verification Checklist

Before merging the remediation branch back to `main`, ensure:
- [ ] STT code compiles without warnings on stable and MSRV (1.75) with and without the `vosk` feature.
- [ ] `cargo nextest run --workspace --locked` and the text-injection integration tests (real backend feature) pass.
- [ ] CI coverage job (`cargo tarpaulin`) succeeds for core crates.
- [ ] All reviewer comments from PRs #112–#114 are marked resolved in GitHub.
- [ ] Pre-commit hooks run automatically for at least one fresh clone (documented proof in README or onboarding guide).

## References
- Review feedback harvested via `gh api repos/Coldaine/ColdVox/pulls/<PR>/comments --paginate`.
- Repository state inspected on branch `merge-prs-112-113-114` as of 2025-09-19 consolidation.
- Original consolidation plan (`docs/merge-strategy.md`) and its stated architectural goals.

