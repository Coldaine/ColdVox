---
doc_type: reference
subsystem: foundation
status: draft
freshness: stale
preservation: preserve
domain_code: fdn
last_reviewed: 2025-10-19
owners: Documentation Working Group
version: 1.0.0
---

# Testing Guide

## Overview

ColdVox has a comprehensive test suite that tests real STT functionality using modern speech recognition models and actual hardware. Tests are designed to work with actual speech recognition and real audio devices rather than mocks to ensure functional correctness. This guide explains how to run tests and set up the required dependencies.

## Test Categories

### Core Tests
**All tests use real STT models and hardware for functional validation**

- ✅ **Test actual STT functionality** (use real Moonshine or Parakeet models)
- ✅ **Validate end-to-end pipeline behavior**
- ✅ **Test with real audio hardware** (microphones, speakers)
- ✅ **Require STT model setup** (see setup section below)

```bash
# Run tests with Moonshine (CPU-efficient)
cargo test

# Run tests for specific crate
cargo test -p coldvox-app

# Run with Parakeet (GPU-accelerated)
cargo test --features parakeet
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

All tests use real STT models and actual audio hardware to validate functionality. This includes development environments and self-hosted CI runners.

### Required Setup

#### 1. STT Model Setup
Tests support multiple STT backends. Choose based on your environment:

**Option A: Moonshine (CPU-efficient, recommended for most users)**
```bash
# Moonshine models are auto-downloaded on first use
# No manual setup required - the plugin handles model initialization
cargo test
```

**Option B: Parakeet (GPU-accelerated)**
```bash
# Requires CUDA/GPU support
cargo test --features parakeet
```

**Option C: Mock (testing/development)**
```bash
# For testing without actual models
cargo test --features mock
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
| `coldvox-app` | Plugin management, STT logic | End-to-end WAV processing | STT model required for all tests |
| `coldvox-vad` | VAD algorithms | Real audio processing | Silero ONNX models tested |
| `coldvox-stt` | Plugin interfaces | Model loading/inference | Real hardware and models used |

### By Feature

```bash
# Audio tests (with real hardware)
cargo test -p coldvox-audio

# STT tests (with available models)
cargo test -p coldvox-app stt --lib

# Text injection tests (with real injection)
cargo test -p coldvox-app --features text-injection injection

# VAD tests (with real audio processing)
cargo test -p coldvox-vad

# Full pipeline with Moonshine (CPU)
cargo test -p coldvox-app test_end_to_end_wav

# Full pipeline with Parakeet (GPU)
cargo test -p coldvox-app test_end_to_end_wav --features parakeet
```

## Key Testing Principles

### Real Hardware Testing
- **Use real hardware**: All tests run against actual audio devices and STT models
- **No mock-only paths**: If mocks are used for unit testing, full real tests must be included in the same test run
- **Comprehensive**: Test actual functionality end-to-end with real hardware and models
- **Reliable**: Target hardware is consistently available across environments

### Test Design
- **No ignored tests**: All tests run by default in standard test execution
- **Real dependencies**: Use actual STT models and audio hardware for validation
- **Full validation**: Test complete pipeline from audio capture to text injection
- **Mock + Real requirement**: Any test suite using mocks must also include corresponding real tests

## Common Issues & Solutions

### "Failed to initialize STT plugin" Errors
```bash
# Moonshine: Models auto-download on first use (may take a moment)
# Parakeet: Requires GPU/CUDA support available
# Mock: Use --features mock for testing without models

# Verify your setup
cargo run --bin mic_probe
cargo test -p coldvox-audio  # Test audio independently first
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

1. Ensure STT models are available (Moonshine auto-downloads, or select appropriate feature)
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

# STT-specific tests
cargo test                                   # Default: Moonshine
cargo test --features parakeet               # GPU: Parakeet
cargo test --features mock                   # Testing: Mock plugin

# Specific test patterns
cargo test plugin_manager                    # Plugin management tests
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
- Models are automatically downloaded when needed (Moonshine)

## STT Plugin Selection

| Plugin | Use Case | Requirements |
|--------|----------|--------------|
| **Moonshine** | Production, CPU | Pure Rust, auto-downloads models |
| **Parakeet** | High-quality, GPU | CUDA/GPU support required |
| **Mock** | Testing, CI | No external dependencies |
| **NoOp** | Debug, validation | Returns empty transcripts |
