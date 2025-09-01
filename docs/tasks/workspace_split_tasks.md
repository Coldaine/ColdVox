# ColdVox workspace split: phased task plan

This document turns the crate-split proposal into concrete, trackable tasks with phases, checklists, and acceptance criteria.

## Goals
- Isolate heavy/optional deps (Vosk, ONNX) behind feature-gated crates
- Improve incremental compile times and reuse of stable components
- Clarify boundaries via thin, testable public APIs
- Keep `cargo run` usable by default (VAD-only, no STT requirement)

## Non-goals
- Public publishing to crates.io (can be a follow-up)
- Big behavior changes; this is a surgical extraction

## Target workspace layout
- crates/coldvox-telemetry: metrics types
- crates/coldvox-foundation: app scaffolding (state, shutdown, health, errors/config)
- crates/coldvox-audio: device/capture/ring buffer/chunker/watchdog/silence detector
- crates/coldvox-vad: VAD config/types/state machine/events (no ONNX)
- crates/coldvox-vad-silero: Silero ONNX wrapper (feature = `silero`)
- crates/coldvox-stt: STT processor/traits (no Vosk)
- crates/coldvox-stt-vosk: Vosk transcriber (feature = `vosk`)
- crates/coldvox-text-injection: text injection session/processor (fast-tracked)
- crates/coldvox-gui (stub): future GUI binary crate (optional; see GUI phase)
- crates/app: thin orchestrator/binaries (main, TUI, probes)

Feature passthrough: `vosk`, `silero` (default on if desired), `level3` (energy VAD optional).

---

## Phase 0 – Prep and safety rails

Why: Improve DX immediately and reduce churn during the split.

Tasks
- [ ] Make `vosk` an optional dependency in `crates/app` and wire `features.vosk = ["dep:vosk"]`
- [ ] Remove `required-features = ["vosk"]` from the `coldvox` bin so `cargo run` works VAD-only
- [ ] Guard all STT code paths with `#[cfg(feature = "vosk")]`
- [ ] Update README/docs run instructions to reflect VAD-only default

Acceptance criteria
- [ ] `cargo run` builds and runs without libvosk installed
- [ ] `cargo run --features vosk` enables STT paths

Risks
- Incomplete cfg gates; Mitigation: compile both with and without `--features vosk` in CI.

---

## Phase 1 – Extract telemetry (small, low-risk)

Tasks
- [ ] Create `crates/coldvox-telemetry` with `PipelineMetrics` and related types
- [ ] Move telemetry code from `crates/app/src` to the new crate
- [ ] Update imports; add dependency in `crates/app/Cargo.toml`
- [ ] Unit tests compile and pass

Acceptance criteria
- [ ] App builds; metrics increment as before (smoke via logs)

---

## Phase 2 – Fast-track text injection extraction (large subsystem)

Why now: Text injection is already substantial and pulls desktop-/platform-specific dependencies (atspi, wl-clipboard-rs, enigo, kdotool, etc.) unrelated to audio/VAD/STT. Isolating it early prevents feature leakage and keeps the main app's dependency graph lean.

Tasks
- [ ] Create `crates/coldvox-text-injection` as a library crate
- [ ] Move `text_injection/{session.rs,processor.rs}` and related configs
- [ ] Introduce backend features: `atspi`, `wl_clipboard`, `enigo`, `xdg_kdotool` (names tentative); make all optional by default
- [ ] Define a stable trait boundary (e.g., `TextInjector`, `TextInjectionSession`) and rework call sites to depend on the trait
- [ ] Update TUI/examples to compile without any text-injection features enabled; wire optional usage behind `#[cfg(feature = "text-injection")]`
- [ ] Document backend support matrix and env/Wayland requirements

Acceptance criteria
- [ ] `cargo build` succeeds with no text-injection features enabled
- [ ] Enabling a backend feature compiles on supported DE/WM; when unsupported, the crate cleanly disables with helpful messages
- [ ] No new deps appear in the default `cargo run` path

Risks
- Backend-specific runtime quirks; Mitigation: keep each backend behind separate feature flags and guard with runtime checks/logging.

---

## Phase 3 – Extract foundation (state/shutdown/health/errors)

Tasks
- [ ] Create `crates/coldvox-foundation` (deps: tracing, thiserror, anyhow optional)
- [ ] Move `foundation/{state,shutdown,health,error}.rs` into lib
- [ ] Define a minimal public API for `AppState`, `StateManager`, `ShutdownHandler`, `HealthMonitor`, `AppError`, `AudioError`, `AudioConfig`
- [ ] Update `crates/app` to depend on `coldvox-foundation`
- [ ] Run the foundation probe example to sanity-check

Acceptance criteria
- [ ] App and probes build; shutdown and state transitions behave as before

Risks
- Type relocation ripples; Mitigation: re-export via `pub use` temporarily in app if needed during transition.

---

## Phase 4 – Extract audio

Tasks
- [ ] Create `crates/coldvox-audio` (deps: cpal, rtrb, dasp, rubato, parking_lot)
- [ ] Move `audio/{device,capture,ring_buffer,watchdog,detector,chunker}.rs`
- [ ] Public API: `DeviceManager`, `AudioCaptureThread::spawn`, `FrameReader`, `AudioChunker` and `ChunkerConfig`, `Watchdog`; frame contract: 512 samples @ 16kHz
- [ ] Depend on `coldvox-foundation` for errors/config; on `coldvox-telemetry` for metrics
- [ ] Update app wiring; run `mic_probe` and existing audio tests

Acceptance criteria
- [ ] `mic_probe` runs; logs show watchdog feed and 512-sample chunking
- [ ] Backpressure behavior unchanged (drops when ring full)

Risks
- CPAL format negotiation; Mitigation: preserve existing device selection code; add a smoke test using the bundled test wavs if present

---

## Phase 5 – Extract VAD (core + silero)

Tasks
- [ ] Create `crates/coldvox-vad` (no ONNX deps)
- [ ] Define `VadEngine` trait, `VadEvent`, `UnifiedVadConfig` (frames: 512 @ 16kHz)
- [ ] Move VAD state machine and config into this crate
- [ ] Create `crates/coldvox-vad-silero` (deps behind `silero` feature) implementing `VadEngine`
- [ ] Replace Git dep `voice_activity_detector` with local `coldvox-vad-silero` path dep
- [ ] Optionally add `level3` energy VAD behind feature
- [ ] Update app and examples; run VAD tests/examples

Acceptance criteria
- [ ] VAD examples/tests pass; speech start/end events mirror current behavior
- [ ] ONNX runtime only compiles when `--features silero` is set

Risks
- ONNX runtime loading issues; Mitigation: support dynamic runtime via feature, keep current runtime binaries under `runtimes/` if needed

---

## Phase 6 – Extract STT (core + vosk)

Tasks
- [ ] Create `crates/coldvox-stt` with `Transcriber` trait, `TranscriptionEvent`, `TranscriptionConfig`, processor gated by VAD events
- [ ] Create `crates/coldvox-stt-vosk` with the Vosk implementation (feature = `vosk`)
- [ ] Ensure model path default (env `VOSK_MODEL_PATH` or `models/vosk-model-small-en-us-0.15`)
- [ ] Update app/TUI wiring; guard with `#[cfg(feature = "vosk")]`
- [ ] Run `vosk_test` example with and without feature

Acceptance criteria
- [ ] App builds and runs without Vosk; STT paths active only when `--features vosk`

Risks
- System lib presence; Mitigation: docs note and CI job that skips STT by default

---

## Phase 7 – GUI stub (optional, future-facing)

Why now: Create a minimal GUI crate skeleton to decouple GUI dependencies and give it a place to grow without affecting app core. Keep it OFF by default and buildable trivially.

Tasks
- [ ] Create `crates/coldvox-gui` (binary crate) with a minimal `main.rs` that prints version and exits
- [ ] No GUI toolkit dependency yet (placeholder). Optionally add a feature-gated dependency placeholder (e.g., `egui` or `gtk`) but keep disabled by default
- [ ] Wire workspace member, add a `[[bin]]` name `coldvox-gui`
- [ ] Add a short README stating goals and future toolkit evaluation criteria

Acceptance criteria
- [ ] `cargo run -p coldvox-gui` prints a stub message without pulling extra deps into the default app build
- [ ] No changes to `crates/app` runtime behavior

Risks
- Premature dependency lock-in; Mitigation: avoid selecting a GUI toolkit until requirements are clearer; keep the crate dependency-free for now.

---

## Phase 8 – TUI separation (optional)

Tasks
- [ ] Option A: keep binaries in `crates/app`
- [ ] Option B: move TUI to `crates/coldvox-tui` and depend on split crates

Acceptance criteria
- [ ] Same user-facing commands continue to work (documented in README)

---

## Phase 9 – CI matrix and caching

Tasks
- [ ] Add workflow to build/test default features on Linux
- [ ] Add a matrix job for feature combos: `{silero, level3} x {vosk on/off}` minimal coverage
- [ ] Cache target per-feature if build times regress notably

Acceptance criteria
- [ ] CI green across chosen matrix; default job runs fast

---

## Phase 10 – Docs and runbooks

Tasks
- [ ] Update README: workspace layout, quickstart (VAD-only), feature flags
- [ ] Add `crates/*/README.md` with crate purpose and API sketch
- [ ] Update docs under `docs/` for tuning knobs and new crate paths

Acceptance criteria
- [ ] A newcomer can build/run VAD-only and enable STT via a documented flag

---

## Contracts and APIs (sketch)

- Audio frames: 512-sample i16 at 16kHz. Prefer `&[i16]` or `Arc<[i16; 512]>` across crate boundaries
- VAD: `VadEngine::process(frame) -> Result<VadEventOrProb, Error>`; `VadEvent::{SpeechStart, SpeechEnd}`
- STT: `Transcriber::feed(frame)`; emits `TranscriptionEvent::{Partial, Final, Error}` via channel
- Errors: central `AppError/AudioError` in foundation; re-export as needed

Edge cases
- No device / format mismatch
- Ring buffer full (drop-on-full behavior)
- Watchdog inactivity (>5s) triggers recovery
- Silero window misalignment: reject non-512 frames with a clear error
- Vosk model path missing: STT disabled with a warning

---

## Rollout and verification checklist

- [ ] Build + clippy + tests pass after each phase
- [ ] VAD-only run tested locally
- [ ] STT run tested with model present
- [ ] TUI dashboard smoke: logs update, status shows last transcript when STT enabled
- [ ] Log file rotation still works (appender wiring)

---

## Next actions (Do this week)

1) Phase 0: fix `vosk` optional gating and remove `required-features` from `coldvox` bin
2) Phase 1: extract `coldvox-telemetry` (fast win), wire into app
3) Phase 2: extract `coldvox-text-injection` (fast-tracked), scaffold backend features; wire to app/TUI behind features
4) Phase 3: extract `coldvox-foundation`, wire probes
5) Re-assess and proceed with audio extraction

Optional commands (fish)
```fish
# VAD-only
cargo run

# With STT (requires libvosk + model)
cargo run --features vosk

# Run examples
cargo run --example vad_demo
cargo run --example vosk_test --features vosk
```
