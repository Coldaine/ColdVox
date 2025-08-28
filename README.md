# ColdVox – Voice AI audio pipeline

[![Status: Phase 4 Complete](https://img.shields.io/badge/Status-Phase%204%20Complete-brightgreen)](docs/PROJECT_STATUS.md)

Rust-based real-time audio capture and processing with robust recovery, VAD, and STT integration.

## Quick Start

```bash
# Build and run the app
cargo run

# Probe binaries
cargo run --bin mic_probe -- --duration 30 --silence_threshold 120
cargo run --bin foundation_probe -- --duration 30

# Debug logging
RUST_LOG=debug cargo run
```

## Features

- Reliable microphone capture with auto-recovery (watchdog)
- Format/channel negotiation with downmixing to 16 kHz mono
- Ring buffer and backpressure handling with stats
- Voice Activity Detection (Silero V5 via vendored fork)
- STT ready (Vosk plan), probes and demos in `examples/`

## Configuration

- CLI flags are the primary interface (see probes for examples).
- Environment variables are not required; `RUST_LOG` can control verbosity.

## Troubleshooting

- No audio frames: check device permissions, try a different input device using `mic_probe`.
- Watchdog triggers repeatedly: lower `--silence_threshold` or verify device sample format.
- Frame drops: ensure a consumer drains the channel; long processing on the main thread can cause backpressure.

## Architecture

```mermaid
flowchart LR
	A[CPAL Input Stream] --> B[Frame Channel (bounded)]
	B --> C[Processing Task]
	C --> D[VAD]
	D --> E[Chunks / STT]
	C --> F[Stats + Watchdog]
```

See `crates/app` and `docs/` for deeper architecture notes.