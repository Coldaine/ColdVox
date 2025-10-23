# Testing Examples: Good vs Bad

This document shows concrete examples of tests transformed from fragmented/mocked to large-span/behavioral, specific to ColdVox's architecture.

---

## Example 1: Audio Watchdog Testing

### ❌ BEFORE: 6 Fragmented Unit Tests

**File**: `crates/app/tests/unit/watchdog_test.rs`
**Lines**: 107
**Problems**: Tests implementation details, doesn't prove user value, breaks on refactor

```rust
// Test 1: Trivial construction test
#[tokio::test]
async fn test_watchdog_creation() {
    let _wd1 = WatchdogTimer::new(Duration::from_secs(1));
    let _wd2 = WatchdogTimer::new(Duration::from_millis(250));
    // ❌ Just creates objects - no value
}

// Test 2: Feed prevents timeout
#[tokio::test]
async fn test_watchdog_feed_prevents_timeout() {
    let test_clock = clock::test_clock();
    let mut wd = WatchdogTimer::new_with_clock(Duration::from_millis(200), test_clock.clone());
    wd.start(running.clone());

    for _ in 0..5 {
        test_clock.sleep(Duration::from_millis(100));
        wd.feed();
    }

    assert!(!wd.is_triggered());
    // ✓ Tests algorithm, but in isolation
}

// Test 3-6: More implementation details...
// test_watchdog_timeout_triggers
// test_watchdog_stop_resets_trigger
// test_restart_does_not_carry_trigger_state
// test_concurrent_feed_operations
```

**Why This Is Bad**:
1. **Implementation Coupling**: Tests internal state (is_triggered)
2. **No User Value**: Doesn't prove watchdog helps users
3. **Fragmentation**: 6 tests for one component
4. **Missing Context**: Watchdog never tested as part of audio pipeline
5. **Refactor Brittle**: Changing watchdog internals breaks tests

---

### ✅ AFTER: 1 Integration Test + 1 Algorithm Test

**File**: `crates/app/tests/integration/audio_recovery_test.rs`
**Lines**: ~80 (but far more valuable)

```rust
#[tokio::test]
async fn test_audio_pipeline_auto_recovers_from_disconnection() {
    """
    Complete user story:
    When microphone briefly disconnects, dictation continues working
    without user intervention.

    This tests the REAL value of the watchdog: keeping audio flowing.
    """

    // 1. Start real audio capture with watchdog enabled
    let config = AudioConfig {
        sample_rate: 16000,
        watchdog_timeout: Duration::from_secs(5),
        ..Default::default()
    };

    let mut capture = AudioCapture::new(config).unwrap();
    capture.start(None).await.unwrap();

    // 2. Verify audio is flowing
    let initial_frames = capture.get_stats().frames_captured;
    tokio::time::sleep(Duration::from_secs(1)).await;
    let after_initial = capture.get_stats().frames_captured;

    assert!(after_initial > initial_frames,
        "Audio should be flowing initially");

    // 3. Simulate device disconnection (test hook)
    capture.simulate_disconnect();
    eprintln!("Simulated audio device disconnect");

    // 4. Wait for watchdog to detect (5 second timeout + margin)
    tokio::time::sleep(Duration::from_secs(6)).await;

    // 5. Verify pipeline auto-recovered
    let recovered_stats = capture.get_stats();

    assert_eq!(recovered_stats.disconnections, 1,
        "Should record one disconnection event");
    assert!(recovered_stats.recovery_attempts >= 1,
        "Watchdog should have triggered recovery");

    // 6. Verify audio is flowing again (THE KEY USER-FACING OUTCOME)
    let before_recovery_check = recovered_stats.frames_captured;
    tokio::time::sleep(Duration::from_secs(1)).await;
    let after_recovery_check = capture.get_stats().frames_captured;

    assert!(after_recovery_check > before_recovery_check,
        "Audio should flow again after watchdog recovery");

    // 7. Clean shutdown
    capture.stop();

    println!("✅ Audio pipeline successfully recovered from disconnection");
    println!("   Total recovery time: ~6 seconds");
    println!("   User impact: Minimal - dictation continues");
}

// Keep ONE focused test for pure algorithm edge cases
#[test]
fn test_watchdog_timer_core_algorithm() {
    """
    Focused test for watchdog timer algorithm edge cases.
    This tests the algorithm correctness, not user behavior.
    """
    let test_clock = clock::test_clock();

    // Test 1: Regular feeding prevents timeout
    let mut wd = WatchdogTimer::new_with_clock(
        Duration::from_millis(200),
        test_clock.clone()
    );
    for _ in 0..5 {
        test_clock.sleep(Duration::from_millis(100));
        wd.feed();
    }
    assert!(!wd.is_triggered(), "Regular feeding should prevent timeout");

    // Test 2: Timeout triggers correctly
    test_clock.sleep(Duration::from_millis(250));
    assert!(wd.is_triggered(), "Should trigger after timeout period");

    // Test 3: Stop resets state
    wd.stop();
    assert!(!wd.is_triggered(), "Stop should clear triggered state");
}
```

**Why This Is Better**:
1. ✅ **External Observer**: Tests what user sees (audio keeps working)
2. ✅ **Real Action**: Uses real AudioCapture component
3. ✅ **Larger Span**: Tests watchdog + capture + recovery together
4. ✅ **Failure Clarity**: Failure means "recovery doesn't work for users"
5. ✅ **Complete Story**: "Mic disconnects → system recovers → dictation continues"
6. ✅ **Refactor Safe**: Only breaks if user-facing behavior changes

**Net Result**: 6 tests → 2 tests, better coverage, proves real value

---

## Example 2: Silence Detection Testing

### ❌ BEFORE: 9 Fragmented Unit Tests

**File**: `crates/app/tests/unit/silence_detector_test.rs`
**Lines**: 175
**Problems**: Tests RMS formula, not user value; no integration with VAD

```rust
#[test]
fn test_rms_calculation() {
    let detector = SilenceDetector::new(100);

    // Test with known values
    let samples = vec![100, -100, 100, -100];
    let is_silent = detector.is_silent(&samples);
    assert!(!is_silent);
    // ❌ Tests implementation (RMS formula)
}

#[test]
fn test_silence_threshold_50() {
    let detector = SilenceDetector::new(50);
    let quiet_samples = generate_noise(100, 40);
    assert!(detector.is_silent(&quiet_samples));
    // ❌ Tests specific threshold in isolation
}

#[test]
fn test_silence_threshold_500() { ... }
#[test]
fn test_continuous_silence_tracking() { ... }
#[test]
fn test_activity_interrupts_silence() { ... }
#[test]
fn test_edge_cases() { ... }
#[test]
fn test_threshold_boundary_conditions() { ... }
#[test]
fn test_real_world_scenarios() { ... }
// ❌ 9 tests testing algorithm details, not user value
```

**Why This Is Bad**:
1. Tests RMS formula implementation
2. Never tests with real audio
3. Never tests as part of VAD pipeline
4. Doesn't prove: "Users only dictate speech, not silence"

---

### ✅ AFTER: 1 Algorithm Test + 2 Integration Tests

**File 1**: `crates/app/tests/integration/vad_speech_segmentation_test.rs`
**Lines**: ~120 (tests real behavior)

```rust
#[tokio::test]
async fn test_vad_correctly_segments_speech_from_silence() {
    """
    Complete user story:
    When user speaks with pauses, VAD correctly identifies speech segments
    and ignores silence/background noise.

    User value: Only speech gets transcribed, not pauses or background noise.
    """

    // Load REAL test audio with known speech pattern
    // Audio contains: 1s silence, 2s speech, 1s silence, 3s speech, 1s silence
    let audio_path = "test_data/speech_with_pauses.wav";
    let audio = load_test_audio_16k_mono(audio_path);

    let expected_segments = vec![
        SpeechSegment { start: 1.0, duration: 2.0 },  // First speech
        SpeechSegment { start: 4.0, duration: 3.0 },  // Second speech
    ];

    // Run through COMPLETE VAD pipeline (Silero + SilenceDetector)
    let vad_config = UnifiedVadConfig {
        mode: VadMode::Silero,
        frame_size_samples: 512,
        sample_rate_hz: 16000,
        silero: Default::default(),
    };

    let segments = run_vad_on_audio(&audio, vad_config).await;

    // Verify user-facing outcome: Found correct speech segments
    assert_eq!(segments.len(), 2,
        "Should detect exactly 2 speech segments");

    for (actual, expected) in segments.iter().zip(expected_segments.iter()) {
        assert!(
            (actual.start_time - expected.start).abs() < 0.2,
            "Segment start time accurate within 200ms: expected {}, got {}",
            expected.start,
            actual.start_time
        );

        assert!(
            (actual.duration - expected.duration).abs() < 0.3,
            "Segment duration accurate within 300ms: expected {}, got {}",
            expected.duration,
            actual.duration
        );
    }

    // Verify silence was correctly ignored
    assert!(
        segments[0].start_time >= 0.8,
        "Initial silence should be ignored (started at {}s)",
        segments[0].start_time
    );

    let last_segment_end = segments[1].start_time + segments[1].duration;
    assert!(
        last_segment_end <= 7.2,
        "Trailing silence should be ignored (ended at {}s)",
        last_segment_end
    );

    println!("✅ VAD correctly segmented speech from silence");
    println!("   Segment 1: {:.2}s - {:.2}s ({:.2}s)",
        segments[0].start_time,
        segments[0].start_time + segments[0].duration,
        segments[0].duration);
    println!("   Segment 2: {:.2}s - {:.2}s ({:.2}s)",
        segments[1].start_time,
        segments[1].start_time + segments[1].duration,
        segments[1].duration);
}

#[tokio::test]
async fn test_vad_rejects_background_noise() {
    """
    Story: VAD distinguishes speech from background noise (fan, keyboard, etc.)

    User value: Background noise doesn't trigger false transcription attempts.
    """

    // Real audio: 2s background noise, 3s speech with noise, 1s noise
    let audio = load_test_audio_16k_mono("test_data/speech_with_background_noise.wav");

    let segments = run_vad_on_audio(&audio, UnifiedVadConfig::default()).await;

    // Should detect speech despite background noise
    assert!(segments.len() >= 1,
        "Should detect speech in noisy environment");

    // Should NOT falsely detect noise-only periods as speech
    assert!(
        segments[0].start_time > 1.5,
        "Should not detect initial noise-only period as speech (started at {}s)",
        segments[0].start_time
    );

    println!("✅ VAD correctly rejected background noise");
    println!("   Noise-only period: 0-2s → No detection");
    println!("   Speech+noise period: 2-5s → Detected at {:.2}s",
        segments[0].start_time);
}
```

**File 2**: `crates/app/tests/unit/silence_detector_algorithm_test.rs`
**Lines**: ~40 (focused algorithm tests)

```rust
#[test]
fn test_silence_detector_rms_algorithm_edge_cases() {
    """
    Focused test for SilenceDetector algorithm edge cases.
    These are hard to test with real audio files.
    """
    let detector = SilenceDetector::new(100);

    // Edge case 1: Empty samples
    assert!(detector.is_silent(&[]),
        "Empty samples should be considered silent");

    // Edge case 2: Maximum positive values
    assert!(!detector.is_silent(&[i16::MAX; 10]),
        "Max positive values should not be silent");

    // Edge case 3: Maximum negative values
    assert!(!detector.is_silent(&[i16::MIN; 10]),
        "Max negative values should not be silent");

    // Edge case 4: Alternating extremes (high RMS)
    assert!(!detector.is_silent(&[i16::MAX, i16::MIN, i16::MAX, i16::MIN]),
        "Alternating max values should have high RMS");

    // Edge case 5: Zero threshold (everything except silence is active)
    let detector_zero = SilenceDetector::new(0);
    assert!(detector_zero.is_silent(&[0, 0, 0]),
        "Zero threshold: zeros should be silent");
    assert!(!detector_zero.is_silent(&[1, 0, 0]),
        "Zero threshold: any non-zero should be active");

    // Edge case 6: Very high threshold (even loud audio is "silent")
    let detector_high = SilenceDetector::new(30000);
    let loud_samples = vec![5000i16; 100];
    assert!(detector_high.is_silent(&loud_samples),
        "Very high threshold should classify loud audio as silent");
}
```

**Why This Is Better**:
1. ✅ Tests with REAL audio files
2. ✅ Tests complete VAD pipeline (not detector in isolation)
3. ✅ Proves user value: "Speech detected, silence ignored"
4. ✅ One algorithm test covers all edge cases
5. ✅ Integration tests won't break on detector refactor

**Net Result**: 9 tests → 3 tests, proves real behavior, better coverage

---

## Example 3: Text Injection Testing

### ❌ BEFORE: Mock-Heavy Test

```rust
#[test]
fn test_injection_with_mocks() {
    // ❌ Create mock injector
    let mock_injector = MockInjector::new();
    mock_injector
        .expect_inject()
        .times(1)
        .with(eq("test text"))
        .returning(|_| Ok(()));

    // ❌ Create mock window manager
    let mock_wm = MockWindowManager::new();
    mock_wm
        .expect_get_focused_window()
        .returning(|| Some("terminal"));

    // Use mocks
    let manager = InjectionManager::new(mock_injector, mock_wm);
    let result = manager.inject("test text");

    // ❌ What did we prove? Only that our mocks work
    assert!(result.is_ok());
}
```

**Why This Is Bad**:
1. Tests mock configuration, not real injection
2. Doesn't prove text actually appears in target application
3. Brittle: Changes to injector interface break test
4. No confidence that real injection works

---

### ✅ AFTER: Real Injection Test

```rust
#[tokio::test]
async fn test_text_injection_into_real_terminal() {
    """
    Complete user story:
    Text from STT is injected into focused terminal application.

    User value: Dictated text appears where user is typing.
    """

    // 1. Launch REAL terminal application
    let terminal = TestTerminal::spawn().await.unwrap();
    let capture_file = terminal.capture_file();

    // Give terminal time to start and grab focus
    tokio::time::sleep(Duration::from_millis(500)).await;

    // 2. Create REAL injection manager with REAL backends
    let injection_config = InjectionConfig {
        allow_kdotool: true,
        allow_enigo: true,
        inject_on_unknown_focus: true,  // For test environment
        ..Default::default()
    };

    let manager = StrategyManager::new(injection_config).await;

    // 3. Perform REAL injection
    let test_text = "Hello from ColdVox testing!";
    let result = manager.inject(test_text).await;

    assert!(result.is_ok(),
        "Injection should succeed: {:?}", result.err());

    // 4. Verify REAL outcome: Text appears in terminal
    tokio::time::sleep(Duration::from_millis(200)).await;
    let captured = fs::read_to_string(capture_file)
        .unwrap_or_default();

    assert!(
        captured.contains(test_text),
        "Injected text should appear in terminal.\n\
         Expected: {}\n\
         Captured: {}",
        test_text,
        captured
    );

    // 5. Verify metrics
    let metrics = manager.get_metrics().await;
    assert_eq!(metrics.successes, 1, "Should record one success");
    assert_eq!(metrics.attempts, 1, "Should record one attempt");

    // 6. Cleanup
    terminal.kill().await.unwrap();

    println!("✅ Text injection verified end-to-end");
    println!("   Backend used: {:?}", manager.last_successful_method());
    println!("   Latency: {:?}", metrics.last_latency);
}

#[tokio::test]
async fn test_injection_fallback_chain() {
    """
    Story: If primary injection method fails, system falls back to alternatives.

    User value: Dictation works even if preferred backend is unavailable.
    """

    let terminal = TestTerminal::spawn().await.unwrap();

    let mut config = InjectionConfig::default();
    config.allow_kdotool = false;  // Disable primary method

    let manager = StrategyManager::new(config).await;

    // Should succeed using fallback method
    let result = manager.inject("Fallback test").await;
    assert!(result.is_ok(), "Should succeed with fallback method");

    // Verify correct fallback was used
    let metrics = manager.get_metrics().await;
    let method_used = manager.last_successful_method();

    assert_ne!(method_used, InjectionMethod::Kdotool,
        "Should not use disabled method");
    println!("✅ Fallback injection successful using {:?}", method_used);
}
```

**Why This Is Better**:
1. ✅ Tests REAL injection into REAL application
2. ✅ Verifies text actually appears (user-facing outcome)
3. ✅ Tests fallback behavior with real backends
4. ✅ Won't break on interface changes (tests behavior)
5. ✅ Proves the feature actually works

---

## Summary: Transformation Patterns

### Pattern 1: Consolidate Related Tests
- **Before**: `test_create()`, `test_start()`, `test_stop()`, `test_reset()`
- **After**: `test_complete_lifecycle()`

### Pattern 2: Move to Integration
- **Before**: Test component in isolation with mocks
- **After**: Test component as part of larger flow with real dependencies

### Pattern 3: Focus Algorithm Tests
- **Before**: 10 tests covering normal + edge cases
- **After**: 1 test covering ALL edge cases, integration tests for normal cases

### Pattern 4: Test User Value
- **Before**: Test internal state/private methods
- **After**: Test observable user-facing outcomes

### Pattern 5: Use Real Dependencies
- **Before**: Mock everything
- **After**: Real services, behavioral fakes, or TestContainers

---

## Quick Decision Tree

```
Need to test new feature?
│
├─ Is it complex algorithm (>20 lines)?
│  └─ YES → Write 1 focused algorithm test + integration test
│  └─ NO → Write integration test only
│
├─ Does it interact with external service?
│  └─ YES → Can we use real service/sandbox?
│     ├─ YES → Use real service
│     └─ NO → Use behavioral fake + document need for real test
│
├─ Is it critical user journey?
│  └─ YES → Write E2E test covering complete flow
│  └─ NO → Integration test is sufficient
│
└─ Can this be part of existing test?
   └─ YES → Extend existing test
   └─ NO → Write new integration test
```

---

## Anti-Patterns to Avoid

### ❌ The Mock Maze
```rust
let mock_a = Mock::new();
let mock_b = Mock::new();
let mock_c = Mock::new();
mock_a.expect_call().returning(|| mock_b);
mock_b.expect_call().returning(|| mock_c);
// Tests mocking, not behavior
```

### ❌ The Implementation Mirror
```rust
#[test]
fn test_internal_state() {
    let obj = MyService::new();
    obj.process();
    assert_eq!(obj._internal_state, "PROCESSED");  // Private field
    assert_eq!(obj._step_count, 3);  // Implementation detail
}
```

### ❌ The Fragmentation Fallacy
```rust
#[test] fn test_step1() { /* ... */ }
#[test] fn test_step2() { /* ... */ }
#[test] fn test_step3() { /* ... */ }
#[test] fn test_step4() { /* ... */ }
// Should be ONE test: test_complete_process()
```

### ❌ The Speed Excuse
```
"But integration tests are slow!"

Reality:
- Modern TestContainers: 200ms startup
- In-memory databases: 5ms operations
- Real audio processing: 100ms for test file
- Total: < 500ms for complete integration test

Fragmented unit tests with mocks:
- 10 tests × 50ms each = 500ms
- Setup/teardown overhead
- Less valuable coverage
```

---

## Before You Write a Test...

Ask yourself these questions:

1. **External Observer**: What would a user expect to see?
2. **Real Action**: Can I use real components instead of mocks?
3. **Larger Span**: Could this be part of a bigger test?
4. **Failure Clarity**: Will failure mean behavior is broken?
5. **Story**: Does this test tell a complete story?
6. **No-Mock**: How can I eliminate mocks?

**If you can't answer these positively, reconsider your test approach.**

