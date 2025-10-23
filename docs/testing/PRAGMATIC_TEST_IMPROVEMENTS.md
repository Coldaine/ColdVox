# Pragmatic Test Improvements: Implementation Plan

**Date**: 2025-10-23
**Status**: Implementation Ready
**Priority**: High

This document outlines specific code changes to align ColdVox's test suite with the Pragmatic Test Architect philosophy.

---

## Phase 1: Consolidate Fragmented Tests (Immediate)

### 1.1 Watchdog Tests → Audio Recovery Test

**File**: `crates/app/tests/unit/watchdog_test.rs`
**Action**: **CONSOLIDATE + INTEGRATE**
**Time Estimate**: 2 hours

**Current State**: 6 separate watchdog tests testing timer behavior in isolation

**Target State**: 2-3 tests testing watchdog as part of audio pipeline

**New Test**:
```rust
// crates/app/tests/integration/audio_recovery_test.rs
#[tokio::test]
async fn test_audio_pipeline_auto_recovers_from_device_disconnect() {
    """
    Complete story: Audio capture continues working even when device temporarily
    disconnects. This tests the watchdog triggering recovery.

    User value: Dictation doesn't stop if microphone briefly disconnects.
    """

    // 1. Start real audio capture pipeline
    let mut capture = AudioCapture::new(config).unwrap();
    capture.start(None).await.unwrap();

    // 2. Verify audio is flowing
    let initial_frames = capture.get_stats().frames_captured;
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert!(capture.get_stats().frames_captured > initial_frames);

    // 3. Simulate device disconnect by stopping stream
    capture.simulate_disconnect(); // Internal test hook

    // 4. Wait for watchdog to detect (5 second timeout)
    tokio::time::sleep(Duration::from_secs(6)).await;

    // 5. Verify pipeline auto-recovered
    let recovered_stats = capture.get_stats();
    assert_eq!(recovered_stats.disconnections, 1, "Should record one disconnection");
    assert!(recovered_stats.recovery_attempts >= 1, "Should attempt recovery");

    // 6. Verify audio is flowing again
    let before_recovery = recovered_stats.frames_captured;
    tokio::time::sleep(Duration::from_secs(1)).await;
    let after_recovery = capture.get_stats().frames_captured;
    assert!(after_recovery > before_recovery, "Audio should flow after recovery");

    capture.stop();
}

#[tokio::test]
async fn test_watchdog_feed_prevents_timeout() {
    // Keep ONE focused test for the core watchdog algorithm
    // This tests pure logic: feeding prevents timeout
    let test_clock = clock::test_clock();
    let mut wd = WatchdogTimer::new_with_clock(Duration::from_millis(200), test_clock.clone());

    for _ in 0..5 {
        test_clock.sleep(Duration::from_millis(100));
        wd.feed();
    }

    assert!(!wd.is_triggered(), "Regular feeding should prevent timeout");
}
```

**Tests to Remove**:
- `test_watchdog_creation` (trivial)
- `test_watchdog_stop_resets_trigger` (implementation detail)
- `test_restart_does_not_carry_trigger_state` (implementation detail)
- `test_concurrent_feed_operations` (covered by integration test)

**Tests to Keep**:
- `test_watchdog_feed_prevents_timeout` (pure algorithm logic)
- `test_watchdog_timeout_triggers` (pure algorithm logic)

---

### 1.2 Silence Detector Tests → VAD Integration Test

**File**: `crates/app/tests/unit/silence_detector_test.rs`
**Action**: **CONSOLIDATE + INTEGRATE**
**Time Estimate**: 3 hours

**Current State**: 9 separate micro-tests testing RMS calculation

**Target State**: 1-2 focused algorithm tests + integration into VAD pipeline tests

**New Tests**:
```rust
// crates/app/tests/integration/vad_speech_segmentation_test.rs
#[tokio::test]
async fn test_vad_correctly_segments_speech_from_silence() {
    """
    Complete story: VAD pipeline correctly identifies speech segments in real audio
    with background noise and silence periods.

    User value: Only speech gets transcribed, not silence or background noise.
    """

    // Load real test audio with known speech/silence pattern
    // Pattern: 1s silence, 2s speech, 1s silence, 3s speech, 1s silence
    let audio = load_test_audio("test_data/speech_with_pauses.wav");
    let expected_segments = vec![
        SpeechSegment { start: 1.0, end: 3.0 },
        SpeechSegment { start: 4.0, end: 7.0 },
    ];

    // Run through complete VAD pipeline (Silero + silence detector)
    let vad_config = UnifiedVadConfig::default();
    let segments = run_vad_on_audio(audio, vad_config).await;

    // Verify: Found exactly the expected speech segments
    assert_eq!(segments.len(), 2, "Should find 2 speech segments");

    for (actual, expected) in segments.iter().zip(expected_segments.iter()) {
        assert!((actual.start - expected.start).abs() < 0.1,
            "Segment start time accurate within 100ms");
        assert!((actual.duration - (expected.end - expected.start)).abs() < 0.2,
            "Segment duration accurate within 200ms");
    }

    // Verify: Silence periods were correctly ignored
    assert!(!segments.iter().any(|s| s.start < 0.5),
        "Initial silence should be ignored");
    assert!(!segments.iter().any(|s| s.end > 7.5),
        "Trailing silence should be ignored");
}

#[tokio::test]
async fn test_vad_handles_noisy_environment() {
    """
    Story: VAD distinguishes speech from background noise (fan, keyboard, etc.)
    """
    let audio = load_test_audio("test_data/speech_with_background_noise.wav");

    let segments = run_vad_on_audio(audio, UnifiedVadConfig::default()).await;

    // Verify: Speech detected despite noise
    assert!(segments.len() >= 1, "Should detect speech in noisy environment");

    // Verify: Background noise alone didn't trigger false positives
    // (This WAV has 2s of noise-only at the start)
    assert!(segments[0].start > 1.5,
        "Should not falsely detect noise-only periods as speech");
}

// Keep ONE focused test for the RMS algorithm edge cases
#[test]
fn test_silence_detector_rms_algorithm() {
    let detector = SilenceDetector::new(100);

    // Edge case: Empty samples
    assert!(detector.is_silent(&[]));

    // Edge case: Max values
    assert!(!detector.is_silent(&[i16::MAX; 10]));

    // Edge case: Alternating max values (high RMS)
    assert!(!detector.is_silent(&[i16::MAX, i16::MIN, i16::MAX, i16::MIN]));
}
```

**Tests to Remove**:
- `test_rms_calculation` → Covered by integration test
- `test_silence_threshold_50/500` → Covered by integration test
- `test_continuous_silence_tracking` → Covered by integration test
- `test_activity_interrupts_silence` → Covered by integration test
- `test_threshold_boundary_conditions` → Keep one version in algorithm test
- `test_real_world_scenarios` → Covered by integration test

**Tests to Keep**:
- `test_silence_detector_rms_algorithm` (consolidated edge cases)

---

### 1.3 Settings Tests → Configuration Validation Test

**File**: `crates/app/tests/settings_test.rs`
**Action**: **CONSOLIDATE**
**Time Estimate**: 1.5 hours

**New Tests**:
```rust
// crates/app/tests/settings_test.rs (refactored)
#[test]
fn test_settings_load_and_validate_complete_config() {
    """
    Story: Application loads valid configuration and validates all fields correctly.
    This tests the complete happy path.
    """
    let settings = Settings::new().unwrap();

    // Verify all critical settings loaded correctly
    assert_eq!(settings.resampler_quality.to_lowercase(), "balanced");
    assert_eq!(settings.activation_mode.to_lowercase(), "vad");
    assert_eq!(settings.injection.max_total_latency_ms, 800);
    assert!(settings.stt.failover_threshold > 0);

    // Verify validation passes
    assert!(settings.validate().is_ok());
}

#[test]
fn test_settings_validation_catches_all_invalid_configs() {
    """
    Story: Configuration validation catches all categories of invalid settings
    and either errors or auto-corrects to safe defaults.
    """
    let mut settings = Settings::default();

    // Category 1: Zero/invalid timeout (should error)
    settings.injection.max_total_latency_ms = 0;
    assert!(settings.validate().is_err(),
        "Should reject zero latency timeout");

    // Category 2: Out-of-range values (should clamp)
    settings = Settings::default();
    settings.injection.keystroke_rate_cps = 200; // Too high
    assert!(settings.validate().is_ok(), "Should auto-correct");
    assert_eq!(settings.injection.keystroke_rate_cps, 20,
        "Should clamp to maximum");

    // Category 3: Invalid enum values (should default)
    settings = Settings::default();
    settings.resampler_quality = "invalid".to_string();
    assert!(settings.validate().is_ok(), "Should auto-correct");
    assert_eq!(settings.resampler_quality, "balanced",
        "Should default to balanced");

    // Category 4: Success rate out of bounds (should clamp)
    settings = Settings::default();
    settings.injection.min_success_rate = 1.5;
    assert!(settings.validate().is_ok(), "Should auto-correct");
    assert_eq!(settings.injection.min_success_rate, 0.3,
        "Should clamp to valid range");
}

#[test]
#[ignore = "Environment variable overrides currently broken - pre-existing issue"]
fn test_settings_env_var_overrides() {
    """
    Story: Environment variables override config file values.
    TODO: Fix config::Environment integration.
    """
    env::set_var("COLDVOX_ACTIVATION_MODE", "hotkey");
    let settings = Settings::from_path(&get_test_config_path()).unwrap();
    assert_eq!(settings.activation_mode, "hotkey");
    env::remove_var("COLDVOX_ACTIVATION_MODE");
}
```

**Tests to Remove** (consolidated):
- `test_settings_new_default`
- `test_settings_validate_zero_timeout`
- `test_settings_validate_invalid_mode`
- `test_settings_validate_invalid_rate`
- `test_settings_validate_success_rate`
- `test_settings_validate_zero_validation`

**Tests to Keep** (consolidated):
- 3 comprehensive tests above

---

## Phase 2: Add Missing Critical Tests (High Priority)

### 2.1 Complete Dictation Journey Test

**New File**: `crates/app/tests/integration/dictation_journey_test.rs`
**Time Estimate**: 4 hours

```rust
#[tokio::test]
async fn test_complete_dictation_journey_with_stt_recovery() {
    """
    Complete user story: User starts dictating, STT service has a temporary error,
    system recovers automatically, user continues dictating without noticing.

    This is a CRITICAL flow that proves the system is resilient.
    """

    // Setup: Complete pipeline with STT that can be instructed to fail
    let audio = load_test_audio("test_data/long_dictation_session.wav");
    let stt_service = FailableSTTService::new();

    let pipeline = DictationPipeline::new()
        .with_audio(audio)
        .with_vad(UnifiedVadConfig::default())
        .with_stt(stt_service.clone())
        .with_injection(InjectionConfig::default())
        .build();

    // Start dictation
    let mut transcription_stream = pipeline.start().await.unwrap();

    // Collect first few transcriptions (should succeed)
    let mut transcriptions = Vec::new();
    for _ in 0..3 {
        if let Some(event) = transcription_stream.recv().await {
            if let TranscriptionEvent::Final { text, .. } = event {
                transcriptions.push(text);
            }
        }
    }
    assert_eq!(transcriptions.len(), 3, "Should get initial transcriptions");

    // Inject STT failure for next 2 requests
    stt_service.fail_next_n_times(2);

    // System should recover and continue
    // Wait for error + recovery
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Verify: Transcription continues after recovery
    let mut post_recovery_count = 0;
    while post_recovery_count < 3 {
        if let Some(event) = transcription_stream.recv().await {
            if let TranscriptionEvent::Final { text, .. } = event {
                transcriptions.push(text);
                post_recovery_count += 1;
            }
        }
    }

    assert_eq!(transcriptions.len(), 6,
        "Should have 3 pre-failure + 3 post-recovery transcriptions");

    // Verify: Text was injected (even during error recovery)
    let injected_text = pipeline.get_injected_text();
    assert!(injected_text.len() >= 5,
        "Should have injected most transcriptions despite temporary failure");
}
```

---

### 2.2 Trace-Based Pipeline Test

**New File**: `crates/app/tests/integration/pipeline_trace_test.rs`
**Time Estimate**: 5 hours
**Dependencies**: OpenTelemetry integration

```rust
#[tokio::test]
async fn test_pipeline_trace_verifies_latency_budget() {
    """
    Story: Complete pipeline meets latency budget from audio input to text injection.
    Uses distributed tracing to verify each component's contribution.
    """

    // Setup: Pipeline with OpenTelemetry tracing
    let tracer = setup_test_tracer();
    let audio = load_test_audio("test_data/quick_phrase.wav");

    let pipeline = DictationPipeline::new()
        .with_tracer(tracer.clone())
        .with_audio(audio)
        .build();

    let trace_id = pipeline.start_with_tracing().await.unwrap();

    // Wait for complete processing
    pipeline.wait_for_completion().await;

    // Get the complete trace
    let trace = tracer.get_trace(trace_id);

    // Verify: All components participated
    assert_eq!(trace.services(), vec![
        "audio-capture",
        "vad-processor",
        "stt-processor",
        "text-injection",
    ]);

    // Verify: Latency budget met
    let audio_to_vad = trace.span_duration("audio-capture");
    let vad_to_stt = trace.span_duration("vad-processor");
    let stt_to_injection = trace.span_duration("stt-processor");
    let injection_latency = trace.span_duration("text-injection");

    assert!(audio_to_vad < Duration::from_millis(50),
        "Audio to VAD should be < 50ms");
    assert!(vad_to_stt < Duration::from_millis(100),
        "VAD to STT should be < 100ms");
    assert!(stt_to_injection < Duration::from_millis(500),
        "STT processing should be < 500ms");
    assert!(injection_latency < Duration::from_millis(800),
        "Text injection should be < 800ms (config default)");

    // Verify: End-to-end latency
    let total = trace.duration();
    assert!(total < Duration::from_secs(2),
        "Complete pipeline should be < 2s for short phrase");

    // Verify: No errors in trace
    assert!(!trace.has_errors(), "Trace should have no errors");
}
```

---

## Phase 3: Update Documentation (Immediate)

### 3.1 Update `docs/dev/TESTING.md`

**File**: `docs/dev/TESTING.md`
**Action**: **ADD SECTION**
**Time Estimate**: 2 hours

**New Section to Add**:

```markdown
## Testing Philosophy: Large-Span, Behavior-First

ColdVox follows the **Pragmatic Test Architect** philosophy:

### Core Principles

1. **Test at the highest meaningful level**
   - Default: Write Service/Integration tests (70% of suite)
   - Only drop to unit tests for complex algorithms
   - Prefer E2E tests for critical user journeys

2. **One comprehensive test > Ten fragmented tests**
   - Example: Instead of testing watchdog timer in isolation, test "audio pipeline recovers from disconnection"
   - Tests should tell complete stories about user value

3. **Real dependencies over mocks**
   - Use TestContainers, fakes, or real services
   - Mocks only for external services we don't control
   - Every mock should have a corresponding real test

4. **Behavior over implementation**
   - Tests should verify user-facing outcomes
   - Tests shouldn't break when you refactor
   - Focus on "what" not "how"

### Test Distribution Target

| Layer | Percentage | When to Use | Example |
|-------|-----------|-------------|---------|
| **Service/Integration** | 70% | Default for all features | Audio capture → VAD → STT flow |
| **E2E/Trace** | 15% | Critical user journeys | Complete dictation session |
| **Pure Logic** | 10% | Complex algorithms only | RMS calculation edge cases |
| **Contract** | 5% | External service boundaries | Vosk model API |

### Decision Framework: When to Write Which Test

**Write an E2E test when:**
- Testing a critical business flow (e.g., "user dictates and text appears")
- Testing error recovery across multiple services
- Verifying latency budgets
- Maximum: 10-15 E2E tests total

**Write a Service/Integration test when:**
- **ALMOST ALWAYS - This is your default**
- Testing any feature or behavior
- Verifying component interactions
- Testing error handling within a service
- This should be 70% of your test suite

**Write a Pure Logic test when:**
- Algorithm complexity > 20 lines
- Parsing complex formats (WAV, config files)
- Mathematical calculations
- **Ask yourself: "Can this be part of a larger test?"**

### The Six Mental Models

Before writing any test, ask:

1. **External Observer**: What would a user expect to see happen?
2. **Real Action**: Can this test perform a real action that proves the system works?
3. **Larger Span**: Could this be part of a bigger, more meaningful test?
4. **Failure Clarity**: If this fails, will I know behavior is broken (not just code changed)?
5. **Story**: Does this test tell a complete story about user value?
6. **No-Mock Challenge**: How can I eliminate every mock in this test?

### Examples

#### ❌ Bad: Fragmented Unit Tests
```rust
#[test] fn test_validate_input() { ... }
#[test] fn test_process_data() { ... }
#[test] fn test_save_result() { ... }
#[test] fn test_send_notification() { ... }
// 4 tests, no complete story
```

#### ✅ Good: Comprehensive Integration Test
```rust
#[tokio::test]
async fn test_audio_pipeline_processes_speech_end_to_end() {
    // Complete story: Audio in → VAD → STT → Injection
    let audio = load_test_audio("speech.wav");
    let pipeline = create_pipeline(audio);

    let result = pipeline.process().await;

    // Verify user-facing outcome
    assert!(result.text_injected.contains("expected phrase"));
    // One test proves the complete feature works
}
```

### Anti-Patterns to Avoid

1. **The Mock Maze**: Tests with 5+ mocks that test mocking, not behavior
2. **The Implementation Mirror**: Tests that check private state/methods
3. **The Fragmentation Fallacy**: 10 small tests instead of 1 comprehensive test
4. **The Speed Excuse**: "Integration tests are slow" (modern tools make them fast!)

### Further Reading

- See `docs/testing/PRAGMATIC_TEST_ANALYSIS.md` for detailed analysis
- See `docs/testing/PRAGMATIC_TEST_IMPROVEMENTS.md` for implementation plan
- See `docs/testing/PRAGMATIC_TEST_ARCHITECT.md` for complete philosophy
```

---

### 3.2 Create Testing Examples Document

**New File**: `docs/testing/TESTING_EXAMPLES.md`
**Time Estimate**: 2 hours

```markdown
# Testing Examples: Good vs Bad

This document shows concrete examples of tests transformed from fragmented/mocked to large-span/behavioral.

## Example 1: Audio Watchdog

### ❌ Before: 6 Fragmented Tests
```rust
#[test] fn test_watchdog_creation() { ... }
#[test] fn test_watchdog_feed_prevents_timeout() { ... }
#[test] fn test_watchdog_timeout_triggers() { ... }
#[test] fn test_watchdog_stop_resets_trigger() { ... }
#[test] fn test_restart_does_not_carry_trigger_state() { ... }
#[test] fn test_concurrent_feed_operations() { ... }
```

**Problems:**
- Tests implementation details (timer behavior)
- Doesn't prove user value
- Breaks on refactor
- Doesn't test real scenario

### ✅ After: 1 Integration Test + 1 Algorithm Test
```rust
#[tokio::test]
async fn test_audio_pipeline_auto_recovers_from_disconnect() {
    // Tests complete user story:
    // "Dictation continues working even if mic briefly disconnects"

    let capture = AudioCapture::new(config).unwrap();
    capture.start(None).await.unwrap();

    // Verify audio flowing
    let frames_before = capture.get_stats().frames_captured;
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert!(capture.get_stats().frames_captured > frames_before);

    // Simulate disconnect
    capture.simulate_disconnect();
    tokio::time::sleep(Duration::from_secs(6)).await;

    // Verify auto-recovery
    assert_eq!(capture.get_stats().recovery_attempts, 1);

    // Verify audio flowing again
    let frames_after_recovery = capture.get_stats().frames_captured;
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert!(capture.get_stats().frames_captured > frames_after_recovery);
}

#[test]
fn test_watchdog_timer_algorithm() {
    // Keep ONE test for pure algorithm edge cases
    let timer = WatchdogTimer::new(Duration::from_millis(100));

    // Feed regularly → no timeout
    for _ in 0..5 {
        sleep(50);
        timer.feed();
    }
    assert!(!timer.is_triggered());

    // Don't feed → timeout
    sleep(150);
    assert!(timer.is_triggered());
}
```

**Benefits:**
- Tests user-facing behavior
- Won't break on refactor
- Proves the feature works end-to-end
- Tells a complete story

---

## Example 2: Text Injection

### ❌ Before: Mock-Heavy Test
```rust
#[test]
fn test_injection_manager() {
    let mock_injector = MockInjector::new();
    mock_injector.expect_inject()
        .times(1)
        .returning(|_| Ok(()));

    let manager = InjectionManager::new(mock_injector);
    manager.inject("test").unwrap();

    // What did we prove? Only that mocks work.
}
```

### ✅ After: Real Injection Test
```rust
#[tokio::test]
async fn test_text_injection_into_real_terminal() {
    // Use REAL terminal application
    let terminal = spawn_test_terminal().await.unwrap();
    let capture_file = terminal.capture_file();

    // Use REAL injection backend
    let manager = StrategyManager::new(InjectionConfig::default()).await;

    // Perform REAL injection
    manager.inject("Hello from ColdVox").await.unwrap();

    // Verify REAL outcome
    let captured = fs::read_to_string(capture_file).unwrap();
    assert!(captured.contains("Hello from ColdVox"));

    // This proves the feature actually works!
}
```

---

## Example 3: Complete User Journey

### ✅ Best: End-to-End Test
```rust
#[tokio::test]
async fn test_complete_dictation_with_error_recovery() {
    """
    Story: User speaks, STT temporarily fails, system recovers,
    user continues dictating without interruption.
    """

    // Real audio with known content
    let audio = load_test_audio("dictation_session.wav");

    // Real pipeline with failable STT
    let stt = FailableSTTService::new();
    let pipeline = DictationPipeline::new()
        .with_audio(audio)
        .with_stt(stt.clone())
        .build();

    // Start processing
    let transcriptions = pipeline.start().await;

    // Get first 3 transcriptions
    assert_eq!(transcriptions.take(3).count(), 3);

    // Inject failure
    stt.fail_next_n_times(2);

    // System should recover
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Verify: Continues working
    assert_eq!(transcriptions.take(3).count(), 3);

    // This ONE test proves resilience end-to-end
}
```

---

## When You're Tempted to Write a Mock...

**Ask yourself:**
1. Can I use TestContainers instead? (Postgres, Redis, etc.)
2. Can I use a behavioral fake? (Simulates real service behavior)
3. Can I use the real service in test mode? (Local SMTP, in-memory queue)
4. Can I use a sandbox? (Stripe test mode, etc.)

**Only use mocks when:**
- External service we don't control (weather API, payment gateway in CI)
- Hardware we can't simulate (GPU, specialized audio device)
- Expensive operations (sending real emails, charging real cards)

**Even then:**
- Mock + Real test requirement: Include real test alongside mock test
- Use behavioral fakes over simple mocks when possible
