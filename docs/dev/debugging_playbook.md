# ColdVox Debugging Playbook

## Overview

This playbook provides troubleshooting steps for common issues in ColdVox subsystems. Use this guide when encountering failures in development, testing, or production.

## Audio Pipeline Issues

### No Audio Data / Silent Capture

**Symptoms**: Watchdog timeout errors, no VAD events, silent recordings.

**Checks**:
1. Verify device selection: `cargo run --bin mic_probe list-devices`
2. Check device permissions (Linux): `arecord -l`
3. Test capture: `cargo run --bin mic_probe mic-capture --duration 5`
4. Watchdog logs: Look for "Watchdog timeout! No audio data for 5s"

**Solutions**:
- Select correct device: `cargo run --bin mic_probe -- --device "USB Audio"`
- PulseAudio/JACK conflicts: Use PipeWire or ALSA directly
- Permissions: `sudo usermod -a -G audio $USER`

### Frame Size / Sample Rate Mismatches

**Symptoms**: VAD errors, incorrect timing, resampling failures.

**Constants**:
- Frame size: 512 samples (32ms at 16kHz)
- Sample rate: 16kHz mono
- Buffer size: 64k samples minimum

**Checks**:
- Verify constants in `crates/coldvox-vad/src/constants.rs`
- Check resampler config in chunker logs
- Test with known WAV: `cargo run --example record_10s`

### Ring Buffer Backpressure

**Symptoms**: Dropped frames, performance degradation.

**Checks**:
- Buffer size: 16384 * 4 = 65536 samples
- Logs: "Ring buffer full, dropping write"
- Metrics: `capture_buffer_fill_pct`, `chunker_buffer_fill_pct`

**Solutions**:
- Increase buffer size in `AudioRingBuffer::new()`
- Reduce consumer latency
- Monitor with `PipelineMetrics`

## VAD Issues

### No Speech Detection

**Symptoms**: VAD never triggers, no SpeechStart events.

**Checks**:
- Threshold: Silero default 0.3, runtime 0.1
- Window size: 512 samples
- Audio format: 16kHz mono f32

**Solutions**:
- Adjust threshold: Lower for sensitive detection
- Test with WAV: `cargo run --example test_silero_wav`
- Check audio levels: Use `mic_probe` to verify input

### False Positives / Noise Triggers

**Symptoms**: VAD triggers on noise, short utterances split.

**Checks**:
- Silence duration: 500ms (prevents splits)
- Min speech duration: 100ms
- Energy levels in logs

**Solutions**:
- Increase `min_silence_duration_ms` to 1000ms
- Adjust threshold higher
- Test debounce with continuous speech

## STT Issues

### Model Loading Failures

**Symptoms**: "Failed to locate Vosk model", STT disabled.

**Checks**:
- Path: `VOSK_MODEL_PATH` env var or default `models/vosk-model-small-en-us-0.15`
- Model files: `graph/` directory exists
- Logs: "Vosk model resolved"

**Solutions**:
- Set `export VOSK_MODEL_PATH=/path/to/model`
- Download model: See `scripts/ci/setup-vosk-cache.sh`
- Verify model integrity

### No Transcriptions

**Symptoms**: Speech detected but no partial/final events.

**Checks**:
- Plugin loaded: Logs show "Using preferred STT plugin: vosk"
- Session events: SpeechStart/End translated to SessionEvent
- Model inference: Vosk logs during speech

**Solutions**:
- Check plugin selection: `--stt-preferred vosk`
- Test standalone: `cargo run --example vosk_test`
- Verify audio format to STT (16kHz mono)

### Transcription Quality Issues

**Symptoms**: Incorrect text, partial failures.

**Checks**:
- Audio quality: Clean 16kHz mono
- Model size: small-en-us vs larger models
- Logs: Partial results enabled

**Solutions**:
- Use larger model for better accuracy
- Ensure clean audio input
- Check `TranscriptionConfig` settings

## Text Injection Issues

### Backend Selection Failures

**Symptoms**: Injection fails, fallback errors.

**Checks**:
- Platform detection: Linux → AT-SPI + clipboard + ydotool/kdotool
- Permissions: uinput access (`sudo usermod -a -G input $USER`)
- Logs: "Injection backend available: atspi"

**Solutions**:
- Test backends: `cargo run --example inject_demo`
- Check permissions: `ls -l /dev/uinput`
- Simulate environments for testing

### Injection Timing / Reliability

**Symptoms**: Text appears late or not at all.

**Checks**:
- Latency settings: `max_total_latency_ms`
- Focus detection: `inject_on_unknown_focus`
- Logs: Injection success/failure

**Solutions**:
- Adjust timeouts: Increase `per_method_timeout_ms`
- Enable fallbacks: Multiple backends
- Test focus: Ensure target window has focus

## Hotkey Issues

### Registration Failures

**Symptoms**: Hotkeys not working, no global shortcuts.

**Checks**:
- Backend: KDE → KGlobalAccel, others → fallback
- Logs: "Successfully registered shortcut"
- Permissions: DBus access

**Solutions**:
- Test backend: `cargo run --example test_hotkey_backend`
- KDE check: `kglobalaccel5 --list`
- Fallback: Ensure generic hotkey support

### Event Handling

**Symptoms**: Hotkey pressed but no VAD events.

**Checks**:
- Event routing: Hotkey → raw_vad_tx → fanout
- Logs: "Hotkey pressed" events
- Mode: ActivationMode::Hotkey

**Solutions**:
- Verify mode: Default is Vad, use `--activation-mode hotkey`
- Test events: `cargo run --example test_kglobalaccel_hotkey`

## Build & Feature Issues

### Feature Gate Problems

**Symptoms**: Missing dependencies, compile errors.

**Checks**:
- Default features: `silero`, `text-injection`, `vosk`
- Optional: `examples`, `tui`, etc.
- Logs: Feature activation in build

**Solutions**:
- Build with features: `cargo build --features vosk,text-injection`
- Check Cargo.toml for gates
- Use `--all-features` for testing

### Dependency Conflicts

**Symptoms**: Linker errors, version mismatches.

**Checks**:
- `cargo tree` for conflicts
- Platform-specific deps

**Solutions**:
- Use `cargo tree -d` to find duplicates
- Update dependencies
- Check platform compatibility

## Logging & Observability

### Missing Logs

**Symptoms**: No output, silent failures.

**Checks**:
- Level: `RUST_LOG=debug`
- Dual output: Console + file (`logs/coldvox.log`)
- TUI: File-only to avoid corruption

**Solutions**:
- Set `RUST_LOG=trace cargo run ...`
- Check file logs: `tail -f logs/coldvox.log`
- TUI logs: Use `--log-level debug`

### Metrics Issues

**Symptoms**: No performance data, FPS = 0.

**Checks**:
- `PipelineMetrics` updates
- Frame rates: capture_fps, chunker_fps, vad_fps
- Logs: Metrics summaries

**Solutions**:
- Enable metrics: Default enabled
- Check subscriptions: Audio frames have listeners
- Monitor with TUI dashboard

## Testing Issues

### Hardware Test Failures

**Symptoms**: Tests fail on CI, pass locally.

**Checks**:
- Real hardware: All tests use actual devices
- Model paths: `VOSK_MODEL_PATH` set
- Permissions: Audio/input access

**Solutions**:
- Setup models: `./scripts/ci/setup-vosk-cache.sh`
- Hardware simulation: Use loopback devices
- Skip hardware tests: `--skip live-hardware-tests`

### Integration Test Timeouts

**Symptoms**: Tests hang, timeout after 60s.

**Checks**:
- Audio flow: Capture → chunker → VAD → STT
- Buffer sizes: Adequate for test duration
- Logs: Progress indicators

**Solutions**:
- Increase timeouts: `timeout 120 cargo test`
- Debug flow: Add logging to pipelines
- Reduce test duration

## Performance Issues

### High CPU Usage

**Symptoms**: Excessive CPU, fan noise.

**Checks**:
- Resampler quality: `Balanced` vs `Fast`
- Buffer sizes: Not too large
- Metrics: Frame rates >30 FPS

**Solutions**:
- Use `ResamplerQuality::Fast`
- Profile: `cargo flamegraph`
- Reduce processing: Disable unused features

### Latency Problems

**Symptoms**: Delayed responses, audio lag.

**Checks**:
- Buffer sizes: Smaller for lower latency
- Frame size: 512 samples = 32ms
- Logs: Latency measurements

**Solutions**:
- Reduce buffer: `AudioRingBuffer::new(8192)`
- Faster resampler: `ResamplerQuality::Fast`
- Monitor metrics: `pipeline_latency_ms`

## Platform-Specific Issues

### Linux Desktop

**Audio**:
- PipeWire preferred over PulseAudio
- ALSA fallback: `export ALSA_DEFAULT=default`
- Permissions: `sudo usermod -a -G audio,input $USER`

**Injection**:
- AT-SPI: Requires accessibility enabled
- ydotool: uinput permissions
- Clipboard: wl-clipboard for Wayland

### Windows/macOS

**Audio**: CoreAudio/AVFoundation default
**Injection**: Enigo backend
**Permissions**: Admin access may be required

## Emergency Recovery

### Complete Reset

1. Stop all processes: `pkill -f coldvox`
2. Clear logs: `rm -rf logs/`
3. Reset config: `rm -f plugins.json`
4. Rebuild: `cargo clean && cargo build`
5. Test minimal: `cargo run --no-default-features --features silero -- --activation-mode vad`

### Debug Mode

1. Enable full logging: `RUST_LOG=trace`
2. Run with features: `--features vosk,text-injection`
3. Capture output: `tee debug.log`
4. Analyze logs for error patterns

### Minimal Reproduction

1. Disable features: `--no-default-features --features silero`
2. Test components individually
3. Add features one by one
4. Isolate failing component

## Common Error Patterns

- **"No audio data"**: Device issues, permissions, conflicts
- **"Model not found"**: Path issues, missing downloads
- **"Injection failed"**: Backend unavailable, permissions
- **"Hotkey not working"**: Backend mismatch, permissions
- **"Timeout"**: Performance issues, buffer problems
- **"Silent failure"**: Logging disabled, error suppression

## Tools & Commands

```bash
# Audio debugging
cargo run --bin mic_probe list-devices
cargo run --bin mic_probe mic-capture --duration 5
arecord -l  # Linux audio devices

# STT debugging
export VOSK_MODEL_PATH=models/vosk-model-small-en-us-0.15
cargo run --example vosk_test

# Injection debugging
cargo run --features text-injection --example inject_demo

# Hotkey debugging
cargo run --example test_hotkey_backend

# Full diagnostics
cargo run --bin tui_dashboard --log-level debug
```

## Contributing Fixes

When fixing issues:
1. Add logging for error paths
2. Update this playbook with new patterns
3. Test on multiple platforms
4. Document environment requirements
5. Include reproduction steps