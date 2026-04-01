---
doc_type: reference
subsystem: general
status: draft
freshness: stale
preservation: preserve
last_reviewed: 2025-10-31
owners: Documentation Working Group
version: 1.0.0
---

# ColdVox Logging Configuration

## Overview

ColdVox uses the `tracing` crate for structured logging with configurable verbosity levels. Logs are written to both stderr (console) and a daily-rotated file in `logs/coldvox.log`.

## Log Levels

ColdVox uses standard Rust log levels, from least to most verbose:

- **ERROR** - Critical errors that require immediate attention
- **WARN** - Warning messages about potential issues
- **INFO** - Standard operational messages (default)
- **DEBUG** - Detailed debugging information (includes silence detection events)
- **TRACE** - Maximum verbosity (includes every audio chunk processed)

## Default Configuration

By default, ColdVox runs at **INFO** level to provide useful operational feedback without overwhelming the logs. This was changed from DEBUG to reduce verbosity in normal operation.

## Controlling Log Levels

### Environment Variable

Set the `RUST_LOG` environment variable to control logging:

```bash
# Standard logging (default, recommended for production)
RUST_LOG=info cargo run

# Verbose debugging (includes silence detection, good for development)
RUST_LOG=debug cargo run

# Maximum verbosity (includes every audio chunk - very noisy!)
RUST_LOG=trace cargo run

# Silence all logs except errors
RUST_LOG=error cargo run
```

### Per-Module Filtering

You can set different log levels for different modules:

```bash
# Info for most modules, but trace for STT debugging
RUST_LOG=info,stt_debug=trace cargo run

# Debug for ColdVox, trace for audio processing
RUST_LOG=coldvox=debug,coldvox_audio=trace cargo run

# Info everywhere except silence detector at debug
RUST_LOG=info,coldvox_audio::detector=debug cargo run
```

### TUI Dashboard

The TUI dashboard accepts a `--log-level` flag:

```bash
# Run TUI with debug logging
cargo run --bin tui_dashboard -- --log-level debug

# Run TUI with trace logging
cargo run --bin tui_dashboard -- --log-level trace
```

## Specific Logging Targets

### STT Debug (`stt_debug`)

The `stt_debug` target provides detailed information about speech-to-text processing:

- **TRACE**: Every audio chunk dispatch, plugin calls, and processing results
- **DEBUG**: Finalization events, transcription events, plugin state changes
- **WARN**: Processing errors

```bash
# Enable detailed STT debugging without affecting other modules
RUST_LOG=info,stt_debug=trace cargo run
```

### Audio Detector (`coldvox_audio::detector`)

Silence detection events:

- **DEBUG**: Silence start/end events with RMS values and thresholds
- **TRACE**: Per-sample RMS calculations (very verbose)

```bash
# Debug silence detection
RUST_LOG=info,coldvox_audio::detector=debug cargo run
```

## Log Files

### Location

Logs are written to:
- Console: stderr
- File: `logs/coldvox.log` (current day)
- Rotated: `logs/coldvox.log.YYYY-MM-DD` (previous days)

### Rotation

- Logs rotate daily at midnight
- Old logs are automatically pruned after 7 days (configurable)
- File logs have ANSI codes disabled for clean analysis

### Viewing Logs

```bash
# Tail current logs
tail -f logs/coldvox.log

# View with filtering
grep "ERROR" logs/coldvox.log

# Follow only STT events
tail -f logs/coldvox.log | grep stt_debug
```

## Troubleshooting

### Too Much Noise

If logs are overwhelming:

1. **Use INFO level** (default): `RUST_LOG=info cargo run`
2. **Filter specific targets**: `RUST_LOG=info,stt_debug=warn cargo run`
3. **Focus on errors**: `RUST_LOG=error cargo run`

### Not Enough Detail

If you need more information:

1. **Enable DEBUG**: `RUST_LOG=debug cargo run`
2. **Enable specific module**: `RUST_LOG=info,coldvox_audio=debug cargo run`
3. **Full trace**: `RUST_LOG=trace cargo run` (warning: very verbose!)

### Audio Processing Issues

For debugging audio pipeline problems:

```bash
RUST_LOG=info,coldvox_audio=debug,stt_debug=debug cargo run
```

### Plugin Issues

For debugging STT plugin problems:

```bash
RUST_LOG=info,stt_debug=trace cargo run
```

## Performance Impact

- **INFO**: Minimal performance impact (recommended)
- **DEBUG**: Low to moderate impact (includes silence detection)
- **TRACE**: Significant impact (logs every audio chunk, ~60-100 times per second)

For production use, keep logging at INFO or WARN level.

## Recent Changes

As of this commit:

1. **Default level changed from DEBUG to INFO** to reduce verbosity
2. **High-frequency logs downgraded**:
   - Silence detector: INFO → DEBUG
   - Audio chunk dispatch: INFO → TRACE
   - Plugin process calls: DEBUG → TRACE
   - Plugin process results: DEBUG → TRACE (success) / WARN (errors)
3. **Documentation improved** with examples for common use cases

These changes significantly reduce log noise while maintaining useful operational feedback. Users who need detailed debugging can still enable it via `RUST_LOG=debug` or `RUST_LOG=trace`.
