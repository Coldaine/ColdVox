# ColdVox - Voice AI Audio Processing

A Rust-based voice AI project focused on real-time audio processing with emphasis on reliability and automatic recovery.

## Quick Start

```bash
# Build and run
cargo run

# Run with debug logging
RUST_LOG=debug cargo run

# Run with specific module debugging
RUST_LOG=coldvox_app::audio=debug cargo run
```

## Logging

ColdVox uses structured logging with automatic file rotation:

- **Console Output**: Real-time logs during development
- **File Storage**: Persistent logs saved to `logs/` directory
- **Daily Rotation**: Automatic file rotation prevents disk space issues
- **Environment Control**: Use `RUST_LOG` to control verbosity

### Log Files Location
```
logs/
├── coldvox.log.2024-08-24    # Previous days
├── coldvox.log.2024-08-25    # Yesterday  
└── coldvox.log               # Today's logs
```

### Log Level Examples
```bash
cargo run                           # Info level (default)
RUST_LOG=error cargo run           # Errors only
RUST_LOG=debug cargo run           # Detailed debugging
RUST_LOG=warn cargo run             # Warnings and above
```

For detailed logging documentation, see [`docs/Logging_Configuration.md`](docs/Logging_Configuration.md).

## Architecture

- **Foundation Layer**: Error handling, health monitoring, state management
- **Audio System**: Microphone capture, device management, watchdog monitoring
- **VAD Integration**: Voice activity detection using Silero model
- **Lock-free Communication**: Ring buffers and MPSC channels

## Development

```bash
# Run tests
cargo test

# Run specific binary probes
cargo run --bin foundation_probe -- --duration 60
cargo run --bin mic_probe -- --duration 120

# Build release
cargo build --release
```

See [`CLAUDE.md`](CLAUDE.md) for detailed development guidance.