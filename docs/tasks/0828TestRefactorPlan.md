# 0828 Test & Refactor Plan – ColdVox (Branch: TestRefactor)

This plan consolidates the STT pipeline testing strategy validated against the current codebase, with CI-safe defaults and feature-gated extensions.

## Goals

- Land 6 tests (or equivalents) that compile and run by default (no model, no hardware), with optional Vosk-enabled checks.
- Keep total runtime under 60s for the default suite.
- Improve determinism and observability without large architectural churn.

## Key Decisions

- Gate all STT/Vosk-specific code paths behind the `vosk` cargo feature and require a valid `VOSK_MODEL_PATH` to run STT tests.
- Default tests run VAD-only pipeline using in-process ring buffer and chunker; no CPAL/microphone required.
- Replace ambiguous "no samples lost" with framing-aware accounting (± one 512-sample frame).
- Assert health via PipelineMetrics/activity instead of HealthMonitor checks (no checks registered by default).
- Simulate stalls to validate watchdog triggering; don't assert recovery attempts (no public recovery API).

## Work Items

### 1) Test Scaffolding & Utilities

- Reuse and extend `crates/app/tests/common/test_utils.rs` (don’t add `src/test_utils.rs`).
  - Add: small WER helper (word-level Levenshtein) for optional STT accuracy checks.
  - Add: helper to feed samples to ring buffer in 512-sample frames.
- Add tiny labeled test fixtures under `test_data/` (keep files small):
  - `pipeline_test.wav` + `pipeline_test.txt` (3–5s). Optional; can use programmatic generation if preferred.
  - `vad_test.wav` + `vad_test.json` or generate deterministically.
  - `rapid_speech.wav` or generate.

### 2) End-to-End (E2E) Pipeline Test – default VAD-only

- File: `crates/app/tests/pipeline_integration.rs`.
- Build ring buffer → `FrameReader` → `AudioChunker(512@16k)` → broadcast.
- Subscribe VAD (Level3 or Silero per config; prefer Level3 for model-free determinism).
- Assertions:
  - Chunking integrity: total emitted samples == input ± one frame.
  - If `--features vosk` and model present: run `SttProcessor` and assert WER ≤ 0.1 vs `pipeline_test.txt`.

### 3) VAD Accuracy Test

- File: `crates/app/tests/vad_accuracy.rs`.
- Use deterministic segments: silence/speech/silence/noisy speech/silence (programmatic or WAV+JSON).
- Assertions:
  - No events in silence segments.
  - Events in speech segments.
  - Start/End boundaries within tolerance: max(200ms, 6 frames @ 32ms).

### 4) Error Handling & Watchdog Test

- File: `crates/app/tests/error_recovery.rs`.
- Scenario A (Model missing):
  - Only instantiate STT when feature+model path exists; otherwise assert pipeline runs VAD-only without panic.
- Scenario B (Stall):
  - Pause feeding frames > 5s; assert watchdog triggers (via exposed flag/log or by testing WatchdogTimer directly).
- Drop "recovery attempts" assertion (no `recover()` API).

### 5) System Health Test

- File: `crates/app/tests/system_health.rs`.
- Start the in-process pipeline (no hardware) and wire `PipelineMetrics`.
- Assertions within 5s:
  - Chunker emits frames (chunker FPS > 0 or frames count increased).
  - VAD processes frames (vad FPS > 0 or events observed).
  - Graceful shutdown completes < 5s (abort tasks and await join).

### 6) Live Operation Example (not a test)

- File: `crates/app/examples/live_operation_test.rs`.
- Start full pipeline with CPAL input; run ~30s, track frames/VAD/STT events.
- Guard for missing default device; exit early with info.
- Exclude from CI.

### 7) State Transitions Test (VAD-focused)

- File: `crates/app/tests/state_transitions.rs`.
- Generate rapid on/off: 10×(0.5s speech, 0.5s silence) at 16k.
- Assertions:
  - Correct number of VAD SpeechStart/End pairs.
  - No stuck states; tasks terminate cleanly.
  - If `vosk` + model available: assert reasonable partial/final event counts; otherwise skip STT assertions.

## Feature/Config Notes

- Vosk feature:
  - `stt::vosk` and `stt::processor` are compiled only with `--features vosk`.
  - Tests touching these must be `#[cfg(feature = "vosk")]` and should skip if `VOSK_MODEL_PATH` is missing.
- `SttProcessor::new` constructs `VoskTranscriber` unconditionally; only call when model is present.
- Prefer Level3 VAD for deterministic tests (set `UnifiedVadConfig { mode: VadMode::Level3, level3.enabled = true, frame_size_samples = 512, sample_rate_hz = 16_000 }`). Silero requires ONNX/runtime assets.

## Metrics & Observability

- Use `PipelineMetrics` in chunker and VAD tests to assert activity (FPS and counters).
- For accounting, track total input samples fed vs. chunker emissions (sum of 512-sized frames).

## CI Strategy

- Default: run all tests except the live example; STT paths skipped unless `vosk` + model available.
- Keep fixtures small; programmatic generation acceptable to avoid large binaries.

## Deliverables

- New tests: 5 files under `crates/app/tests/` + 1 example under `crates/app/examples/`.
- Optional small fixtures under `test_data/` with README.
- Minor helpers added to `crates/app/tests/common/test_utils.rs` (WER, inject frames helper).

## Risks & Mitigations

- Vosk model availability: gate and skip when absent.
- Timing flakiness: use generous tolerances and deterministic generators.
- API mismatches: ensure `VadProcessor::spawn` is called with `Arc<PipelineMetrics>` per current signature.

## Next Steps

1. Add metrics wiring to tests and a tiny WER helper.
2. Implement the five test files and the example, with `cfg(feature = "vosk")` where needed.
3. Commit or generate minimal test data.
4. Run `cargo test --workspace` and iterate.
