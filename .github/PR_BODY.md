Title: Refactor: Unified audio→VAD→STT→injection pipeline, stable WAV E2E, and Wayland-first injection ordering

Summary
- This PR stabilizes ColdVox end-to-end: CPAL capture → FrameReader/Chunker → Silero VAD → Vosk STT → text injection. We replaced synthetic audio with a real WAV stream for tests, unified VAD spawning, fixed capture lifecycle on PipeWire, and reworked injection strategy to prefer AT‑SPI first on Wayland with fast fallback to Clipboard.

Key Changes
- app
  - Add `audio/wav_file_loader.rs` for real WAV frame streaming (16 kHz, 512-sample windows) with trailing silence to flush VAD
  - Refactor `runtime.rs` glue for deterministic hooks and unified VAD/STT wiring
  - Rewrite E2E WAV test to assert true pipeline behavior (Vosk+Silero, no mocks) and validate text injection
- audio
  - `capture.rs`: initialize device monitor as running=true, restoring capture FPS under PipeWire
- vad/vad-silero
  - Debounce/timing cleanups; consistent windowing; improved logging
- stt/stt-vosk
  - Helpers/constants and finalize handling; reliable final transcript emission
- text-injection
  - StrategyManager prefers AT‑SPI first on Wayland, then Clipboard; Combo Clipboard+ydotool gated via config
  - Cooldown and per-method success tracking; environment-first ordering logic
- telemetry
  - Minor additions for STT metrics exposure
- docs
  - Expanded refactoring/integration plan; architecture and test guidance

Behavioral Notes
- On Wayland, AT‑SPI is tried first; if DBus methods are unavailable, failure is fast and remembered via cooldown; Clipboard succeeds next
- ydotool-backed paste is disabled by default unless explicitly allowed via config
- VOSK model must exist; resolved via VOSK_MODEL_PATH or default models dir

Risk/Impact
- No public API breaks; runtime behavior improved. Injection ordering changed on Wayland (AT‑SPI > Clipboard). ydotool path gated.

Test Plan
- Workspace check/build and `coldvox-app` E2E WAV test
- Logs confirm: SpeechStart/End, Vosk partials/final (“one with the worst record”), and Clipboard injection success after AT‑SPI fast-fail

How to Run (examples)
- App: cargo run --features vosk
- TUI: cargo run --bin tui_dashboard
- E2E WAV test: cargo test -p coldvox-app test_end_to_end_wav_pipeline -- --nocapture

Follow-ups
- Optional CLI/config to toggle injection preferences and enable ydotool path
- Live mic validation/tuning for VAD thresholds and longer windows

Screenshots/Logs
- See test output and logs in `logs/coldvox.log` for detailed pipeline traces.
