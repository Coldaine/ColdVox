---
doc_type: standard
subsystem: general
version: 1.0.0
status: draft
owners: Documentation Working Group
last_reviewed: 2025-10-22
---

# Assistant Interaction Guide

This repository aligns with the organization-wide Master Documentation Playbook v1.0.0. Before taking action, assistants must:

- Review [`docs/standards.md`](./standards.md) and follow placement/metadata rules.
- Consult [`docs/MasterDocumentationPlaybook.md`](./MasterDocumentationPlaybook.md) for canonical structure expectations.
- Record every actionable task exclusively in [`docs/todo.md`](./todo.md) and link to supporting specs in `docs/tasks/`.

## ColdVox Overview

ColdVox is a Rust-based voice AI pipeline that captures audio, detects speech activity, transcribes to text, and injects the text into the focused application. The default configuration uses a VAD-gated STT flow with multiple text-injection strategies for Linux desktops.

**Future Vision (Experimental)** – See [`docs/architecture.md`](./architecture.md#coldvox-future-vision) for the always-on intelligent listening plan, decoupled threading model, and tiered STT memory strategy.

## Workspace Structure

ColdVox is a multi-crate Cargo workspace:

- `crates/app/` – Main application crate (`coldvox-app`)
  - **Audio glue**: `src/audio/vad_adapter.rs`, `src/audio/vad_processor.rs`
  - **STT integration**: `src/stt/processor.rs`, `src/stt/vosk.rs`, `src/stt/persistence.rs`
  - **Text injection**: `src/text_injection/` integration layer
  - **Hotkey system**: `src/hotkey/` global hotkey support with KDE KGlobalAccel
  - **Binaries**: `src/main.rs`, `src/bin/tui_dashboard.rs`, `src/bin/mic_probe.rs`
- `crates/coldvox-foundation/` – Core scaffolding & shared types
  - `state.rs`: `AppState` + `StateManager`
  - `shutdown.rs`: graceful shutdown + panic hook
  - `health.rs`: `HealthMonitor`
  - `error.rs`: `AppError`, `AudioError`, `AudioConfig`
- `crates/coldvox-audio/` – Audio capture & processing
  - `device.rs`: CPAL host/device discovery with PipeWire-aware priorities
  - `capture.rs`: `AudioCaptureThread::spawn`
  - `ring_buffer.rs`: `AudioRingBuffer`
  - `frame_reader.rs`: `FrameReader`
  - `chunker.rs`: `AudioChunker`
  - `resampler.rs`: `StreamResampler`
  - `watchdog.rs`: watchdog reset loop
  - `detector.rs`: RMS-based `SilenceDetector`
- `crates/coldvox-vad/` – VAD traits & configs (`VadEngine`, `UnifiedVadConfig`)
- `crates/coldvox-vad-silero/` – Silero V5 ONNX implementation (`SileroEngine`)
- `crates/coldvox-stt/` – STT abstractions (`TranscriptionEvent`, processors)
- `crates/coldvox-stt-vosk/` – Vosk STT integration (`VoskTranscriber`)
- `crates/coldvox-text-injection/` – Text injection backends
  - Linux backends: `atspi_injector.rs`, `clipboard_injector.rs`, `ydotool_injector.rs`, `kdotool_injector.rs`
  - Cross-platform: `enigo_injector.rs`
  - Fallback orchestration: `combo_clip_ydotool.rs`, `manager.rs`, `session.rs`
- `crates/coldvox-telemetry/` – Metrics (`PipelineMetrics`, `FpsTracker`)
- `crates/coldvox-gui/` – GUI components & bridge integration

## Development Commands

**Working Directory** – Run commands from the workspace root.

### Building

```bash
# Main app with default features (Silero VAD + text injection, no STT by default)
cargo build

# With Vosk STT
cargo build --features vosk

# Full feature set
cargo build --features vosk,text-injection

# Workspace build (all crates)
cargo build --workspace

# Release build with Vosk + injection
cargo build --release --features vosk,text-injection
```

### Running

```bash
# Main application (default features)
cargo run

# Specify input device
cargo run -- --device "USB Microphone"

# Enable Vosk STT + injection
cargo run --features vosk,text-injection

# With explicit device and STT
cargo run --features vosk,text-injection -- --device "USB Microphone"

# TUI Dashboard (S=Start, A=Toggle VAD/PTT, R=Reset, Q=Quit)
cargo run --bin tui_dashboard
cargo run --bin tui_dashboard -- --device "USB Microphone" --log-level "info,stt=debug,coldvox_audio=debug"

# Mic probe utility
cargo run --bin mic_probe -- --duration 30

# Examples (enable required features)
cargo run --example foundation_probe
cargo run --example record_10s
cargo run --example vosk_test --features vosk,examples
cargo run --example inject_demo --features text-injection
cargo run --example test_silero_wav --features examples
```

### Testing

```bash
# All tests
cargo test

# Verbose output
cargo test -- --nocapture

# Specific package
cargo test -p coldvox-app

# Integration tests
cargo test integration

# End-to-end WAV test (requires Vosk model discovery)
cargo test -p coldvox-app --features vosk test_end_to_end_wav --nocapture
```

### Linting & Formatting

```bash
cargo check --all-targets
cargo fmt -- --check
cargo clippy -- -D warnings
```

## Feature Flags

Default features: `silero`, `vosk`, `text-injection`.

- `vosk` – Offline STT integration (libvosk required)
- `text-injection` – Platform-aware injection backends
- `silero` – Silero V5 ONNX-based VAD (default)
- `examples` – Example binaries dependencies
- `live-hardware-tests` – Enables hardware-specific test suites

## Additional Pointers

- Reference crate-specific docs through `docs/reference/crates/*.md` thin indexes.
- Retention policies and task linkage expectations are enforced via the docs CI (`.github/workflows/docs-ci.yml`).
- For assistant coordination updates, mirror changes here into `CLAUDE.md`.
