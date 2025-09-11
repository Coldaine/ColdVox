# STT Plugin Architecture Migration Guide

This document explains how to migrate from the old direct VoskTranscriber usage to the new plugin-based STT architecture.

## Overview

The STT system has been refactored to use a plugin architecture that supports:
- Multiple STT backends (Vosk, Whisper, Mock, NoOp)
- Automatic failover between backends
- Resource management (model loading/unloading)
- Configurable retry logic
- Comprehensive metrics and logging

## CLI Changes

### New Flags

- `--stt-backend <backend>`: Select STT backend (auto, whisper, vosk, mock, noop)
- `--stt-fallback <list>`: Comma-separated fallback order (default: whisper,vosk,mock,noop)
- `--stt-max-mem-mb <mb>`: Maximum memory limit for STT plugins
- `--stt-max-retries <count>`: Retry attempts before failover (default: 3)
- `--whisper-model-path <path>`: Path to Whisper model file
- `--whisper-mode <mode>`: Whisper quality mode (fast/balanced/quality)
- `--whisper-quant <type>`: Quantization type (q5_1/q8_0/fp16)

### Deprecated Flags (with backward compatibility)

- `--vosk-model-path`: Still works but shows deprecation warning. Use `--stt-backend=vosk` instead.

### Environment Variables

All CLI flags have corresponding environment variables:
- `COLDVOX_STT_BACKEND`
- `COLDVOX_STT_FALLBACK`
- `COLDVOX_STT_MAX_MEM_MB`
- `COLDVOX_STT_MAX_RETRIES`
- `WHISPER_MODEL_PATH`
- `COLDVOX_WHISPER_MODE`
- `COLDVOX_WHISPER_QUANT`

## Usage Examples

### Basic Usage

```bash
# Use automatic backend selection
./coldvox --stt-backend auto

# Use specific backend
./coldvox --stt-backend whisper --whisper-model-path ./models/whisper.bin

# Use Vosk with backward compatibility
./coldvox --stt-backend vosk --vosk-model-path ./models/vosk-model

# Use mock for testing
./coldvox --stt-backend mock
```

### Advanced Configuration

```bash
# Custom fallback order
./coldvox --stt-backend auto --stt-fallback vosk,whisper,mock

# Memory constraints
./coldvox --stt-backend auto --stt-max-mem-mb 1000

# Retry configuration
./coldvox --stt-backend whisper --stt-max-retries 5
```

### Testing with Synthetic Data

```bash
# Test all backends
cargo run --features examples --example synthetic_stt -- --test-all

# Test specific backend
cargo run --features examples --example synthetic_stt -- --backend whisper --model-path ./models/whisper.bin

# Test with mock data
cargo run --features examples --example synthetic_stt -- --backend mock --duration 10
```

## Architecture Overview

```
Runtime → SttPluginManager → SttPlugin (Vosk/Whisper/Mock/NoOp)
                          ↓
                     PluginAdapter (implements StreamingStt)
                          ↓
                     SttProcessor (generic over StreamingStt)
```

### Key Components

1. **SttPluginManager**: Manages plugin lifecycle, selection, and failover
2. **PluginAdapter**: Adapts SttPlugin interface to StreamingStt trait
3. **SttProcessor**: Generic processor that works with any StreamingStt implementation
4. **SttPlugin trait**: Common interface for all STT backend implementations

### Plugin Implementations

- **VoskPlugin**: Wraps existing VoskTranscriber
- **WhisperPlugin**: Mock implementation (ready for whisper-rs integration)
- **MockPlugin**: Returns synthetic transcriptions for testing
- **NoOpPlugin**: Returns no transcriptions (minimal overhead)

## Error Handling

The new system includes comprehensive error classification:

### Error Types

- **Transient errors**: Retry automatically (DecodeTimeout, ResourceExhausted, IoError)
- **Failover errors**: Switch to next plugin (InitializationFailed, BackendUnavailable, ModelLoadFailed)
- **Permanent errors**: Log and continue (ConfigurationError)

### Failover Behavior

1. Plugin fails → Check error type
2. If transient → Retry with exponential backoff (up to max_retries)
3. If should_failover → Switch to next plugin in fallback list
4. If all plugins fail → Use NoOpPlugin as last resort

## Metrics and Monitoring

New metrics added to PipelineMetrics:
- `active_stt_backend`: Currently active backend name
- `last_stt_decode_ms`: Last decode time in milliseconds
- `last_stt_audio_duration_ms`: Last audio duration processed
- `last_stt_rt_factor`: Real-time factor (decode_time / audio_duration)
- `stt_failover_count`: Number of failovers that occurred

### Structured Logging

Failover events are logged with structured data:
```rust
tracing::warn!(
    event = "stt_failover", 
    from = %previous_backend,
    to = %new_backend,
    reason = %error_reason
);
```

## Resource Management

### Model Lifecycle

- Models are loaded on-demand when plugin is selected
- Models are unloaded after TTL expiry (default: 5 minutes) 
- Models are unloaded during failover to free memory
- Memory usage is tracked and reported in metrics

### Memory Management

- Plugins report estimated memory usage
- System can limit plugin selection based on memory constraints
- Automatic garbage collection of inactive models

## Testing

### Unit Tests

```bash
# Run STT plugin tests
cargo test stt_plugin_tests

# Run with specific features
cargo test --features vosk,whisper stt_plugin_tests
```

### Integration Tests

```bash
# Run synthetic STT example
cargo run --features examples --example synthetic_stt

# Test headless validation
cargo test --features examples --test synthetic_stt
```

### Live Tests

```bash
# Test with actual microphone (requires live-hardware-tests feature)
cargo test --features live-hardware-tests
```

## Migration Checklist

For existing installations:

- [ ] Update CLI arguments if using custom STT configuration
- [ ] Test with `--stt-backend mock` first to verify pipeline works
- [ ] Configure fallback order for your use case
- [ ] Update any scripts/configs that used `--vosk-model-path`
- [ ] Test failover behavior in your environment
- [ ] Monitor new metrics for performance insights

## Troubleshooting

### Common Issues

1. **No STT plugins available**: Check that model files exist and are readable
2. **Plugin initialization failed**: Check model paths and file permissions  
3. **High memory usage**: Set `--stt-max-mem-mb` to limit plugin selection
4. **Frequent failovers**: Check model quality and adjust `--stt-max-retries`

### Debug Commands

```bash
# List available devices and STT status
./coldvox --list-devices

# Enable debug logging
RUST_LOG=debug ./coldvox --stt-backend auto

# Test specific plugin
cargo run --features examples --example synthetic_stt -- --backend vosk
```