# ColdVox – Voice AI audio pipeline

[![Status: Phase 3 Complete](https://img.shields.io/badge/Status-Phase%203%20Complete-brightgreen)](docs/PROJECT_STATUS.md)
[![STT: Blocked](https://img.shields.io/badge/STT-Blocked%20by%20Dependencies-yellow)](docs/PROJECT_STATUS.md)

Rust-based real-time audio capture and processing with robust recovery, VAD, and STT integration.

## Quick Start

```bash
# Build and run the app (STT requires vosk feature and system library)
cargo run --bin mic_probe  # Basic audio pipeline without STT
cargo run --features vosk  # With STT (requires libvosk installed)

# Probe binaries
cargo run --bin mic_probe -- --duration 30 --silence_threshold 120
cargo run --bin foundation_probe -- --duration 30

# Debug logging
RUST_LOG=debug cargo run --features vosk
```

## Features

- Reliable microphone capture with auto-recovery (watchdog)
- Device‑native capture to ring buffer (no resampling on capture thread)
- AudioChunker handles stereo→mono and resampling to 16 kHz
- Ring buffer and backpressure handling with stats
- Voice Activity Detection (Silero V5 via vendored fork)
- STT framework implemented (Vosk - requires system dependencies)
- Optional push-to-talk mode activated by holding <kbd>Ctrl</kbd>+<kbd>Super</kbd> with a small on-screen indicator centered one-third from the bottom of the screen

## Configuration

- CLI flags are the primary interface (see probes for examples).
  - `--activation-mode`: select `hotkey` (default) or `vad` to control how speech is detected
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
