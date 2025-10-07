# ColdVox Refactor – Debugging and Completion Plan

Owner: Coldaine
Date: 2025-09-22
Branch: anchor/oct-06-2025

## Goals
- Get the workspace to compile cleanly across key feature combinations.
- Exercise core binaries and examples to surface runtime regressions.
- Validate audio → VAD → STT → text injection pipeline invariants.
- Tighten logging/telemetry to accelerate future debugging.
- Produce a repeatable playbook for troubleshooting.

## Work Phases

1) Build Matrix + Feature Audit
- Enumerate all crate features and cross-crate gates.
- Run cargo checks for default and critical feature sets.
- Capture compile errors/warnings and open issues.

2) Runtime Smoke Tests
- Run `mic_probe`, `tui_dashboard`, and main app (with/without Vosk).
- Verify logging outputs and model detection behavior.
- Record quick notes and logs under `logs/`.

3) Subsystem Validations
- Audio pipeline: device selection, resample, chunker (512 @ 16k), ring buffer, watchdog.
- VAD (Silero): thresholds, windowing, debounce; ONNX loading.
- STT (Vosk): activation modes, partial/final events, persistence toggles.
- Text injection: backend selection per desktop/session, fallbacks.
- Hotkeys: KDE KGlobalAccel vs fallback workflow.

4) Examples as Probes
- Run foundation, recording, text injection, hotkey, Silero wav, Vosk test.
- Compare observed behavior to docs (`docs/`, `examples/`).

5) Telemetry & Observability
- Confirm `FpsTracker` and `PipelineMetrics` present and useful.
- Verify log rotation and `RUST_LOG` handling.

6) Issue Triage and Fix Passes
- Prioritize: build blockers → runtime crashes → correctness → UX.
- Track each defect with concise repro + patch.

## Acceptance Criteria
- cargo check passes for: default, `--features vosk`, `--no-default-features --features silero`, `--features text-injection,examples`.
- `mic_probe`, `tui_dashboard` start and produce expected logs.
- Vosk example produces transcripts with a valid model path.
- Silero wav example emits VAD events with 512-sample windows.
- Text injection demo injects text on supported desktop sessions.
- No critical warnings; deny config satisfied where applicable.

## Notes / Open Questions
- Confirm system `libvosk` availability on CI and local.
- Validate platform auto-detection in `crates/app/build.rs` for desktop backends.
- Ensure examples are wired via workspace `examples/Cargo.toml`.
