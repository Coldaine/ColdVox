# User Configuration Design

## Overview

ColdVox provides comprehensive configuration through CLI arguments and environment variables, allowing users to customize audio capture, speech-to-text processing, and text injection behavior without code changes.

## Configuration Hierarchy

### Priority Order
1. **CLI Arguments** - Highest priority
2. **Environment Variables** - Fallback when CLI not specified
3. **Default Values** - Built-in defaults

### Environment Variable Pattern
All environment variables follow the `COLDVOX_*` prefix convention:
- `COLDVOX_STT_PREFERRED` - Preferred STT plugin
- `COLDVOX_ENABLE_TEXT_INJECTION` - Enable text injection
- `COLDVOX_ALLOW_YDOTOOL` - Allow ydotool backend

## Audio Configuration

### Device Selection
```bash
# CLI
--device "HyperX QuadCast"
--list-devices

# Environment
# (Device selection is CLI-only)
```

**Device Priority Logic:**
1. User-specified device (exact match)
2. User-specified device (substring match with warning)
3. ALSA "default" device (DE-aware)
4. "pipewire" device
5. OS default input device
6. Hardware preference scoring
7. System fallback

### Audio Quality
```bash
# CLI
--resampler-quality {fast|balanced|quality}

# Environment
# (Resampler quality is CLI-only)
```

**Quality Levels:**
- `fast`: Low CPU, higher latency
- `balanced`: Default, good CPU/quality trade-off
- `quality`: High CPU, best quality

## Speech-to-Text Configuration

### Plugin Management
```bash
# CLI
--stt-preferred vosk
--stt-fallbacks whisper,mock
--stt-require-local

# Environment
COLDVOX_STT_PREFERRED=vosk
COLDVOX_STT_FALLBACKS=whisper,mock
COLDVOX_STT_REQUIRE_LOCAL=true
```

### Resource Management
```bash
# CLI
--stt-max-mem-mb 512
--stt-model-ttl-secs 300
--stt-disable-gc

# Environment
COLDVOX_STT_MAX_MEM_MB=512
COLDVOX_STT_MODEL_TTL_SECS=300
COLDVOX_STT_DISABLE_GC=true
```

### Failover Behavior
```bash
# CLI
--stt-failover-threshold 3
--stt-failover-cooldown-secs 30

# Environment
COLDVOX_STT_FAILOVER_THRESHOLD=3
COLDVOX_STT_FAILOVER_COOLDOWN_SECS=30
```

### Language and Debugging
```bash
# CLI
--stt-language en
--stt-debug-dump-events
--stt-metrics-log-interval-secs 60

# Environment
COLDVOX_STT_LANGUAGE=en
COLDVOX_STT_DEBUG_DUMP_EVENTS=true
COLDVOX_STT_METRICS_LOG_INTERVAL_SECS=60
```

## Text Injection Configuration

### Backend Control
```bash
# CLI
--enable-text-injection
--allow-ydotool
--allow-kdotool
--allow-enigo

# Environment
COLDVOX_ENABLE_TEXT_INJECTION=true
COLDVOX_ALLOW_YDOTOOL=true
COLDVOX_ALLOW_KDOTOOL=true
COLDVOX_ALLOW_ENIGO=true
```

### Behavior Tuning
```bash
# CLI
--inject-on-unknown-focus
--restore-clipboard

# Environment
COLDVOX_INJECT_ON_UNKNOWN_FOCUS=true
COLDVOX_RESTORE_CLIPBOARD=true
```

### Performance Limits
```bash
# CLI
--max-total-latency-ms 5000
--per-method-timeout-ms 1000
--cooldown-initial-ms 100

# Environment
COLDVOX_INJECTION_MAX_LATENCY_MS=5000
COLDVOX_INJECTION_METHOD_TIMEOUT_MS=1000
COLDVOX_INJECTION_COOLDOWN_MS=100
```

## Transcription Persistence

### Storage Options
```bash
# CLI (requires 'vosk' feature)
--save-transcriptions
--save-audio
--output-dir /path/to/transcriptions
--transcript-format {json|csv|text}
--retention-days 30

# Environment
# (Transcription options are CLI-only)
```

## Interface Modes

### Activation Methods
```bash
# CLI
--activation-mode {vad|hotkey}
--tui

# Environment
# (Interface modes are CLI-only)
```

**Activation Modes:**
- `vad`: Voice Activity Detection triggers transcription
- `hotkey`: Manual hotkey activation (push-to-talk)

## Configuration Validation

### Required Dependencies
- **Vosk STT**: Requires `libvosk` system library and model files
- **Text Injection**: Platform-specific dependencies (ydotool, kdotool, etc.)
- **Audio Hardware**: Input device availability checked at startup

### Error Handling
- **Invalid values**: Graceful fallback to defaults with warnings
- **Missing dependencies**: Feature disabled with informative messages
- **Permission issues**: Runtime detection with guidance

## Advanced Usage

### Development Overrides
```bash
# Force mock behaviors for testing
MOCK_PACTL_OUTPUT="PulseAudio (on PipeWire)"
MOCK_APLAY_OUTPUT="pulse\npipewire"

# Hardware test control
COLDVOX_AUDIO_FORCE_HEADLESS=true
COLDVOX_AUDIO_FORCE_NON_HEADLESS=true
```

### Logging Configuration
```bash
# Standard Rust logging
RUST_LOG=debug
RUST_LOG=info,stt=debug,coldvox_audio=trace

# Application logging writes to:
# - stderr (with colors)
# - logs/coldvox.log (daily rotation, no ANSI)
```

## Configuration Examples

### Basic Voice Dictation
```bash
cargo run --features vosk,text-injection -- \
  --device "USB Microphone" \
  --activation-mode vad \
  --enable-text-injection
```

### High-Quality Recording Setup
```bash
cargo run --features vosk,text-injection -- \
  --device "HyperX QuadCast" \
  --resampler-quality quality \
  --save-transcriptions \
  --save-audio \
  --transcript-format json
```

### Development/Testing Configuration
```bash
RUST_LOG=debug \
COLDVOX_STT_PREFERRED=mock \
COLDVOX_STT_DEBUG_DUMP_EVENTS=true \
cargo run -- --tui --activation-mode hotkey
```

### Production Environment
```bash
COLDVOX_STT_PREFERRED=vosk \
COLDVOX_STT_REQUIRE_LOCAL=true \
COLDVOX_ENABLE_TEXT_INJECTION=true \
COLDVOX_RESTORE_CLIPBOARD=true \
cargo run --features vosk,text-injection --release
```

## Migration and Compatibility

### Environment Variable Changes
- Configuration schema is stable for minor version updates
- Major version changes may require environment variable migration
- Deprecated options remain functional with warnings for one major version

### Platform Differences
- **Linux**: Full feature set with PipeWire/ALSA integration
- **macOS/Windows**: Limited to Enigo text injection backend
- **Headless**: Audio tests automatically skipped in CI environments

## Troubleshooting

### Common Configuration Issues

1. **No Audio Device**: Check `--list-devices` output and permissions
2. **STT Not Working**: Verify Vosk model installation and `VOSK_MODEL_PATH`
3. **Text Injection Fails**: Review backend permissions and `--allow-*` flags
4. **Performance Issues**: Adjust `--resampler-quality` and memory limits

### Debug Configuration
```bash
RUST_LOG=debug \
COLDVOX_STT_DEBUG_DUMP_EVENTS=true \
COLDVOX_STT_METRICS_LOG_INTERVAL_SECS=10 \
cargo run -- --tui
```