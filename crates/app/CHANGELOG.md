# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/Coldaine/ColdVox/releases/tag/coldvox-app-v0.1.0) - 2025-09-01

### Added

- *(text-injection)* complete Phase-3 text injection implementation with backend detection and comprehensive metrics
- *(text-injection)* implement Phase-2 text injection scaffolding with feature gates
- enhance TUI dashboard with STT integration and update documentation
- add transcription persistence and comprehensive STT testing
- add session-based text injection with async command execution
- implement centralized audio resampling in chunker
- *(pipeline)* integrate STT and refactor audio pipeline\n\n- Switch to broadcast fan-out for audio stream\n- Add SttProcessor gated by VAD SpeechStart/SpeechEnd\n- Wire Vosk transcriber; main and TUI updated to new APIs\n- Update AudioCaptureThread/Chunker/VadProcessor signatures (sample rate explicit)\n- Fix Cargo features for vosk; resolve module visibility\n- Clean up clippy warnings, unused imports, dead code; minor idioms (clamp, init)
- *(tui)* add interactive TUI dashboard (ratatui + crossterm) with real-time audio, pipeline stages, and VAD status\n\n- New binary: tui_dashboard (start/stop, metrics, logs)\n- Live level meter (dB), sparkline history, stage pulses\n- Device selection via -D, non-blocking event loop\n\nfeat(telemetry): introduce PipelineMetrics for cross-thread monitoring\n\n- Audio peak/RMS/dB, FPS, buffer fill, stage activity\n- Speaking indicators, segment counts, basic error counters\n\nrefactor(probes): move to common types, modernize mic_capture\n\n- New TestContext/TestError(Kind) and thresholds module\n- Async mic capture probe collecting FPS, drop rate, watchdog\n- Stubs for foundation and record_to_wav retained with notes\n\nfeat(vad): clarify defaults; keep Level3 disabled, prefer Silero\n\n- Config docs/comments; guard Level3 usage in VadAdapter\n- Level3 file header comment for intent\n\ndocs: update CLAUDE.md and add enhanced TUI + Vosk plans\n\n- CLAUDE.md: architecture, probes, examples paths, status\n- New docs: enhanced_tui_dashboard.md, vosk_integration_plan.md\n\nchore: add sample test-run artifacts under .coldvox/test_runs\n\n- Mic capture JSON results for quick regression checks\n\nNote: Forks/ColdVox-voice_activity_detector shows -dirty submodule state (no code changes here).
- enhance logging with persistent file storage and rotation
- implement Phase 2 ring buffer using rtrb library

### Fixed

- *(text-injection)* ensure get_method_priority never returns empty; add NoOp fallback, clap env feature, Sync bound; fix main imports
- *(app)* add async-trait dep for async TextInjector so main builds
- repair corrupted Cargo.toml and regenerate Cargo.lock
- resolve PR #4 comments - async trait implementation and import hygiene
- remove stderr logging from TUI dashboard
- implement major audio pipeline reliability improvements
- *(vad)* align UnifiedVadConfig.sample_rate_hz with 16kHz pipeline for Silero; avoid mismatched resampler and ensure correct frame/timestamp alignment in main, tui_dashboard, and vad_mic probe
- *(probes)* improve live audio testing infrastructure and fix configuration issues
- *(stt)* streamline STT implementation for production readiness
- *(vad)* resolve trait method ambiguity in Level3Vad tests
- resolve watchdog timer epoch logic error preventing timeout detection

### Other

- Complete workspace split refactoring
- ignore .coldvox artifacts and untrack prior test_runs so release-plz sees clean working tree
- Merge branch 'main' into fix/text-injection-pipeline-a
- manager and clipboard/focus refinements; update Cargo manifests; adjust tests
- add criterion benchmark for text chunking (old collect vs new iterator)
- add window_manager helpers (X11/Wayland/KDE) and initial unit tests covering focus, permissions, window info, and adaptive strategy
- robust availability check incl. binary perms and /dev/uinput access; improved error messages
- expand InjectionConfig defaults and add NoOp method; provide default type_text/paste; add NoOpInjector
- inject shared InjectionMetrics into InjectionSession; fix integration tests to pass metrics
- split ProcessorMetrics from types::InjectionMetrics; update re-exports and example usage
- *(tasks)* add end-to-end text injection test plan; set up phased tasks, CI wiring, and risk mitigations
- improve STT accuracy with buffered audio processing
- fix build by gating vosk E2E test, update examples/vad_demo to async API, and avoid awaiting borrowed handles; injection processor avoids holding locks across await
- chunker reacts to device config updates + resampler quality option wiring
- enhance STT processor with improved error handling and persistence support
- simplify float clamping in audio capture
- fix markdown lint in Vosk plan; Probes: add periodic metrics logging in vad_mic
- plumb PipelineMetrics across main and probes; buffer fill + FPS logging; docs: update to rtrb ring buffer
- add daily-rotated file logging + stderr for main app and TUI; doc constants unification to 512/16k
- Onemore
- Added test data for live audio
- *(app)* replace corrupted pipeline_integration.rs with stable chunker test; add ring buffer feeder + WER helper; align to current APIs and metrics
- plumb optional PipelineMetrics; TUI snapshot gating; tests and docs cleanup\n\n- vad_processor: spawn accepts Option<Arc<PipelineMetrics>>; update call sites\n- chunker/vad: FPS tracking, frame counters, stage marks\n- frame_reader: buffer fill reporting with capacity and optional metrics\n- TUI: add capture_frames/chunker_frames; show N/A until first snapshot\n- tests: update VAD unit; add pipeline_integration (ring buffer → chunker → VAD)\n- Cargo: add examples feature; gate example bins and vosk_test\n- docs: fix stale refs, simplify ring buffer doc to rtrb, add maintenance checklist and test plan
- add STT test planning docs and sample recording; implement Silero VAD wrapper with tests
- *(vad)* add AudioResampler unit tests (pass-through, frame aggregation, 48k→16k) and a Level3 silence integration test for VadProcessor; validate adapter resampling behavior and silence stability
- record mic_capture run on 2025-08-26 (0 FPS) for troubleshooting
- archive large design docs; replace tops with pointers to docs/archive
- re-export VadMicCheck from probes module
- update architecture diagram
- *(vad)* remove vendored VAD crate; use remote git dependency
- Replace linear resampler with Rubato and remove redundant tests
- *(probes)* consolidate test binaries into unified probe system
- add Live Test Dashboard plan and Phase docs; Implement VAD chunking notes; Add TUI dashboard placeholder bin; Fix Cargo.toml examples section; Add simple VAD test harnesses
- *(vad)* make vendored voice_activity_detector a required dep; add dev rand
- *(mic)* improve volume metering, WAV writing, and device listing UX
- *(core)* add Level3 energy VAD, Silero wrapper, state machine, adapter, and processor
- *(vad-wiring)* export vad module and vad_processor in audio and lib
- *(ring-buffer)* finalize rtrb producer/consumer split and robust chunked read/write
- *(logging)* add tracing-appender with daily file rotation and wire RUST_LOG env filter
- *(lint)* enforce tracing-only logging via .clippy.toml and remove eprintln from panic hook
- Linter Changes
- add comprehensive Phase 1 test suite with integration and unit tests
- device discovery/selection and capture pipeline tweaks
- *(app)* update Cargo.lock
- add feature-gated VAD adapter, upstream sync script, and MODIFICATIONS log; improve mic_probe clean shutdown
- Device selection: add auto-prefer hardware (HyperX/QuadCast) before OS default; Audio: add I8 input support
- add CPAL U8 input support; fix mic_probe crash on U8-only devices
- Phase 1 fixes: watchdog epoch + stop(); dynamic CPAL format + channel handling with downmix; clean stop for audio; start HealthMonitor on boot; minor stats fix
- Complete implementation of Phase 0 & Phase 1: ColdVox audio foundation
