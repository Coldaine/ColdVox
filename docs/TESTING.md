# Testing Guide

## Overview

ColdVox has a comprehensive test suite that tests real STT functionality using Vosk models and actual hardware. Tests are designed to work with actual speech recognition and real audio devices rather than mocks to ensure functional correctness. This guide explains how to run tests and set up the required dependencies.

## Test Categories

### Core Tests
**All tests use real Vosk models and hardware for functional validation**

- ✅ **Test actual STT functionality** (use real Vosk models)
- ✅ **Validate end-to-end pipeline behavior**
- ✅ **Test with real audio hardware** (microphones, speakers)
- ✅ **Require Vosk model setup** (see setup section below)

```bash
# Run tests with Vosk model (all environments)
VOSK_MODEL_PATH="$(pwd)/models/vosk-model-small-en-us-0.15" cargo test

# Run tests for specific crate
VOSK_MODEL_PATH="$(pwd)/models/vosk-model-small-en-us-0.15" cargo test -p coldvox-app
```

### Integration Tests (Full Hardware & Models)
**All tests run by default - no tests are ignored**

- ✅ **Use real external models/hardware**
- ✅ **Comprehensive end-to-end validation**
- ✅ **Run in all environments** (dev, self-hosted CI)
- ✅ **Test real-world functionality end-to-end**

```bash
# Run all tests (includes integration tests)
cargo test

# Run specific integration test
cargo test test_end_to_end_wav_pipeline --nocapture

# Run with real hardware (available in all environments)
cargo test test_candidate_order_default_first
```

## Environment Setup

### Hardware Requirements
**All environments must have real hardware available**

All tests use real Vosk models and actual audio hardware to validate functionality. This includes development environments and self-hosted CI runners.

### Required Setup

#### 1. Vosk Model Setup
All tests require a real Vosk model for STT functionality:

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
**Real audio hardware is required and available in all environments:**

```bash
# Check available devices
cargo run --bin mic_probe

# All environments have working audio devices
# No mocking or headless overrides are used
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
| `coldvox-audio` | Device enumeration, resampling | Real hardware detection | All tests run with real devices |
| `coldvox-app` | Plugin management, STT logic | End-to-end WAV processing | Vosk model required for all tests |
| `coldvox-vad` | VAD algorithms | Real audio processing | Silero ONNX models tested |
| `coldvox-stt` | Plugin interfaces | Model loading/inference | Real hardware and models used |

### By Feature

```bash
# Audio tests (with real hardware)
cargo test -p coldvox-audio

# STT tests (with real Vosk models)
cargo test -p coldvox-app stt --lib

# Text injection tests (with real injection)
cargo test -p coldvox-app --features text-injection injection

# VAD tests (with real audio processing)
cargo test -p coldvox-vad

# Full pipeline with real models and hardware
cargo test -p coldvox-app test_end_to_end_wav --features vosk
```

## Nextest (Advanced Test Runner)

Cargo-Nextest provides faster, clearer test runs with better flake handling for the ColdVox workspace:

- **Faster execution**: Parallel test running with intelligent scheduling
- **Better output**: Clearer failure reporting and progress tracking
- **Flake detection**: Identifies and handles flaky tests with retries
- **Self-hosted CI compatibility**: Works with real hardware testing

### Installation
```bash
cargo install cargo-nextest --locked
```

### Usage Examples
```bash
# Run all workspace tests with nextest
cargo nextest run --workspace --locked

# Run tests for a specific crate
cargo nextest run -p coldvox-audio

# Run with Vosk model (if available)
VOSK_MODEL_PATH="$(pwd)/models/vosk-model-small-en-us-0.15" cargo nextest run --workspace --locked

# Run with retries for flaky tests
cargo nextest run --workspace --retries 2

# Limit parallelism for sensitive tests
cargo nextest run --test-threads 1
```

### Configuration
Create `.config/nextest.toml` for persistent settings:
```toml
[profile.default]
retries = 2
fail-fast = false
status-level = "failures"
final-status-level = "flaky"
slow-timeout = { period = "120s", terminate-after = 2 }
```

## Coverage Analysis (Tarpaulin)

Cargo-Tarpaulin measures code coverage to identify gaps in testing:

- **Line/branch coverage**: Detailed analysis of tested code paths
- **HTML/LCOV reports**: Visual and machine-readable output formats
- **Linux self-hosted only**: Requires ptrace capabilities
- **Core crates focus**: Initially excludes GUI and text injection for stability

### Installation
```bash
cargo install cargo-tarpaulin --locked
```

### Usage Examples
```bash
# Run coverage for a specific crate with Vosk features
cargo tarpaulin -p coldvox-stt --features vosk --out Html --output-dir coverage

# Run coverage for core crates (recommended)
cargo tarpaulin \
  --packages coldvox-foundation coldvox-telemetry coldvox-audio coldvox-vad coldvox-vad-silero coldvox-stt coldvox-stt-vosk \
  --features vosk \
  --exclude coldvox-gui --exclude coldvox-text-injection \
  --out Html --out Lcov --output-dir coverage

# Open HTML report
open coverage/tarpaulin-report.html
```

### Coverage Goals
- **Core crates**: >80% line coverage
- **Foundation/Telemetry**: >90% line coverage
- **Audio/VAD/STT**: >85% line coverage
- **GUI/Text Injection**: Evaluate separately due to platform dependencies

## Key Testing Principles

### Real Hardware Testing
- **Use real hardware**: All tests run against actual audio devices and Vosk models
- **No mock-only paths**: If mocks are used for unit testing, full real tests must be included in the same test run
- **Comprehensive**: Test actual functionality end-to-end with real hardware and models
- **Reliable**: Target hardware is consistently available across environments

### Test Design
- **No ignored tests**: All tests run by default in standard test execution
- **Real dependencies**: Use actual Vosk models and audio hardware for validation
- **Full validation**: Test complete pipeline from audio capture to text injection
- **Mock + Real requirement**: Any test suite using mocks must also include corresponding real tests

## Common Issues & Solutions

### "Failed to locate Vosk model" Errors
```bash
# Fix: Set up Vosk model
export VOSK_MODEL_PATH="/path/to/vosk-model-small-en-us-0.15"
# OR
./scripts/ci/setup-vosk-cache.sh
```

### Audio Device Tests
```bash
# Check available devices (should always have devices in all environments)
cargo run --bin mic_probe

# All tests run against real hardware
cargo test
```

### Test Execution
All tests are designed to run with real hardware and models:

1. Ensure Vosk model is available at `VOSK_MODEL_PATH`
2. Verify audio hardware is accessible via `mic_probe`
3. All tests run by default - no tests should be ignored

### Permission Errors (Linux)
```bash
# Fix uinput permissions for text injection
sudo usermod -a -G input $USER
sudo chmod 666 /dev/uinput
```

## Test Commands Reference

```bash
# Development workflow (all tests with real hardware)
cargo test                                    # All tests including integration
cargo check --all-targets                    # Quick compile check
cargo test --workspace                       # All crates with real hardware

# Advanced testing with nextest
just test-nextest                            # Run all tests with nextest
cargo nextest run --workspace --locked       # Direct nextest command

# Coverage analysis
just test-coverage                           # Run coverage for core crates
cargo tarpaulin -p coldvox-audio             # Coverage for specific crate

# Environment setup
./scripts/ci/setup-vosk-cache.sh            # Setup models for all tests
export VOSK_MODEL_PATH="$(pwd)/models/vosk-model-small-en-us-0.15"

# Specific test patterns
cargo test plugin_manager                    # Plugin management tests
cargo test --features vosk test_vosk         # Vosk-specific tests
cargo test audio_device                      # Audio hardware tests (real devices)

# Debug failing tests
cargo test failing_test_name -- --nocapture  # Show full output
RUST_LOG=debug cargo test test_name          # Enable debug logging
```

## Continuous Integration

**Self-hosted runners with real hardware:**
- ✅ All tests run with real audio devices and models
- ✅ No tests are ignored or skipped
- ✅ Full hardware validation in CI environment
- ✅ Compilation checks for all feature combinations

**Local development:**
- Run `cargo test` for complete validation (includes all tests)
- All tests use real hardware and models
- Use `scripts/ci/setup-vosk-cache.sh` for model setup