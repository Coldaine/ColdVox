# Testing Guide

## Overview

ColdVox has a comprehensive test suite that tests real STT functionality using Vosk models. Tests are designed to work with actual speech recognition rather than mocks to ensure functional correctness. This guide explains how to run tests and set up the required dependencies.

## Test Categories

### Core Tests
**Most tests use real Vosk models for functional validation**

- ✅ **Test actual STT functionality** (use real Vosk models)
- ✅ **Validate end-to-end pipeline behavior**
- ⚠️ **Require Vosk model setup** (see setup section below)

```bash
# Run tests with Vosk model (recommended)
VOSK_MODEL_PATH="$(pwd)/models/vosk-model-small-en-us-0.15" cargo test

# Run tests for specific crate
VOSK_MODEL_PATH="$(pwd)/models/vosk-model-small-en-us-0.15" cargo test -p coldvox-app
```

### Integration Tests (Require Setup)
**Marked with `#[ignore]` - must be explicitly enabled**

- ⚠️ **Require external models/hardware**
- ⚠️ **Slower execution** (download models, audio processing)
- ⚠️ **May fail in headless/CI environments**
- ✅ **Test real-world functionality end-to-end**

```bash
# Run integration tests (requires Vosk model setup)
cargo test -- --ignored

# Run specific integration test
cargo test test_end_to_end_wav_pipeline -- --ignored --nocapture

# Run with real hardware (non-headless only)
cargo test test_candidate_order_default_first -- --ignored
```

## Environment Setup

### For Core Tests
**Requires Vosk model setup** - see instructions below

Tests use real Vosk models to validate actual speech recognition functionality.

### For Integration Tests

#### 1. Vosk Model Setup
Integration tests require a real Vosk model for STT functionality:

```bash
# Option A: Use the automated setup script
./scripts/ci/setup-vosk-cache.sh

# Option B: Manual setup
# 1. Download a Vosk model
wget https://alphacephei.com/vosk/models/vosk-model-small-en-us-0.15.zip
unzip vosk-model-small-en-us-0.15.zip
mkdir -p models/
mv vosk-model-small-en-us-0.15 models/

# 2. Set environment variable
export VOSK_MODEL_PATH="$(pwd)/models/vosk-model-small-en-us-0.15"
```

#### 2. Audio Hardware Setup
Some tests require actual audio input devices:

```bash
# Check available devices
cargo run --bin mic_probe

# Override headless detection if needed
export COLDVOX_AUDIO_FORCE_NON_HEADLESS=true  # Force hardware tests
export COLDVOX_AUDIO_FORCE_HEADLESS=true      # Skip hardware tests
```

#### 3. Text Injection Setup (Linux)
For text injection integration tests:

```bash
# Install dependencies
./scripts/setup_text_injection.sh

# Ensure proper permissions for uinput devices
sudo usermod -a -G input $USER
# Log out and back in after group change
```

## Test Organization

### By Crate

| Crate | Unit Tests | Integration Tests | Notes |
|-------|------------|-------------------|-------|
| `coldvox-audio` | Device enumeration, resampling | Real hardware detection | Some tests auto-skip in headless |
| `coldvox-app` | Plugin management, STT logic | End-to-end WAV processing | Vosk model required for integration |
| `coldvox-vad` | VAD algorithms | Real audio processing | Silero ONNX models tested |
| `coldvox-stt` | Plugin interfaces, mocking | Model loading/inference | Mock vs real plugin separation |

### By Feature

```bash
# Audio-only tests
cargo test -p coldvox-audio

# STT tests (unit only, no real models)
cargo test -p coldvox-app stt --lib

# Text injection tests
cargo test -p coldvox-app --features text-injection injection

# VAD tests
cargo test -p coldvox-vad

# Integration: Full pipeline with real models
cargo test -p coldvox-app test_end_to_end_wav -- --ignored --features vosk
```

## Key Testing Principles

### Unit Test Design
- **Use mock plugins**: `configure_for_testing()` helper ensures tests use `MockPlugin` instead of real Vosk
- **Deterministic**: Set `MOCK_PACTL_OUTPUT` and `MOCK_APLAY_OUTPUT` for audio detection tests
- **Fast**: Target <1s per test, <10s total suite
- **Isolated**: No shared state, no external dependencies

### Integration Test Design
- **Marked with `#[ignore]`**: Prevents accidental execution in CI
- **Clear requirements**: Document what models/hardware are needed
- **Graceful degradation**: Auto-skip when dependencies unavailable
- **Meaningful errors**: Guide users to setup instructions when tests fail

## Common Issues & Solutions

### "Failed to locate Vosk model" Errors
```bash
# Fix: Set up Vosk model
export VOSK_MODEL_PATH="/path/to/vosk-model-small-en-us-0.15"
# OR
./scripts/ci/setup-vosk-cache.sh
```

### Audio Device Tests Failing
```bash
# Check if headless environment
cargo run --bin mic_probe

# Force skip hardware-dependent tests
export COLDVOX_AUDIO_FORCE_HEADLESS=true
cargo test
```

### Test Uses Real STT Instead of Mock
If a unit test is accidentally trying to load real models:

1. Check if test uses `configure_for_testing()` helper
2. Verify test is not marked with `#[ignore]`
3. Ensure test creates manager with `create_test_manager()`

### Permission Errors (Linux)
```bash
# Fix uinput permissions for text injection
sudo usermod -a -G input $USER
sudo chmod 666 /dev/uinput
```

## Test Commands Reference

```bash
# Development workflow (unit tests only)
cargo test                                    # All unit tests
cargo check --all-targets                    # Quick compile check
cargo test --workspace                       # All crates unit tests

# Full validation (with integration tests)
./scripts/ci/setup-vosk-cache.sh            # Setup models
cargo test -- --ignored                      # Run integration tests
cargo test --workspace -- --ignored          # All integration tests

# Specific test patterns
cargo test plugin_manager                    # Plugin management tests
cargo test --features vosk test_vosk         # Vosk-specific tests
cargo test audio_device -- --ignored         # Audio hardware tests

# Debug failing tests
cargo test failing_test_name -- --nocapture  # Show full output
RUST_LOG=debug cargo test test_name          # Enable debug logging
```

## Continuous Integration

**GitHub Actions runs:**
- ✅ All unit tests (fast, always enabled)
- ⚠️ Integration tests only on self-hosted runners with models pre-cached
- ✅ Compilation checks for all feature combinations

**Local development:**
- Run `cargo test` frequently (unit tests only)
- Run `cargo test -- --ignored` before major releases (integration tests)
- Use `scripts/ci/setup-vosk-cache.sh` for full local validation