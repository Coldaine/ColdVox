---
doc_type: plan
subsystem: audio
version: "1.0"
status: active
owners: [Coldaine]
last_reviewed: 2026-02-16
---

# Phase 1 Implementation Plan: Automated Audio Quality Monitoring

**Goal:** Automatically detect and warn about audio quality issues in real-time without user configuration.

**Timeline:** 2-3 days
**Status:** 🚧 In Progress
**Started:** 2026-02-09

---

## What We're Building

A real-time audio quality monitoring system that runs in the audio capture callback and automatically detects:
1. **Too quiet** - RMS level below threshold
2. **Clipping** - Peak levels exceeding maximum
3. **Off-axis** - Speaker has moved away from mic (detected via spectral analysis)

The system will:
- Run with **minimal overhead** (< 1ms per frame)
- Provide **immediate warnings** (no latency)
- Require **zero configuration** (automatic thresholds)
- Display **visual feedback** in GUI

---

## Architecture Decisions

### Decision 1: New Crate `coldvox-audio-quality`

**Choice:** Create a separate crate instead of adding to `coldvox-audio`.

**Rationale:**
- **Separation of concerns**: Audio capture shouldn't know about quality analysis
- **Testability**: Can test quality analysis independently with synthetic audio
- **Optional feature**: Can be feature-gated if needed
- **Reusability**: Other crates can use quality analysis without depending on audio capture

**Trade-offs:**
- ✅ Better code organization
- ✅ Easier to test
- ✅ Cleaner dependencies
- ⚠️ Adds one more crate to workspace (minor overhead)

---

### Decision 2: Analysis in Audio Callback (Hot Path)

**Choice:** Run RMS/peak/FFT analysis directly in the audio callback thread.

**Rationale:**
- **Zero latency**: Warnings appear instantly when quality degrades
- **Performance feasible**: RMS (~0.01ms) + FFT (~0.5ms) = ~0.5ms total overhead
- **Frame budget**: 512 samples @ 16kHz = 32ms budget, 0.5ms is only 1.5% overhead
- **Real-time safe**: No allocations, all calculations use pre-allocated buffers

**Trade-offs:**
- ✅ Instant feedback
- ✅ Minimal latency
- ⚠️ Adds overhead to critical path (but well within budget)
- ❌ Must be real-time safe (no allocations/locks)

**Alternative considered:** Background thread
- Would add 50-100ms latency (unacceptable for moving user)
- Would require buffering audio samples
- Chosen approach is better for this use case

---

### Decision 3: Thresholds Based on dBFS

**Choice:** Use decibel Full Scale (dBFS) for level thresholds.

**Rationale:**
- **Industry standard**: All audio tools use dBFS (-∞ to 0 dB)
- **Logarithmic**: Matches human perception of loudness
- **Well-documented**: Clear meaning for thresholds
  - -40 dBFS = very quiet
  - -20 dBFS = good speech level
  - -1 dBFS = clipping threshold

**Thresholds:**
```rust
const TOO_QUIET_THRESHOLD_DBFS: f32 = -40.0;  // Below this = warning
const OPTIMAL_MIN_DBFS: f32 = -25.0;           // Optimal range starts here
const OPTIMAL_MAX_DBFS: f32 = -12.0;           // Optimal range ends here
const CLIPPING_THRESHOLD_DBFS: f32 = -1.0;     // Above this = clipping
```

**Trade-offs:**
- ✅ User-understandable (can google "what is -40 dBFS")
- ✅ Matches other audio software
- ✅ Easy to adjust empirically
- ⚠️ Requires sqrt calculation (but cheap: ~20ns)

---

### Decision 4: Off-Axis Detection via High-Freq Rolloff

**Choice:** Detect off-axis by comparing high-frequency (4-8kHz) to mid-frequency (500Hz-2kHz) energy.

**Rationale:**
- **Physics**: Cardioid mics have pronounced high-frequency rolloff off-axis (15-20 dB)
- **Measurable**: Ratio drops from ~0.7 (on-axis) to ~0.2 (off-axis)
- **Fast**: FFT is already computed, ratio is simple division
- **Robust**: Works regardless of absolute volume

**Algorithm:**
```rust
fn detect_off_axis(spectrum: &FrequencySpectrum) -> bool {
    let high_freq = spectrum.average_energy(4000.0, 8000.0);  // Sibilants
    let mid_freq = spectrum.average_energy(500.0, 2000.0);    // Fundamental
    let ratio = high_freq / mid_freq;

    ratio < OFF_AXIS_THRESHOLD  // 0.3 empirically tuned
}
```

**Trade-offs:**
- ✅ Simple and fast
- ✅ Physics-based (not arbitrary)
- ✅ Works with any absolute volume
- ⚠️ Threshold needs empirical tuning with real mic
- ⚠️ Won't work well with omnidirectional pattern (user should use cardioid)

**Alternative considered:** Machine learning classifier
- Overkill for this problem
- Requires training data
- Physics-based approach is simpler and interpretable

---

### Decision 5: Rolling Window for Stability

**Choice:** Use 500ms rolling window for RMS, 1-second hold for peak.

**Rationale:**
- **Smooth warnings**: Prevents flicker from momentary dips
- **Responsive**: 500ms is fast enough to catch movement
- **Speech cadence**: Matches natural pauses in speech

**Implementation:**
```rust
struct RmsWindow {
    samples: VecDeque<f32>,  // Pre-allocated to 8000 samples (500ms @ 16kHz)
    sum_squares: f64,
}

// Update every 32ms (512 samples)
// Window holds ~15 frames (500ms / 32ms)
```

**Trade-offs:**
- ✅ Stable warnings (no flicker)
- ✅ Fast response (500ms is acceptable)
- ⚠️ Small memory overhead (~32KB for window)
- ✅ Pre-allocated (real-time safe)

---

### Decision 6: Channel-Based Communication to UI

**Choice:** Use `tokio::sync::broadcast` channel to send quality updates to UI.

**Rationale:**
- **Non-blocking**: Audio callback never waits for UI
- **Multiple subscribers**: GUI, TUI, telemetry can all listen
- **Bounded**: Won't accumulate if UI is slow (oldest dropped)
- **Already used**: Consistent with existing codebase patterns

**Message rate limiting:**
- Only send when status **changes** (Good → TooQuiet)
- Or every 2 seconds if status unchanged (for UI updates)

**Trade-offs:**
- ✅ Non-blocking (audio callback never stalls)
- ✅ Multiple UI components can subscribe
- ✅ Bounded memory usage
- ⚠️ UI might miss some messages if overloaded (acceptable - they'll get next update)

---

### Decision 7: FFT Library Choice

**Choice:** Use `rustfft` (via `spectrum-analyzer` crate).

**Rationale:**
- **Pure Rust**: No C dependencies, easy to build
- **Fast**: SIMD-optimized
- **Small**: 512-point FFT is trivial
- **Well-tested**: Used in production audio applications

**Alternative considered:** `realfft`
- Real-valued FFT (2x faster)
- But `spectrum-analyzer` already wraps `rustfft` nicely
- Convenience outweighs marginal speed gain

**Trade-offs:**
- ✅ Easy to use
- ✅ No unsafe code
- ✅ Cross-platform
- ⚠️ Slightly slower than hand-optimized FFTW (irrelevant at 512 points)

---

### Decision 8: No PESQ/STOI in Phase 1

**Choice:** Defer PESQ/STOI to Phase 2, implement only RMS/peak/FFT in Phase 1.

**Rationale:**
- **Complexity**: PESQ requires PyO3 integration (non-trivial)
- **Performance**: PESQ is too slow for real-time (100-200ms per window)
- **MVP**: RMS/peak/off-axis covers 90% of user's needs
- **Incremental**: Easier to test and validate simple metrics first

**Trade-offs:**
- ✅ Faster to implement
- ✅ Lower risk
- ✅ Can validate approach before investing in PESQ
- ⚠️ No "speech quality score" yet (coming in Phase 2)

---

## Implementation Order

### Step 1: Create `coldvox-audio-quality` Crate (30 min)
- [ ] Add crate to workspace
- [ ] Define public API: `AudioQualityMonitor`
- [ ] Add dependencies: `rustfft`, `ringbuf` (or VecDeque)

**Why first:** Foundation for everything else.

---

### Step 2: Implement RMS/Peak Calculation (1 hour)
- [ ] `struct LevelMonitor` with rolling window
- [ ] RMS calculation (sqrt of mean of squares)
- [ ] Peak hold with decay
- [ ] Convert to dBFS: `20 * log10(level)`
- [ ] Unit tests with synthetic signals

**Why second:** Simplest metric, validates infrastructure.

**Test cases:**
```rust
#[test]
fn test_rms_silence() {
    let samples = vec![0i16; 512];
    let rms = calculate_rms(&samples);
    assert_eq!(rms, -f32::INFINITY);  // Silence = -∞ dB
}

#[test]
fn test_rms_full_scale() {
    let samples = vec![32767i16; 512];  // Max positive
    let rms = calculate_rms(&samples);
    assert_approx_eq!(rms, 0.0);  // Full scale = 0 dBFS
}
```

---

### Step 3: Implement FFT-Based Off-Axis Detection (2 hours)
- [ ] `struct SpectralAnalyzer` wrapping `spectrum-analyzer`
- [ ] Frequency band energy calculation
- [ ] High-freq / mid-freq ratio
- [ ] Threshold-based classification
- [ ] Unit tests with synthetic chirps

**Why third:** More complex than RMS, but self-contained.

**Test cases:**
```rust
#[test]
fn test_on_axis_signal() {
    // White noise (all frequencies equal)
    let samples = generate_white_noise(512);
    let is_off_axis = detect_off_axis(&samples);
    assert!(!is_off_axis);
}

#[test]
fn test_off_axis_signal() {
    // Low-pass filtered (high freqs attenuated)
    let samples = generate_lowpass_noise(512, 3000.0);
    let is_off_axis = detect_off_axis(&samples);
    assert!(is_off_axis);
}
```

---

### Step 4: Integrate into Audio Callback (2 hours)
- [ ] Add `AudioQualityMonitor` to `AudioCapture` struct
- [ ] Call `monitor.analyze(samples)` in audio callback
- [ ] Send quality status updates to broadcast channel
- [ ] Add quality event receiver to main loop
- [ ] Integration test: verify events are received

**Why fourth:** Now we connect to real audio pipeline.

**Critical:** Ensure no allocations in callback (verify with Valgrind/heaptrack).

---

### Step 5: Add Visual Feedback to GUI (3 hours)
- [ ] Create `AudioQualityWidget` for GUI
- [ ] Subscribe to quality events
- [ ] Display current status (Good/Warning/Bad)
- [ ] Show dBFS level meter
- [ ] Show specific warnings ("TOO QUIET", "OFF-AXIS")
- [ ] Color-code: Green (good), Yellow (warning), Red (bad)

**Why fifth:** User-facing feature, validates entire pipeline.

**UI mockup:**
```
┌─ Audio Quality ────────────────────────┐
│ Status: ✅ Good                         │
│ Level:  ████████░░ -18 dBFS            │
│                                         │
│ ⚠️  Warning: Slightly off-axis         │
│    Move closer to microphone           │
└─────────────────────────────────────────┘
```

---

### Step 6: Add Telemetry Logging (1 hour)
- [ ] Log quality metrics to `coldvox-telemetry`
- [ ] Track warning frequency over time
- [ ] Add metrics to `PipelineMetrics`

**Why sixth:** Enables debugging and long-term quality analysis.

**Metrics to track:**
- `audio_quality_warnings_total{type="too_quiet"}` (counter)
- `audio_quality_warnings_total{type="clipping"}` (counter)
- `audio_quality_warnings_total{type="off_axis"}` (counter)
- `audio_rms_dbfs` (gauge)
- `audio_peak_dbfs` (gauge)

---

### Step 7: Real-World Testing & Tuning (2-3 hours)
- [ ] Test with real microphone (HyperX QuadCast)
- [ ] Validate thresholds with actual movement
- [ ] Tune off-axis threshold empirically
- [ ] Test edge cases (silence, loud noise, etc.)
- [ ] User validation: does it catch real issues?

**Why seventh:** Empirical validation is critical.

**Test scenarios:**
1. Sit normally, speak at normal volume → Should be "Good"
2. Speak very quietly → Should warn "Too quiet"
3. Speak very loudly → Should warn "Clipping"
4. Turn 90° away from mic → Should warn "Off-axis"
5. Move 5 feet away → Should warn "Too quiet" or "Off-axis"

---

### Step 8: Documentation & Cleanup (1 hour)
- [ ] Add rustdoc comments to public API
- [ ] Update CHANGELOG.md
- [ ] Write user-facing docs: "Understanding Audio Quality Warnings"
- [ ] Add troubleshooting guide

**Why last:** Polish and knowledge transfer.

---

## Performance Budget

**Audio callback budget:** 32ms per frame (512 samples @ 16kHz)

**Our overhead:**
- RMS calculation: ~0.01ms (10 microseconds)
- Peak detection: ~0.005ms (5 microseconds)
- FFT (512-point): ~0.5ms (500 microseconds)
- Spectral ratio: ~0.01ms (10 microseconds)
- **Total: ~0.525ms (525 microseconds)**

**Percentage of budget:** 0.525ms / 32ms = **1.6%**

✅ **Well within budget.** Leaves 98.4% for existing audio processing.

---

## Risk Mitigation

### Risk 1: FFT Allocates in Hot Path
**Likelihood:** Medium
**Impact:** High (would cause audio glitches)

**Mitigation:**
- Pre-allocate FFT planner and buffers outside callback
- Use `#[cfg(test)]` with allocation tracker in unit tests
- Manual testing with Valgrind or heaptrack

**Contingency:** If FFT is too slow, defer to background thread with 50ms latency.

---

### Risk 2: Off-Axis Threshold Not Universal
**Likelihood:** Medium
**Impact:** Medium (false positives/negatives)

**Mitigation:**
- Make threshold configurable (`COLDVOX_OFF_AXIS_THRESHOLD` env var)
- Log spectral ratios during testing to tune empirically
- Provide calibration tool: "Speak on-axis, speak off-axis, we'll learn threshold"

**Contingency:** If too noisy, add hysteresis (different thresholds for on→off vs off→on).

---

### Risk 3: Too Many Warnings (Alert Fatigue)
**Likelihood:** Medium
**Impact:** Medium (user ignores warnings)

**Mitigation:**
- Only warn on state **changes** (Good → Bad)
- Add 2-second cooldown between repeated warnings
- Provide "dismiss" button to suppress warnings for 5 minutes

**Contingency:** Add ML-based "importance" scoring (Phase 4).

---

## Testing Strategy

### Unit Tests
- RMS calculation with known signals
- Peak detection with synthetic pulses
- FFT with sine waves (verify frequency peaks)
- Off-axis detection with filtered noise

### Integration Tests
- Inject test audio into `AudioCapture`
- Verify quality events are emitted
- Verify events reach UI

### Manual Testing
- Real microphone with real speech
- Movement scenarios (turn away, move back)
- Edge cases (silence, shouting, background noise)

### Performance Testing
- Run audio callback for 1 hour, measure max latency
- Profile with `perf` to verify no allocations
- Stress test with rapid status changes

---

## Success Criteria

**Phase 1 is complete when:**

1. ✅ System detects "too quiet" automatically (RMS < -40 dBFS)
2. ✅ System detects clipping automatically (peak > -1 dBFS)
3. ✅ System detects off-axis automatically (spectral ratio < 0.3)
4. ✅ GUI displays current quality status and warnings
5. ✅ Audio callback overhead < 2% of frame budget
6. ✅ No allocations in audio callback (verified)
7. ✅ User validation: "Yes, it catches when I move away"

**User acceptance test:**
User moves around room while speaking. System correctly identifies:
- When they're too far from mic
- When they turn away
- When mic gain is too low
- When mic gain is too high (clipping)

---

## Why These Choices Matter

### 1. Real-Time Analysis in Callback
**User moves around constantly.** Any latency (50-100ms) means warnings lag behind reality. By the time user sees "off-axis", they've already moved back. Real-time analysis ensures warnings are **actionable**.

### 2. Physics-Based Detection (Not ML)
**Interpretable and debuggable.** When off-axis detection triggers, we know exactly why (high-freq rolloff). We can tune the threshold empirically. ML would be a black box requiring training data we don't have yet.

### 3. Minimal Overhead (<2%)
**Audio glitches are unacceptable.** Users will tolerate slightly imperfect warnings, but they will **not** tolerate choppy audio. 1.6% overhead leaves plenty of headroom.

### 4. Zero Configuration
**User wants it to "just work."** No sliders to adjust, no calibration wizard. Reasonable defaults that work for 90% of users, with escape hatches (env vars) for power users.

### 5. Visual Feedback First, PESQ Later
**Ship fast, iterate.** Visual level meter + warnings gets user 80% of value in 20% of time. PESQ is nice-to-have but not critical for MVP. If visual feedback solves the problem, we might not even need PESQ.

---

## Next Steps

1. **Immediate:** Create `coldvox-audio-quality` crate
2. **Today:** Implement RMS/peak calculation with tests
3. **Tomorrow:** Add FFT-based off-axis detection
4. **Day 3:** Integrate into audio callback and GUI

**Estimated completion:** End of week (2026-02-11)

**Follow-up:** Phase 2 (PESQ/STOI) begins after user validation of Phase 1.
