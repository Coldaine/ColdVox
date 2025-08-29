# Vosk Implementation Gaps and Improvements

## Current Status

Vosk integration is functional behind the `vosk` feature flag and is disabled
by default unless a model is found. This document outlines gaps and recommended
improvements.

## Priority 1: Critical Gaps

### 1.1 Missing Unit Tests

**Issue**: No unit tests for STT modules (vosk.rs, processor.rs)

**Impact**: Cannot verify correctness or catch regressions

**Solution**:

```rust
// Add to crates/app/src/stt/vosk.rs
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_transcriber_creation() {
        // Test with valid/invalid model paths
    }
    
    #[test]
    fn test_accept_frame() {
        // Test audio processing
    }
    
    #[test]
    fn test_utterance_lifecycle() {
        // Test utterance ID management
    }
}
```

### 1.2 No STT Metrics in TUI Dashboard

**Issue**: TUI dashboard doesn't display STT metrics

**Impact**: Cannot monitor STT performance in real-time

**Solution**: Add STT panel to tui_dashboard.rs showing:

-  Transcription rate (words/minute)
-  Partial/final counts
-  Error rate
-  Processing latency
-  Current utterance state

### 1.3 No Error Recovery Mechanism

**Issue**: STT processor lacks automatic recovery from failures

**Impact**: System continues with broken STT instead of recovering

**Solution**: Implement watchdog pattern similar to AudioCapture:

-  Monitor consecutive error count
-  Automatic model reload on failure
-  Exponential backoff retry
-  Health check mechanism

## Priority 2: Operational Features

### 2.1 Transcription Persistence

**Issue**: Transcriptions only logged, not stored

**Impact**: Cannot review historical transcriptions

**Solution**:

```rust
// Add transcription storage
pub struct TranscriptionStore {
    output_dir: PathBuf,
    format: OutputFormat, // JSON, CSV, SQLite
    rotation: RotationPolicy,
}
```

### 2.2 Runtime Configuration Updates

**Issue**: Must restart to change STT settings

**Impact**: Poor operational flexibility

**Solution**:

-  Add config reload mechanism
-  Support model hot-swapping
-  Dynamic enable/disable STT

### 2.3 Performance Benchmarking

**Issue**: No performance benchmarks

**Impact**: Unknown processing overhead and latency

**Solution**:

-  Add criterion benchmarks
-  Measure transcription latency
-  Test with different model sizes
-  Profile memory usage

## Priority 3: Enhanced Features

### 3.1 Multi-Model Support

**Issue**: Only one model at a time

**Impact**: Cannot optimize for different use cases

**Solution**:

-  Support model switching based on context
-  Parallel processing with multiple models
-  Quality vs speed trade-offs

### 3.2 Integration Tests

**Issue**: No end-to-end tests with real audio

**Impact**: Cannot verify full pipeline

**Solution**:

-  Test with WAV files
-  Verify VAD→STT integration
-  Test error scenarios

### 3.3 Production Deployment

**Issue**: No production deployment documentation

**Impact**: Unclear how to deploy in production

**Solution**: Document:

-  Systemd service configuration
-  Resource requirements
-  Monitoring setup
-  Log management
-  API endpoints

## Implementation Plan

### Phase 1: Testing & Monitoring (1 week)

1.  Add unit tests for STT modules
2.  Integrate STT metrics into TUI dashboard
3.  Add integration test with WAV files

### Phase 2: Reliability (1 week)

1.  Implement error recovery mechanism
2.  Add health checks
3.  Add performance benchmarks

### Phase 3: Operations (2 weeks)

1.  Add transcription persistence
2.  Implement runtime config updates
3.  Create production deployment guide

### Phase 4: Enhancements (Optional)

1.  Multi-model support
2.  Advanced features (speaker diarization, etc.)
3.  API endpoints for external access

## Testing Requirements

### Unit Tests Needed

-  [ ] VoskTranscriber creation with valid/invalid models
-  [ ] Audio frame processing
-  [ ] Utterance lifecycle management
-  [ ] Configuration updates
-  [ ] Error handling paths

### Integration Tests Needed

-  [ ] VAD → STT event flow
-  [ ] End-to-end with test audio files
-  [ ] Concurrent processing
-  [ ] Memory leak tests
-  [ ] Performance under load

### Example Test Audio Files

```bash
# Download test audio files
mkdir -p test_data
cd test_data

# LibriSpeech samples (16kHz, mono)
wget http://www.openslr.org/resources/12/test-clean.tar.gz
tar -xzf test-clean.tar.gz

# Or create custom test files
sox input.wav -r 16000 -c 1 test_16khz_mono.wav
```

## Configuration Enhancements

### Proposed Config Structure

```toml
[stt]
enabled = true
engine = "vosk"  # Future: whisper, deepgram, etc.

[stt.vosk]
model_path = "models/vosk-model-small-en-us-0.15"
partial_results = true
max_alternatives = 1
include_words = false
buffer_size_ms = 512

[stt.vosk.fallback]
enabled = true
model_path = "models/vosk-model-tiny-en-us-0.15"

[stt.output]
format = "json"  # json, csv, sqlite
directory = "transcriptions"
rotation = "daily"
keep_days = 30
```

## Monitoring & Observability

### Metrics to Track

-  Latency: Time from audio received to transcription emitted
-  Throughput: Words per minute transcribed
-  Accuracy: (Requires reference transcripts)
-  Resource Usage: CPU, memory per model
-  Error Rate: Failed transcriptions per hour

### Logging Improvements

```rust
// Add structured logging fields
tracing::info!(
    target: "stt",
    utterance_id = %id,
    duration_ms = duration.as_millis(),
    word_count = words.len(),
    model = config.model_path,
    "Transcription completed"
);
```

## API Design (Future)

### REST Endpoints

```text
GET  /api/v1/stt/status          # STT system status
GET  /api/v1/stt/metrics         # Current metrics
GET  /api/v1/stt/transcriptions  # Recent transcriptions
POST /api/v1/stt/config          # Update configuration
```

### WebSocket Stream

```text
WS /api/v1/stt/stream  # Real-time transcription stream
```

## Summary

The Vosk implementation is functional but needs:

1.  Testing: Unit and integration tests
2.  Monitoring: TUI dashboard integration
3.  Reliability: Error recovery mechanism
4.  Operations: Persistence and config management
5.  Documentation: Production deployment guide

These improvements will make the STT system production-ready and maintainable.
