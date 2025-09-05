# ColdVox – Voice AI audio pipeline

[![Status: STT Integration Enhancement](https://img.shields.io/badge/Status-STT%20Integration%20Enhancement-blue)](docs/PROJECT_STATUS.md)
[![CI](https://github.com/Coldaine/ColdVox/actions/workflows/ci.yml/badge.svg)](https://github.com/Coldaine/ColdVox/actions/workflows/ci.yml)

Rust-based real-time audio capture and processing with robust recovery, VAD, and STT integration.

## What's New (v2.0.1)

- Text Injection: FocusProvider dependency injection for reliable focus handling in tests and prod
- Mocked fallback tests and utilities for deterministic injection behavior
- Headless CI: Xvfb + fluxbox readiness checks, workflow validation via `gh`
- Clippy and doc warning cleanup; async `ydotool` availability check
- Documentation: testing guide, architecture diagram updates, and coverage analysis

See release notes: https://github.com/Coldaine/ColdVox/releases/tag/v2.0.1

## Workspace Structure

ColdVox is organized as a Cargo workspace with the following crates:

- **`crates/app/`** - Main application binaries and CLI interface
- **`crates/coldvox-foundation/`** - Core types, errors, and foundation functionality
- **`crates/coldvox-audio/`** - Audio capture, processing, and device management
- **`crates/coldvox-telemetry/`** - Metrics and performance monitoring
- **`crates/coldvox-stt/`** - Speech-to-text framework and interfaces
- **`crates/coldvox-stt-vosk/`** - Vosk STT implementation
- **`crates/coldvox-text-injection/`** - Text injection for automation
  - Testing: see `docs/testing.md` for feature modes and mocking notes

## Quick Start

### Default End-to-End Pipeline

The recommended way to run ColdVox is with the default end-to-end pipeline, which includes Silero VAD, Vosk STT, and Text Injection.

```bash
# Build and run the default pipeline
cargo run -p coldvox-app --bin coldvox --features "silero,vosk,text-injection"

# For debugging with logging
RUST_LOG=debug cargo run -p coldvox-app --bin coldvox --features "silero,vosk,text-injection"
```

### Minimal Configurations (for Development & Debugging)

You can run with a reduced feature set for specific development or debugging scenarios.

```bash
# VAD-only mode (without STT)
cargo run -p coldvox-app --bin coldvox --no-default-features --features "silero,text-injection"

# Run audio probe utilities
cargo run -p coldvox-app --bin mic_probe -- --duration 30
cargo run -p coldvox-app --bin tui_dashboard  # S=Start, A=Toggle VAD/PTT, R=Reset, Q=Quit
```

## Features

**Core (always available):**
- Reliable microphone capture with auto-recovery (watchdog)
- Device‑native capture to ring buffer (no resampling on capture thread)
- AudioChunker handles stereo→mono and resampling to 16 kHz
- Ring buffer and backpressure handling with stats
- Voice Activity Detection (Silero V5 via vendored fork)
- Optional push-to-talk mode activated by holding <kbd>Ctrl</kbd>+<kbd>Super</kbd>

**Optional features (via feature flags):**
- **`vosk`**: Speech-to-text using Vosk engine (requires system dependencies)
- **`text-injection`**: Automated text input for transcribed speech
- **`examples`**: Additional example programs and demos
- **`live-hardware-tests`**: Hardware-specific test suites

## Configuration

- CLI flags are the primary interface (see probes for examples).
  - `--activation-mode`: select `hotkey` (default) or `vad` to control how speech is detected
  - TUI defaults mirror the app and run the same runtime; no flags required for common usage.
- Environment variables:
  - `RUST_LOG`: Controls logging verbosity (info/debug)
  - `VOSK_MODEL_PATH`: Path to Vosk model files (defaults to models/vosk-model-small-en-us-0.15)

## Troubleshooting

- No audio frames: check device permissions, try a different input device using `mic_probe`.
- Watchdog triggers repeatedly: lower `--silence_threshold` or verify device sample format.
- Frame drops: ensure a consumer drains the channel; long processing on the main thread can cause backpressure.
- STT build fails: Install libvosk system library (see docs/vosk_integration_plan.md for details).
- STT disabled at runtime: Download Vosk model files from https://alphacephei.com/vosk/models

## Architecture

```mermaid
flowchart LR
    A[CPAL Input Stream] --> B[AudioRingBuffer]
    B --> C[FrameReader]
    C --> D[AudioChunker\n(resample + downmix)]
    D -->|broadcast| V[VAD]
    D -->|broadcast| S[STT]
```

Notes:
- Audio capture pushes device‑native samples (converted to i16) to a lock‑free ring buffer.
- FrameReader reconstructs timestamps from sample counts at the device rate.
- AudioChunker converts multi‑channel to mono, resamples to 16 kHz, and emits fixed 512‑sample frames.
- VAD and STT subscribe to a broadcast of chunked frames.

See `crates/app` and `docs/` for deeper architecture notes.

## License

ColdVox is licensed under your chosen license.

### Third-Party Software

This project includes vendored dependencies:

#### Vosk Speech Recognition
- **Location**: `vendor/vosk/`
- **Copyright**: 2019-2022 Alpha Cephei Inc.
- **License**: Apache License 2.0
- **Source**: https://github.com/alphacep/vosk-api

The vendored Vosk binary (`libvosk.so`) is distributed under the Apache License, Version 2.0.
See `vendor/vosk/LICENSE` for the full license text.
