# Candle Whisper Tests

This document describes the test coverage for the Candle-based Whisper implementation.

## Test Coverage

### types.rs Tests
✅ **11 comprehensive tests** covering:
- Whisper task equality and enum behavior
- TranscribeOptions default and custom configurations
- Segment creation and field validation
- Transcript creation with single and multiple segments
- Text trimming and concatenation
- Empty segment handling
- Clone implementations for types

### audio.rs Tests
✅ **11 tests** covering:
- Hz to Mel scale conversion (including edge cases: zero, nyquist frequency)
- PCM16 to f32 conversion with range validation
- Empty input handling
- Whisper audio constants validation
- Hann window generation (shape, boundary values, peak detection)
- Mel filterbank computation (shape, non-negativity, non-zero values)

### timestamps.rs Tests
✅ **10 tests** covering:
- Token to seconds conversion across ranges
- Timestamp token identification
- Timestamp rule application (overlaps, inversions, equal times, missing values)
- Segment extraction from empty token sequences
- Boundary value testing

## Running Tests

Due to ALSA system library requirements in the dependency tree, tests cannot be run in environments without audio libraries installed.

### In Development Environment:
```bash
# Install ALSA development libraries first
sudo apt-get install -y libasound2-dev pkg-config

# Run all tests for coldvox-stt with whisper feature
cargo test -p coldvox-stt --features whisper

# Run specific test module
cargo test -p coldvox-stt --features whisper candle::types
cargo test -p coldvox-stt --features whisper candle::audio
cargo test -p coldvox-stt --features whisper candle::timestamps
```

### CI/CD Considerations:
- Tests require ALSA libraries to be installed in the build environment
- Consider using `cargo-nextest` for parallel test execution
- Audio tests can be run on CPU without CUDA

## Test Gaps and Future Work

### Not Yet Tested:
- **loader.rs** - Model loading logic (requires actual model files)
- **decode.rs** - Decoder logic (requires loaded model and tokenizer)
- **mod.rs** - WhisperEngine integration (requires full model stack)
- **candle_whisper_plugin.rs** - Plugin adapter (requires integration testing)

### Integration Tests Needed:
1. **Model Loading Test**: Download small test model and verify loading
2. **End-to-End Transcription**: Test with known audio → verify output
3. **Multi-lingual Support**: Verify language detection and specification
4. **Timestamp Accuracy**: Validate timestamp generation with reference data
5. **Plugin Integration**: Test plugin lifecycle and transcription events
6. **Performance Benchmarks**: RTF, WER, latency, memory usage

### Recommended Test Fixtures:
- Small test audio files (3-10 seconds)
- Pre-computed mel spectrograms for audio tests
- Reference transcripts for accuracy validation
- Minimal test model (e.g., tiny.en) for integration tests

## Test Implementation Notes

All tests are:
- **Unit tests** using `#[cfg(test)]` and `#[test]` attributes
- **Self-contained** without external dependencies where possible
- **Documented** with clear assertions and failure messages
- **Fast** - focusing on logic validation without I/O

The tests validate:
- ✅ Type correctness and API contracts
- ✅ Mathematical operations (Hz/Mel conversion, audio processing)
- ✅ Edge cases and boundary conditions
- ✅ Error handling paths
- ✅ Invariants and postconditions
