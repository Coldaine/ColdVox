<!-- Archived from docs/2_audio_capture/Phase1_Test_Suite_Design.md on 2025-08-26 -->
# Phase 1 Test Suite Design

## Overview
This document outlines the comprehensive test suite design for Phase 1 (Microphone Capture with Recovery) of the ColdVox project. The test suite ensures all audio capture, device management, and recovery mechanisms work reliably.

## Test Architecture

### Test Organization
```
crates/app/
├── src/
│   └── audio/
│       ├── capture.rs
│       ├── device.rs
│       ├── watchdog.rs
│       └── detector.rs
└── tests/
	├── unit/
	│   ├── audio_capture_test.rs
	│   ├── device_manager_test.rs
	│   ├── watchdog_test.rs
	│   └── silence_detector_test.rs
	├── integration/
	│   ├── capture_integration_test.rs
	│   ├── recovery_test.rs
	│   └── buffer_overflow_test.rs
	└── common/
		├── mod.rs
		└── test_utils.rs
```

## Unit Tests

### 1. AudioCapture Unit Tests (`audio_capture_test.rs`)

#### Test Cases
```rust
#[cfg(test)]
mod tests {
	use super::*;
	use mockall::*;

	#[test]
	fn test_audio_frame_creation() {
		// Test AudioFrame struct creation with various sample rates
	}

	#[test]
	fn test_capture_stats_initialization() {
		// Verify CaptureStats starts with zero counters
	}

	#[test]
	fn test_capture_stats_atomic_operations() {
		// Test thread-safe counter increments
	}

	#[test]
	fn test_sample_buffer_bounds() {
		// Verify 100-frame buffer limit behavior
	}

	#[test]
	fn test_format_conversion_i16() {
		// Test identity conversion for i16
	}

	#[test]
	fn test_format_conversion_f32() {
		// Test f32 to i16 conversion with scaling
	}

	#[test]
	fn test_format_conversion_u16() {
		// Test u16 to i16 conversion with offset
	}

	#[test]
	fn test_format_conversion_u8() {
		// Test u8 to i16 conversion with scaling and offset
	}

	#[test]
	fn test_format_conversion_i8() {
		// Test i8 to i16 conversion with scaling
	}

	#[test]
	fn test_stereo_to_mono_downmix() {
		// Test averaging of stereo channels
	}

	#[test]
	fn test_multichannel_downmix() {
		// Test 5.1/7.1 to mono conversion
	}

	#[test]
	fn test_resample_48khz_to_16khz() {
		// Test fractional-phase resampling
	}

	#[test]
	fn test_resample_44100_to_16khz() {
		// Test non-integer ratio resampling
	}
}
```

### 2. DeviceManager Unit Tests (`device_manager_test.rs`)

```rust
#[cfg(test)]
mod tests {
	use super::*;
	use mockall::*;

	// Mock CPAL traits
	mock! {
		Device {}
		impl DeviceTrait for Device {
			fn name(&self) -> Result<String, DeviceNameError>;
			fn supported_input_configs(&self) -> Result<SupportedInputConfigs, SupportedStreamConfigsError>;
			fn default_input_config(&self) -> Result<SupportedStreamConfig, DefaultStreamConfigError>;
		}
	}

	#[test]
	fn test_device_enumeration() {
		// Test listing all available devices
	}

	#[test]
	fn test_pipewire_preference() {
		// Verify pipewire is selected when available
	}

	#[test]
	fn test_hardware_preference_order() {
		// Test HyperX/QuadCast preference over generic devices
	}

	#[test]
	fn test_exact_name_match() {
		// Test exact device name matching
	}

	#[test]
	fn test_case_insensitive_match() {
		// Test fallback to case-insensitive substring match
	}

	#[test]
	fn test_device_not_found_error() {
		// Test error when specified device doesn't exist
	}

	#[test]
	fn test_default_device_fallback() {
		// Test fallback to OS default when no preferences match
	}

	#[test]
	fn test_supported_configs_enumeration() {
		// Test getting supported formats for a device
	}
}
```

### 3. WatchdogTimer Unit Tests (`watchdog_test.rs`)

```rust
#[cfg(test)]
mod tests {
	use super::*;
	use std::time::Duration;
	use std::sync::mpsc;

	#[test]
	fn test_watchdog_creation() {
		// Test watchdog with various timeout durations
	}

	#[test]
	fn test_watchdog_pet() {
		// Test that petting prevents timeout
	}

	#[test]
	fn test_watchdog_timeout() {
		// Test timeout after specified duration
	}

	#[test]
	fn test_watchdog_stop() {
		// Test clean watchdog shutdown
	}

	#[test]
	fn test_epoch_change() {
		// Test that epoch changes on stop/start cycles
	}

	#[test]
	fn test_concurrent_pet_operations() {
		// Test thread-safe petting from multiple threads
	}

	#[test]
	fn test_timeout_callback_execution() {
		// Test callback is called exactly once on timeout
	}
}
```

### 4. SilenceDetector Unit Tests (`silence_detector_test.rs`)

```rust
#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_rms_calculation() {
		// Test RMS calculation accuracy
	}

	#[test]
	fn test_silence_threshold_50() {
		// Test detection with threshold=50
	}

	#[test]
	fn test_silence_threshold_500() {
		// Test detection with threshold=500
	}

	#[test]
	fn test_continuous_silence_tracking() {
		// Test 3-second silence warning trigger
	}

	#[test]
	fn test_activity_interrupts_silence() {
		// Test that activity resets silence counter
	}

	#[test]
	fn test_edge_cases() {
		// Test empty samples, single sample, max values
	}
}
```

## Integration Tests

### 1. Capture Integration Tests (`capture_integration_test.rs`)

```rust
#[cfg(test)]
mod tests {
	use coldvox_app::audio::*;
	use std::time::Duration;
	use std::thread;

	#[test]
	fn test_end_to_end_capture_pipewire() {
		// Test full capture pipeline with pipewire
	}

	#[test]
	fn test_stats_reporting() {
		// Verify stats update correctly during capture
	}

	#[test]
	fn test_frame_flow() {
		// Test frames flow from device to consumer
	}

	#[test]
	fn test_clean_shutdown() {
		// Test Ctrl+C handling and resource cleanup
	}

	#[test]
	fn test_concurrent_operations() {
		// Test capture with multiple consumers
	}
}
```

### 2. Recovery Integration Tests (`recovery_test.rs`)

```rust
#[cfg(test)]
mod tests {
	use coldvox_app::audio::*;
	use std::sync::Arc;
	use std::sync::atomic::{AtomicBool, Ordering};

	#[test]
	fn test_watchdog_triggered_recovery() {
		// Simulate no-data timeout and recovery
	}

	#[test]
	fn test_exponential_backoff() {
		// Test retry delays: 1s, 2s, 4s
	}

	#[test]
	fn test_max_retry_attempts() {
		// Test failure after 3 attempts
	}

	#[test]
	fn test_recovery_with_jitter() {
		// Verify jitter is applied to retry delays
	}

	#[test]
	fn test_disconnect_reconnect_counters() {
		// Verify disconnection/reconnection stats
	}
}
```

### 3. Buffer Overflow Tests (`buffer_overflow_test.rs`)

```rust
#[cfg(test)]
mod tests {
	use coldvox_app::audio::*;
	use std::thread;
	use std::time::Duration;

	#[test]
	fn test_buffer_overflow_drop_oldest() {
		// Test DropOldest policy
	}

	#[test]
	fn test_buffer_overflow_drop_newest() {
		// Test DropNewest policy (default)
	}

	#[test]
	fn test_frame_dropped_counter() {
		// Verify frames_dropped increments correctly
	}

	#[test]
	fn test_buffer_recovery() {
		// Test buffer recovers after consumer resumes
	}
}
```

## Test Infrastructure

### Test Utilities (`common/test_utils.rs`)

```rust
pub mod test_utils {
	use std::sync::Arc;
	use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
	use cpal::{SampleFormat, StreamConfig};

	/// Generate test audio samples
	pub fn generate_sine_wave(freq: f32, sample_rate: u32, duration_ms: u32) -> Vec<i16> {
		// Generate sine wave test data
	}

	/// Generate silent samples
	pub fn generate_silence(sample_count: usize) -> Vec<i16> {
		vec![0; sample_count]
	}

	/// Mock audio device for testing
	pub struct MockAudioDevice {
		pub name: String,
		pub format: SampleFormat,
		pub channels: u16,
		pub sample_rate: u32,
	}

	/// Test harness for capture testing
	pub struct CaptureTestHarness {
		capture: AudioCapture,
		control_flag: Arc<AtomicBool>,
	}

	impl CaptureTestHarness {
		pub fn new() -> Self {
			// Setup test harness
		}

		pub fn simulate_frames(&mut self, count: usize) {
			// Simulate frame generation
		}

		pub fn simulate_silence(&mut self, duration: Duration) {
			// Simulate silence period
		}

		pub fn simulate_disconnect(&mut self) {
			// Simulate device disconnect
		}
	}

	/// Verify timing constraints
	pub fn assert_duration_within(actual: Duration, expected: Duration, tolerance: Duration) {
		let diff = if actual > expected {
			actual - expected
		} else {
			expected - actual
		};
		assert!(diff <= tolerance, 
			"Duration {:?} not within {:?} of expected {:?}", 
			actual, tolerance, expected);
	}
}
```

## Test Execution Strategy

### 1. Unit Test Execution
```bash
# Run all unit tests
cargo test --lib

# Run specific module tests
cargo test audio::capture::tests

# Run with output
cargo test -- --nocapture

# Run with specific test filter
cargo test test_format_conversion
```

### 2. Integration Test Execution
```bash
# Run all integration tests
cargo test --test '*'

# Run specific integration test
cargo test --test capture_integration_test

# Run with longer timeout for recovery tests
cargo test --test recovery_test -- --test-threads=1
```

### 3. Manual Test Verification
```bash
# Use existing mic_probe for manual testing
cargo run --bin mic_probe -- --duration 30 --silence-threshold 100

# Test with expect_disconnect for recovery
cargo run --bin mic_probe -- --expect_disconnect --duration 60
```

## Mock Strategy

### Device Mocking
```rust
use mockall::predicate::*;

fn create_mock_device(name: &str, format: SampleFormat) -> MockDevice {
	let mut mock = MockDevice::new();
	mock.expect_name()
		.returning(move || Ok(name.to_string()));
	mock.expect_default_input_config()
		.returning(move || Ok(create_test_config(format)));
	mock
}
```

### Stream Mocking
```rust
fn create_mock_stream() -> MockStream {
	let mut mock = MockStream::new();
	mock.expect_play()
		.returning(|| Ok(()));
	mock.expect_pause()
		.returning(|| Ok(()));
	mock
}
```

## Test Data

### Sample Test Files
- `test_data/sine_440hz_16khz.raw`: 440Hz sine wave
- `test_data/silence_16khz.raw`: Silent audio
- `test_data/speech_16khz.raw`: Sample speech
- `test_data/noise_16khz.raw`: Background noise

## Coverage Goals

### Target Coverage
- Unit Tests: 80% line coverage
- Integration Tests: Critical paths covered
- Manual Tests: Device-specific scenarios

### Coverage Measurement
```bash
# Install cargo-tarpaulin
cargo install cargo-tarpaulin

# Run coverage
cargo tarpaulin --out Html --output-dir coverage
```

## Continuous Integration

### GitHub Actions Workflow
```yaml
name: Phase 1 Tests

on: [push, pull_request]

jobs:
  test:
	runs-on: ubuntu-latest
	steps:
	  - uses: actions/checkout@v2
	  - uses: actions-rs/toolchain@v1
		with:
		  toolchain: stable
	  - name: Install ALSA dev
		run: sudo apt-get install libasound2-dev
	  - name: Run tests
		run: cargo test --all-features
	  - name: Check formatting
		run: cargo fmt -- --check
	  - name: Run clippy
		run: cargo clippy -- -D warnings
```

## Test Maintenance

### Best Practices
1. Keep tests independent and isolated
2. Use descriptive test names
3. Test both success and failure paths
4. Mock external dependencies
5. Use property-based testing for complex scenarios
6. Keep test data minimal but representative
7. Document non-obvious test scenarios

### Test Review Checklist
- [ ] Tests cover all public APIs
- [ ] Error conditions are tested
- [ ] Edge cases are covered
- [ ] Tests are deterministic
- [ ] Tests run quickly (<100ms each)
- [ ] Mock usage is appropriate
- [ ] Test names clearly describe scenario

## Next Steps

1. Implement test infrastructure utilities
2. Create mock implementations
3. Write unit tests for each module
4. Implement integration tests
5. Set up CI/CD pipeline
6. Generate initial coverage report
7. Address coverage gaps
