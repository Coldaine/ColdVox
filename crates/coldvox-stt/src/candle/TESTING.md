# Candle Whisper Testing Plan

This document outlines the testing strategy for the Candle Whisper implementation, including which tests can run in CI/headless environments and which require manual testing.

## Tests That Run in CI (Headless)

These tests are included in the codebase and run automatically:

### types.rs
- ✅ Device equality and parsing
- ✅ TranscribeOptions defaults and custom values
- ✅ Segment creation and edge cases (zero duration, negative times)
- ✅ Transcript structure
- ✅ WhisperEngineInit configuration
- ⚠️ **Limited**: CUDA device creation (tests error messages, not actual devices)

### audio.rs
- ✅ PCM to f32 conversion (boundary values, empty input, range validation)
- ✅ Pad/trim operations (short audio, long audio, exact length, empty)
- ✅ Constants verification
- ✅ Log-mel spectrogram with CPU device (empty, short, exact length)
- ✅ Both i16 and f32 input variants

### loader.rs
- ✅ Model format detection (safetensors, gguf, invalid formats)
- ✅ Path handling (nested paths, case sensitivity)
- ✅ Error handling for missing files
- ✅ Format equality checks
- ❌ **Cannot test**: Actual model loading (requires model files)

### timestamps.rs
- ✅ Token to seconds conversion (boundary values, non-timestamp tokens, precision)
- ✅ Segment extraction (empty, no timestamps, trailing text, whitespace filtering)
- ✅ SegmentBuilder state management
- ✅ Constants verification

### decode.rs
- ✅ DecoderState initialization and state transitions
- ✅ MAX_TOKENS limit enforcement
- ✅ Special token constant verification
- ❌ **Cannot test**: Actual decoding (requires model and tokenizer)

### mod.rs (WhisperEngine)
- ✅ API structure tests (via existing unit tests)
- ❌ **Cannot test**: Engine initialization (requires model files)
- ❌ **Cannot test**: Transcription pipeline (requires model and audio)

### whisper_candle.rs (Plugin)
- ✅ Buffer size constants
- ✅ Model size memory estimates
- ✅ Plugin info and capabilities
- ✅ Device parsing (CPU, CUDA, Metal)
- ✅ Builder pattern validation
- ✅ Factory creation
- ❌ **Cannot test**: Actual transcription (requires model and initialization)

## Tests That Require Model Files (Manual/Integration)

These tests are documented as commented-out code blocks in the source files. They require actual Whisper model files to run.

### Location: `loader.rs`
```rust
#[test]
#[ignore] // Requires model files
fn test_load_safetensors_real_model()
```
**Requirements:**
- Download a Whisper model from HuggingFace (e.g., `openai/whisper-tiny`)
- Place in `tests/fixtures/whisper-tiny/`
- Files needed: `model.safetensors`, `config.json`, `tokenizer.json`

**How to run:**
```bash
cargo test test_load_safetensors_real_model --ignored -- --nocapture
```

### Location: `decode.rs`
```rust
#[test]
#[ignore] // Requires model files
fn test_init_prompt_transcribe()
fn test_init_prompt_translate()
fn test_sample_token_greedy()
fn test_sample_token_temperature()
fn test_full_decode_pipeline()
```
**Requirements:**
- Same as above, plus test audio files
- Recommended: Use `tests/fixtures/audio/sample.wav` (16kHz mono)

**How to run:**
```bash
cargo test --features whisper test_init_prompt --ignored -- --nocapture
```

## Tests That Require Network Access

### Location: `loader.rs`
```rust
#[test]
#[ignore] // Requires network access
fn test_download_model_from_hub()
```
**Requirements:**
- Internet connection
- HuggingFace Hub access (no authentication needed for public models)

**How to run:**
```bash
cargo test test_download_model_from_hub --ignored -- --nocapture
```

**Note:** This test downloads ~100MB, so it's slow and should not run in CI.

## Tests That Require GPU Hardware

### CUDA Tests
**Requirements:**
- NVIDIA GPU with CUDA support
- CUDA toolkit installed
- cuDNN libraries

**How to test:**
```bash
# Set device to CUDA
WHISPER_DEVICE=cuda:0 cargo test --features whisper --ignored
```

**Tests affected:**
- `types.rs::test_cuda_device_creation` (full test, not just error messages)
- `whisper_candle.rs::test_parse_device_cuda` (validation)
- Full pipeline tests with CUDA device

### Metal Tests (Apple Silicon)
**Requirements:**
- Apple Silicon Mac (M1, M2, M3, etc.)
- macOS 12.0 or later

**How to test:**
```bash
WHISPER_DEVICE=metal cargo test --features whisper --ignored
```

**Tests affected:**
- `types.rs::test_metal_device_creation`
- `whisper_candle.rs::test_parse_device_metal`
- Full pipeline tests with Metal device

## End-to-End Integration Tests

These tests exercise the complete pipeline from audio input to transcription output.

### Recommended Test Audio
Place test audio files in `tests/fixtures/audio/`:

1. **silence.wav** - 5 seconds of silence (should transcribe to empty or "[SILENCE]")
2. **speech.wav** - Clear speech: "The quick brown fox jumps over the lazy dog"
3. **noise.wav** - Background noise without speech (test no_speech_prob)
4. **multilingual.wav** - Non-English speech (test language detection)

### Running Integration Tests
```bash
# Basic integration test (CPU only)
cargo test --features whisper integration -- --nocapture

# GPU integration tests
WHISPER_DEVICE=cuda:0 cargo test --features whisper integration --ignored -- --nocapture
```

## Performance Benchmarks

These are not automated tests but manual benchmarks to measure performance.

### Benchmark 1: Model Loading Time
```bash
time cargo run --features whisper --example load_model
```
**Expected:**
- Tiny model: < 1 second
- Base model: 1-2 seconds
- Large model: 3-5 seconds

### Benchmark 2: Transcription Speed
```bash
cargo run --features whisper --example benchmark_transcription
```
**Expected (CPU, Base model):**
- Real-time factor: 0.1-0.3x (10-30 seconds to transcribe 100 seconds of audio)
- GPU: 0.01-0.05x (1-5 seconds for 100 seconds of audio)

### Benchmark 3: Memory Usage
```bash
/usr/bin/time -v cargo run --features whisper --example memory_test
```
**Expected peak memory:**
- Tiny: 150-200 MB
- Base: 300-400 MB
- Medium: 1.5-2 GB
- Large: 3-4 GB

## Test Data Requirements

### Minimal Test Setup
For basic testing, download the Whisper Tiny model:

```bash
# Create fixtures directory
mkdir -p tests/fixtures/whisper-tiny

# Download from HuggingFace (requires git-lfs or manual download)
cd tests/fixtures/whisper-tiny
wget https://huggingface.co/openai/whisper-tiny/resolve/main/model.safetensors
wget https://huggingface.co/openai/whisper-tiny/resolve/main/config.json
wget https://huggingface.co/openai/whisper-tiny/resolve/main/tokenizer.json
```

### Full Test Suite Setup
For comprehensive testing:

```bash
# Download multiple models
models=("tiny" "base" "small")
for model in "${models[@]}"; do
    mkdir -p tests/fixtures/whisper-$model
    cd tests/fixtures/whisper-$model
    # Download model, config, tokenizer as above
    cd ../../..
done

# Create test audio (requires ffmpeg)
mkdir -p tests/fixtures/audio
# Generate 5 seconds of silence at 16kHz mono
ffmpeg -f lavfi -i anullsrc=r=16000:cl=mono -t 5 tests/fixtures/audio/silence.wav
```

## CI Configuration Notes

The current CI setup should:
- ✅ Run all unit tests with `cargo test`
- ✅ Run with `--features whisper` to test feature-gated code
- ❌ **Do NOT** run tests marked with `#[ignore]` (they require model files)
- ❌ **Do NOT** download models in CI (too slow, too large)

Example CI command:
```bash
cargo test --features whisper --workspace -- --test-threads=1
```

## Known Limitations

1. **No GPU in CI**: GitHub Actions runners don't have CUDA/Metal GPUs
   - Solution: GPU tests must be run manually on developer machines

2. **Large model files**: Cannot store in git repository
   - Solution: Provide download scripts, document where to get models

3. **Audio fixtures**: Adding binary WAV files increases repo size
   - Solution: Generate simple test audio programmatically where possible

4. **Numerical precision**: Some tests may fail on different architectures
   - Solution: Use epsilon comparisons for floating-point tests

## Future Improvements

1. **Mock model loader**: Create a minimal fake model for testing API without real models
2. **Audio generators**: Programmatically generate test audio (sine waves, noise)
3. **Snapshot testing**: Record expected outputs and compare against them
4. **Fuzzing**: Use cargo-fuzz to test edge cases in audio processing
5. **Property-based testing**: Use proptest for timestamp conversion logic

## Summary

**Tests that run automatically in CI:**
- ~50+ unit tests across all modules
- Focus on logic, error handling, edge cases
- No external dependencies (models, network, GPU)

**Tests that require manual setup:**
- ~10 integration tests requiring model files
- ~5 GPU-specific tests
- 1 network download test
- ~3 performance benchmarks

**Total test coverage:**
- Unit tests: ~80% of code paths
- Integration tests: ~40% (requires manual setup)
- GPU code paths: Minimal (manual testing only)
