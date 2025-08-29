<!-- markdownlint-disable -->

# 2025-08-29 — Small cleanups and quick wins

This note tracks low-risk cleanups and small improvements discovered today.

## Summary

-  Implement STT config hot-reload (wire existing `update_config` into the
  running pipeline).
-  Add a minimal STT metrics panel to the TUI dashboard.
-  Remove or update outdated docs (done: `docs/vosk_implementation_gaps.md`).
-  Create Criterion bench skeletons (just stubs to start measuring later).
-  Minor docs polish (README/TUI notes) and a tiny formatting fix.

## Tasks

-  [ ] STT config hot reload control path

  -  Files: `crates/app/src/stt/processor.rs`, `crates/app/src/stt/vosk.rs`
  -  Add a control channel/API on `SttProcessor` to receive a new
    `TranscriptionConfig` at runtime.
  -  Apply changes at a safe boundary (Idle or right after `SpeechEnd`) via
    `VoskTranscriber::update_config(...)`.
  -  On failure (e.g., model not found), retain the old recognizer and log an
    error; retry later with simple backoff.
  -  Optional: expose a simple trigger (watch a file path or a one-shot
    function called from main). Keep minimal.
  -  Acceptance: toggling `partial_results` or swapping `model_path` takes
    effect without restart and doesn’t panic.

-  [ ] TUI: add basic STT metrics panel

  -  File: `crates/app/src/bin/tui_dashboard.rs`
  -  Show: partial count, final count, error count, and a crude latency
    (time from `SpeechStart` to first partial/final).
  -  Use a simple shared struct similar to `PipelineMetrics` or a small local
    snapshot sent over a channel.
  -  Keep layout minimal (one extra box, no complex graphs).
  -  Acceptance: metrics update in near real-time while STT runs; panel hides
    gracefully if STT is disabled.

-  [x] Remove outdated gap doc

  -  File: `docs/vosk_implementation_gaps.md`
  -  Status: removed — several items are now implemented (unit tests,
     persistence, end-to-end WAV test).

-  [ ] Criterion benches — skeleton only

  -  Path: `crates/app/benches/`
  -  Create stubs for: VAD frame processing, STT `accept_frame` on
    silence/small speech.
  -  Don’t wire heavy assets yet; focus on compile-ready placeholders with
    `#[ignore]` or feature guards.
  -  Acceptance: `cargo bench` discovers targets (even if ignored) and
    compiles cleanly.

-  [ ] README and docs polish

  -  Files: `README.md`, `docs/enhanced_tui_dashboard.md`
  -  Note current STT status: enabled when `VOSK_MODEL_PATH` or default model
    exists. Add one-liner on enabling persistence flags.
  -  In TUI doc, mark STT metrics as “available” once implemented; otherwise
    keep “planned” consistent.
  -  Acceptance: concise, accurate instructions; no references to removed
    docs.

-  [ ] Tiny formatting fix (non-functional)

  -  File: `crates/app/src/stt/persistence.rs`
  -  Minor newline/style glitch around the `handle_vad_event` closing brace
    before the `/// Handle transcription event` doc comment.
  -  Acceptance: tidy formatting without logic changes.

## Notes

-  Hot-reload edge cases: prefer switching at utterance boundaries to avoid
  partial loss. Model loading may be slow — consider `spawn_blocking` if
  needed.
-  Benchmarks can evolve later; stubs just reserve space and wiring for
  measurable growth.
