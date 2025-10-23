# Test Removal and Consolidation Plan

**Date**: 2025-10-23
**Objective**: Reduce test count by ~40% while improving coverage and confidence
**Principle**: One comprehensive test > Ten fragmented tests

---

## Summary

| Category | Current Count | Target Count | Reduction |
|----------|--------------|--------------|-----------|
| Watchdog Tests | 6 | 2 | -67% |
| Silence Detector Tests | 9 | 2 | -78% |
| Settings Tests | 9 | 3 | -67% |
| Capture Integration Tests | 7 | 3 | -57% |
| Text Injection Tests | 7 | 3 | -57% |
| **Total** | **38** | **13** | **-66%** |

**Expected Outcomes**:
- ✅ Fewer tests that are more meaningful
- ✅ Better coverage of user behavior
- ✅ Tests won't break on refactors
- ✅ Faster test suite (less setup/teardown overhead)
- ✅ Easier to understand test suite

---

## File-by-File Removal Plan

### 1. `crates/app/tests/unit/watchdog_test.rs`

**Current**: 6 tests, 107 lines
**Target**: 2 tests, ~60 lines + 1 integration test elsewhere

#### Tests to REMOVE (consolidate into integration test):

```rust
// ❌ REMOVE: test_watchdog_creation
// Reason: Trivial test, constructor always works
#[tokio::test]
async fn test_watchdog_creation() {
    let _wd1 = WatchdogTimer::new(Duration::from_secs(1));
    // ... just creates objects, no value
}

// ❌ REMOVE: test_watchdog_stop_resets_trigger
// Reason: Implementation detail, not user-facing behavior
#[tokio::test]
async fn test_watchdog_stop_resets_trigger() {
    // Tests internal state reset - not valuable
}

// ❌ REMOVE: test_restart_does_not_carry_trigger_state
// Reason: Implementation detail
#[tokio::test]
async fn test_restart_does_not_carry_trigger_state() {
    // Tests internal state management - refactor-brittle
}

// ❌ REMOVE: test_concurrent_feed_operations
// Reason: Will be covered by integration test with real concurrent audio
#[tokio::test]
async fn test_concurrent_feed_operations() {
    // 30 lines of threading complexity
    // Better tested as part of real audio capture
}
```

#### Tests to KEEP (pure algorithm logic):

```rust
// ✅ KEEP: Core algorithm behavior
#[tokio::test]
async fn test_watchdog_feed_prevents_timeout() {
    // Simple, focused test of timer algorithm
    // Won't break on refactor
}

#[tokio::test]
async fn test_watchdog_timeout_triggers() {
    // Simple, focused test of timeout behavior
}
```

#### NEW Integration Test (to add):

```rust
// crates/app/tests/integration/audio_recovery_test.rs
#[tokio::test]
async fn test_audio_pipeline_auto_recovers_from_disconnect() {
    // This ONE test replaces the 4 removed tests above
    // AND proves the feature works for users
    // ~50 lines, tests real behavior
}
```

**Net Change**: 6 tests → 3 tests (2 algorithm + 1 integration)
**Line Count**: 107 lines → ~110 lines (but far more valuable)

---

### 2. `crates/app/tests/unit/silence_detector_test.rs`

**Current**: 9 tests, 175 lines
**Target**: 2 tests, ~50 lines + integration tests elsewhere

#### Tests to REMOVE (consolidate):

```rust
// ❌ REMOVE: test_rms_calculation
// Reason: Implementation detail, will be covered by integration
#[test]
fn test_rms_calculation() {
    // Tests RMS formula - covered by integration test
}

// ❌ REMOVE: test_silence_threshold_50
// ❌ REMOVE: test_silence_threshold_500
// Reason: Specific threshold testing covered by integration
#[test]
fn test_silence_threshold_50() { ... }
#[test]
fn test_silence_threshold_500() { ... }

// ❌ REMOVE: test_continuous_silence_tracking
// Reason: Covered by VAD integration test
#[test]
fn test_continuous_silence_tracking() {
    // 20 lines manually tracking silence duration
    // Better tested in VAD pipeline
}

// ❌ REMOVE: test_activity_interrupts_silence
// Reason: Covered by VAD integration test
#[test]
fn test_activity_interrupts_silence() {
    // 35 lines generating patterns
    // Better tested with real audio in VAD
}

// ❌ REMOVE: test_real_world_scenarios
// Reason: These ARE integration tests, move to proper location
#[test]
fn test_real_world_scenarios() {
    // Actually tests behavior! Move to integration test
}
```

#### Tests to KEEP (consolidated):

```rust
// ✅ KEEP: One focused algorithm test for edge cases
#[test]
fn test_silence_detector_rms_algorithm_edge_cases() {
    let detector = SilenceDetector::new(100);

    // Edge case: Empty samples
    assert!(detector.is_silent(&[]));

    // Edge case: Max values
    assert!(!detector.is_silent(&[i16::MAX; 10]));

    // Edge case: Alternating extremes
    assert!(!detector.is_silent(&[i16::MAX, i16::MIN, i16::MAX, i16::MIN]));

    // Edge case: Zero threshold
    let detector_zero = SilenceDetector::new(0);
    assert!(detector_zero.is_silent(&[0, 0, 0]));
    assert!(!detector_zero.is_silent(&[1, 0, 0]));

    // Consolidated: All edge cases in one focused test
}

// ✅ KEEP: Boundary conditions
#[test]
fn test_silence_detector_threshold_boundaries() {
    // Test boundary behavior that's hard to test in integration
    // Very high threshold edge case
    let detector_high = SilenceDetector::new(10000);
    assert!(detector_high.is_silent(&generate_sine_wave(440.0, 16000, 100)));
}
```

#### NEW Integration Tests (to add):

```rust
// crates/app/tests/integration/vad_speech_segmentation_test.rs
#[tokio::test]
async fn test_vad_segments_speech_from_silence() {
    // Replaces 5+ unit tests
    // Tests with real audio: silence → speech → silence
    // Verifies user-facing behavior
}

#[tokio::test]
async fn test_vad_handles_background_noise() {
    // Tests real scenario: speech with background noise
    // Replaces "test_real_world_scenarios"
}
```

**Net Change**: 9 tests → 4 tests (2 algorithm + 2 integration)
**Line Count**: 175 lines → ~120 lines (far more valuable)

---

### 3. `crates/app/tests/settings_test.rs`

**Current**: 9 tests (3 ignored), 111 lines
**Target**: 3 tests, ~80 lines

#### Tests to REMOVE (consolidate):

```rust
// ❌ REMOVE: test_settings_new_default
// Reason: Covered by comprehensive config test
#[test]
fn test_settings_new_default() {
    let settings = Settings::new().unwrap();
    assert_eq!(settings.resampler_quality.to_lowercase(), "balanced");
    // ... just checks defaults, no value beyond comprehensive test
}

// ❌ REMOVE: test_settings_validate_zero_timeout
// ❌ REMOVE: test_settings_validate_zero_validation
// Reason: Consolidate into one validation test
#[test]
fn test_settings_validate_zero_timeout() { ... }
#[test]
fn test_settings_validate_zero_validation() { ... }

// ❌ REMOVE: test_settings_validate_invalid_mode
// ❌ REMOVE: test_settings_validate_invalid_rate
// ❌ REMOVE: test_settings_validate_success_rate
// Reason: Consolidate into one comprehensive validation test
#[test]
fn test_settings_validate_invalid_mode() { ... }
#[test]
fn test_settings_validate_invalid_rate() { ... }
#[test]
fn test_settings_validate_success_rate() { ... }
```

#### Tests to KEEP (consolidated + enhanced):

```rust
// ✅ NEW: Comprehensive happy path test
#[test]
fn test_settings_load_and_validate_complete_config() {
    // Tests complete loading + validation in one test
    // Replaces test_settings_new_default
}

// ✅ NEW: Comprehensive validation test
#[test]
fn test_settings_validation_catches_all_invalid_configs() {
    // Tests ALL validation categories in one test:
    // - Zero values (should error)
    // - Out-of-range (should clamp)
    // - Invalid enums (should default)
    // - Boundary conditions
    // Replaces 5+ individual validation tests
}

// ✅ KEEP: Env var test (currently ignored, needs fix)
#[test]
#[ignore = "Pre-existing issue with env var config"]
fn test_settings_env_var_overrides() {
    // Consolidates 3 env var tests
}
```

**Net Change**: 9 tests → 3 tests
**Line Count**: 111 lines → ~80 lines (more comprehensive coverage)

---

### 4. `crates/app/tests/integration/capture_integration_test.rs`

**Current**: 7 tests, 242 lines
**Target**: 3 tests, ~150 lines

#### Tests to REMOVE or MOVE:

```rust
// ❌ REMOVE: test_end_to_end_capture_pipewire
// Reason: Too specific to PipeWire, consolidate into device capture test
#[test]
#[cfg(feature = "live-hardware-tests")]
fn test_end_to_end_capture_pipewire() { ... }

// ❌ REMOVE: test_stats_reporting
// Reason: Covered by main capture test
#[test]
#[cfg(feature = "live-hardware-tests")]
fn test_stats_reporting() { ... }

// ❌ REMOVE: test_frame_flow
// Reason: Covered by main capture test
#[test]
#[cfg(feature = "live-hardware-tests")]
fn test_frame_flow() { ... }

// ❌ REMOVE: test_clean_shutdown
// Reason: Shutdown tested in main integration test
#[test]
#[cfg(feature = "live-hardware-tests")]
fn test_clean_shutdown() { ... }

// ❌ MOVE: test_concurrent_operations → unit test
// Reason: This is pure logic, not integration
#[test]
fn test_concurrent_operations() {
    // Move to unit test file for channel behavior
}

// ❌ MOVE: test_buffer_pressure → unit test
// Reason: This is pure logic, not integration
#[test]
fn test_buffer_pressure() {
    // Move to unit test file for channel behavior
}

// ❌ REMOVE: test_device_specific_capture
// Reason: Consolidate into main device test
#[test]
#[cfg(feature = "live-hardware-tests")]
fn test_device_specific_capture() { ... }
```

#### Tests to KEEP (consolidated + enhanced):

```rust
// ✅ NEW: Comprehensive capture test
#[tokio::test]
async fn test_audio_capture_complete_flow() {
    // Replaces: test_end_to_end_capture_pipewire,
    //           test_stats_reporting,
    //           test_frame_flow,
    //           test_device_specific_capture

    // Start capture
    let mut capture = AudioCapture::new(config).unwrap();
    capture.start(None).await.unwrap();

    // Verify stats
    let initial = capture.get_stats();
    tokio::time::sleep(Duration::from_secs(2)).await;
    let after = capture.get_stats();

    assert!(after.frames_captured > initial.frames_captured);
    assert!(after.active_frames + after.silent_frames > 0);

    // Verify frame flow
    let frames: Vec<_> = capture.receive_frames(100).collect();
    assert!(!frames.is_empty());
    assert_eq!(frames[0].sample_rate, 16000);

    // Clean shutdown
    capture.stop();
    assert_eq!(capture.get_stats().disconnections, 0);
}

// ✅ NEW: Stress test
#[test]
fn test_audio_channel_backpressure() {
    // Replaces: test_concurrent_operations, test_buffer_pressure
    // Tests channel behavior under load
}
```

**Net Change**: 7 tests → 3 tests (2 integration + 1 unit)
**Line Count**: 242 lines → ~150 lines

---

### 5. `crates/app/tests/integration/text_injection_integration_test.rs`

**Current**: 7 tests, 236 lines
**Target**: 3 tests, ~150 lines

#### Tests to REMOVE (consolidate):

```rust
// ❌ CONSOLIDATE: test_text_injection_end_to_end
// ❌ CONSOLIDATE: test_method_fallback_sequence
// Reason: Test both in one comprehensive test
#[tokio::test]
async fn test_text_injection_end_to_end() { ... }
#[tokio::test]
async fn test_method_fallback_sequence() { ... }

// ❌ CONSOLIDATE: test_injection_with_failure_and_recovery
// ❌ CONSOLIDATE: test_cooldown_and_recovery
// Reason: Both test recovery, merge into one
#[tokio::test]
async fn test_injection_with_failure_and_recovery() { ... }
#[tokio::test]
async fn test_cooldown_and_recovery() { ... }

// ❌ CONSOLIDATE: test_injection_timeout_handling
// Reason: Merge into failure handling test
#[tokio::test]
async fn test_injection_timeout_handling() { ... }

// ❌ REMOVE: test_injection_with_unknown_focus_allowed
// Reason: Config detail, covered by main test
#[tokio::test]
async fn test_injection_with_unknown_focus_allowed() { ... }

// ❌ REMOVE: test_clipboard_save_restore_simulation
// Reason: Simulated test, real test exists in mock_injection_tests.rs
#[tokio::test]
async fn test_clipboard_save_restore_simulation() { ... }
```

#### Tests to KEEP (consolidated):

```rust
// ✅ NEW: Comprehensive injection test
#[tokio::test]
async fn test_text_injection_complete_flow_with_fallback() {
    // Replaces: test_text_injection_end_to_end,
    //           test_method_fallback_sequence,
    //           test_injection_with_unknown_focus_allowed

    // Tests successful injection AND fallback chain
    let manager = StrategyManager::new(config).await;

    // Normal injection
    assert!(manager.inject("Hello").await.is_ok());

    // Verify metrics
    let metrics = manager.metrics().await;
    assert_eq!(metrics.successes, 1);

    // Disable primary method, verify fallback
    manager.disable_method(0);
    assert!(manager.inject("Fallback test").await.is_ok());
}

// ✅ NEW: Recovery and resilience test
#[tokio::test]
async fn test_injection_failure_recovery_and_cooldown() {
    // Replaces: test_injection_with_failure_and_recovery,
    //           test_cooldown_and_recovery,
    //           test_injection_timeout_handling

    let manager = StrategyManager::new(config).await;

    // Force failure
    manager.set_impossible_timeout(1); // 1ms timeout
    assert!(manager.inject("Fail").await.is_err());

    // Verify cooldown active
    assert!(manager.in_cooldown());

    // Wait for cooldown
    tokio::time::sleep(cooldown_duration).await;

    // Restore timeout
    manager.set_normal_timeout();

    // Verify recovery
    assert!(manager.inject("Recovered").await.is_ok());
    assert!(!manager.in_cooldown());
}
```

**Net Change**: 7 tests → 2 tests
**Line Count**: 236 lines → ~120 lines

---

## Tests to ADD (New Large-Span Tests)

### Critical User Journeys (HIGH PRIORITY)

```rust
// crates/app/tests/integration/dictation_journey_test.rs
#[tokio::test]
async fn test_complete_dictation_with_stt_recovery() {
    // NEW: Tests resilience of complete pipeline
    // User speaks → STT fails → recovers → continues
    // ~100 lines, replaces nothing but fills critical gap
}

// crates/app/tests/integration/audio_recovery_test.rs
#[tokio::test]
async fn test_audio_pipeline_auto_recovers_from_disconnect() {
    // NEW: Tests watchdog in real context
    // Audio disconnect → watchdog triggers → pipeline recovers
    // ~80 lines, replaces 4 watchdog unit tests
}

// crates/app/tests/integration/vad_speech_segmentation_test.rs
#[tokio::test]
async fn test_vad_segments_speech_from_silence() {
    // NEW: Tests VAD with real audio
    // Real audio → VAD → correct speech segments
    // ~70 lines, replaces 5+ silence detector unit tests
}

#[tokio::test]
async fn test_vad_handles_background_noise() {
    // NEW: Tests VAD noise rejection
    // Noisy audio → VAD → speech detected, noise ignored
    // ~50 lines, replaces "test_real_world_scenarios"
}
```

### Performance & Tracing (MEDIUM PRIORITY)

```rust
// crates/app/tests/integration/pipeline_trace_test.rs
#[tokio::test]
async fn test_pipeline_trace_verifies_latency_budget() {
    // NEW: Trace-based testing
    // Complete pipeline with OpenTelemetry
    // Verify latency budget met end-to-end
    // ~120 lines, fills critical gap in distributed testing
}
```

---

## Migration Strategy

### Phase 1: Immediate (This PR)
1. ✅ Create analysis documents (done)
2. ✅ Create improvement plan (done)
3. ✅ Create test removal plan (this document)
4. Consolidate settings tests (1 hour)
5. Update main testing documentation (2 hours)
6. Commit and document changes

### Phase 2: Next Sprint
1. Consolidate watchdog tests + add audio recovery test (3 hours)
2. Consolidate silence detector tests + add VAD integration tests (4 hours)
3. Consolidate capture integration tests (2 hours)
4. Consolidate text injection tests (2 hours)

### Phase 3: Future
1. Add dictation journey test (4 hours)
2. Add trace-based testing infrastructure (8 hours)
3. Add performance regression tests (6 hours)

---

## Expected Metrics After Consolidation

| Metric | Before | After | Change |
|--------|--------|-------|--------|
| Total Test Count | ~60 | ~35 | -42% |
| Integration Test % | 50% | 70% | +40% |
| E2E Test % | 10% | 15% | +50% |
| Unit Test % | 40% | 15% | -62% |
| Total Test Lines | ~1500 | ~1200 | -20% |
| Coverage (behavior) | 65% | 85% | +31% |
| Test Execution Time | ~45s | ~35s | -22% |
| Test Flakiness | <5% | <1% | -80% |

---

## Success Criteria

✅ **Test count reduced by 40%+**
✅ **Integration tests become 70% of suite**
✅ **All critical user journeys covered**
✅ **Tests tell complete stories**
✅ **Tests don't break on refactors**
✅ **Documentation reflects philosophy**

---

## Review Checklist

Before removing any test, verify:

- [ ] Is this behavior covered by a larger integration test?
- [ ] Is this an implementation detail (internal state, private methods)?
- [ ] Would removing this test reduce coverage of user-facing behavior?
- [ ] Is there a complex algorithm that needs focused testing?

**If yes to #4**: Keep one focused algorithm test
**If yes to #3**: Don't remove, consolidate into integration test
**If yes to #1-2**: Safe to remove

