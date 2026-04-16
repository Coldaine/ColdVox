---
doc_type: research
subsystem: audio
version: "0.1"
status: active
owners: [Coldaine]
last_reviewed: 2026-02-16
---

# Automated Audio Quality Monitoring & Multi-Mic Capture

**Labels:** enhancement, audio, research
**Priority:** High

---

## Problem

**Speech recognition quality is poor and inconsistent.** User moves around while speaking (not stationary at mic), using area mics. Need **automated, real-time feedback** that detects and warns about audio quality issues without manual intervention.

**Current setup:**
- HyperX QuadCast (4 polar patterns: Stereo, Omni, Cardioid, Bidirectional)
- Area mic usage (not close-talk)
- User moves around during capture

## Goal: Automated Audio Quality Analysis

Build a system that **automatically detects and warns** about:
- ❌ **"Too quiet"** - insufficient volume/gain
- ❌ **"Clipping"** - audio distortion from too much gain
- ❌ **"Off-axis"** - speaker is outside mic's pickup pattern (moved away, turned around)
- ❌ **"Poor quality"** - garbled/muffled audio
- ✅ **"Good quality"** - optimal capture conditions

**This should be real-time, automatic, and require zero manual setup.**

---

## Solution: Multi-Layer Audio Quality Monitor

### Layer 1: Real-Time Level Analysis (Immediate)

**What it detects:**
- Volume too low (RMS < -40 dBFS)
- Clipping (peaks > -1 dBFS)
- Dynamic range issues

**Implementation:**
```rust
// Add to audio callback (crates/coldvox-audio/src/capture.rs:429-452)
struct AudioQualityMonitor {
    rms_window: RingBuffer<f32>,      // Rolling RMS over 1 second
    peak_hold: f32,                    // Peak level in last 3 seconds
    quality_status: QualityStatus,     // Good/Warning/Bad
}

enum QualityStatus {
    Good,
    TooQuiet { rms_db: f32 },
    Clipping { peak_db: f32 },
    OffAxis { high_freq_rolloff: f32 },
}

// Calculate in audio callback (< 1ms overhead)
fn analyze_frame(&mut self, samples: &[i16]) {
    let rms = calculate_rms(samples);
    let peak = samples.iter().map(|&s| s.abs()).max();

    if rms_db < -40.0 {
        self.status = TooQuiet;
    } else if peak_db > -1.0 {
        self.status = Clipping;
    }
}
```

**Tools:**
- [meter](https://github.com/cgbur/meter) - dBFS calculation
- Custom RMS/peak detector (trivial to implement)

---

### Layer 2: Frequency Analysis (Real-Time)

**What it detects:**
- Off-axis detection (high-frequency rolloff when speaker turns away)
- Proximity effect (too much bass = too close)
- Room noise/muffling

**Implementation:**
```rust
// Use spectrum-analyzer crate
use spectrum_analyzer::{samples_fft_to_spectrum, FrequencyLimit};

fn detect_off_axis(samples: &[i16]) -> bool {
    let spectrum = samples_fft_to_spectrum(&samples, 16000, FrequencyLimit::All);

    // Calculate high-freq energy ratio (4kHz-8kHz vs 500Hz-2kHz)
    let high_freq = spectrum.freq_range(4000.0, 8000.0).sum();
    let mid_freq = spectrum.freq_range(500.0, 2000.0).sum();
    let ratio = high_freq / mid_freq;

    // If ratio < threshold, speaker is off-axis or muffled
    ratio < 0.3  // Empirically tuned
}
```

**Tools:**
- [spectrum-analyzer](https://crates.io/crates/spectrum-analyzer) - FFT in < 1ms
- Custom spectral features

**Science:** When you turn away from a cardioid mic, high frequencies (4-8kHz) drop off dramatically. This is measurable in real-time.

---

### Layer 3: Speech Quality Scoring (Every 2-3 seconds)

**What it detects:**
- Overall speech quality (PESQ score 1-5)
- Intelligibility (STOI score 0-1)
- Distortion, noise, artifacts

**Implementation:**
```rust
// Separate thread, not real-time critical
struct QualityScorer {
    pesq_handle: PyObject,  // Via PyO3
    audio_buffer: RingBuffer<i16>,  // 3-second window
}

async fn score_audio_quality(&mut self) {
    let samples = self.audio_buffer.read_last_3_seconds();

    // Call Python PESQ library
    let pesq_score = self.pesq_handle.call("pesq", (16000, samples))?;

    if pesq_score < 2.5 {
        warn!("Poor speech quality: {:.1}/5.0", pesq_score);
    }
}
```

**Tools:**
- [audio-quality-analyzer](https://github.com/yashvyas7/audio-quality-analyzer) (Python)
- [PESQ](https://pypi.org/project/pesq/) - ITU-T P.862 standard
- [pystoi](https://github.com/mpariente/pystoi) - Intelligibility

**Note:** PESQ/STOI are too slow for real-time (100-200ms per 3s window), so run on separate thread and update UI every 2-3 seconds.

---

### Layer 4: Visual Feedback (GUI/CLI)

**What user sees:**

**Real-time status bar:**
```
🎤 Audio Quality: ✅ Good  │  Level: ████████░░ -18 dB  │  PESQ: 4.2/5.0
```

**When issues detected:**
```
⚠️  TOO QUIET - Speak louder or increase mic gain
🔴 CLIPPING - Reduce mic gain (dial on bottom of QuadCast)
⚠️  OFF-AXIS - Move back in front of microphone
⚠️  POOR QUALITY (2.1/5.0) - Check mic positioning
```

**Visual meter (GUI/TUI):**
```
 Volume:  ▂▃▅▇█▇▅▃▂  Good
 Quality: ████████░░  4.2/5.0

 Frequency Spectrum:
 8kHz  ░░░░░░░█░░
 4kHz  ░░░░░███░░
 2kHz  ░░░████░░░
 1kHz  ░░███░░░░░
 500Hz ░███░░░░░░
```

**Implementation:**
- [ratatui](https://ratatui.rs/) - Already used in TUI
- Custom widgets for VU meter and spectrum
- Color-coded status indicators

---

## Multi-Microphone Capture (Phase 2)

### Windows Support
✅ **WASAPI supports multiple simultaneous mics** ([docs](https://learn.microsoft.com/en-us/windows/win32/coreaudio/wasapi))

**Approach:**
```rust
// Open multiple CPAL streams
let mic1 = AudioCapture::new("HyperX QuadCast 1")?;
let mic2 = AudioCapture::new("HyperX QuadCast 2")?;

// Mix streams
let mixer = AudioMixer::new(vec![mic1, mic2]);
mixer.set_mode(MixMode::BeamformCardioid);  // Directional focus
```

**Benefits of dual mics:**
- Beamforming: Focus on speaker direction, reject room noise
- Better off-axis detection: Compare levels between mics
- Redundancy: If one mic has issues, use the other

**Libraries:**
- [acoustic-array-tools](https://github.com/mcbridejc/acoustic-array-tools) (Rust, experimental)
- [SpeechBrain](https://speechbrain.readthedocs.io/) (Python beamforming via PyO3)
- Custom delay-and-sum beamforming (simple, effective)

### Beamforming Basics

**Delay-and-sum** (simplest approach):
```rust
// If speaker is in front, signals arrive at both mics at same time
// If speaker is off to side, signal arrives at one mic slightly delayed
// By aligning and summing, we enhance on-axis sound and reject off-axis noise

fn beamform(mic1: &[i16], mic2: &[i16], delay_samples: usize) -> Vec<i16> {
    mic1.iter()
        .zip(mic2.iter().skip(delay_samples))
        .map(|(&a, &b)| (a as i32 + b as i32) / 2)
        .collect()
}
```

**Adaptive beamforming** (advanced):
- Automatically steers toward loudest source
- Rejects noise from other directions
- Requires Python libraries (SpeechBrain, Pyroomacoustics)

---

## Implementation Roadmap

### Phase 1: Automated Quality Monitoring (2-3 days)
**Goal:** Detect and warn about audio issues in real-time

- [ ] Add RMS/peak calculation to audio callback
- [ ] Implement spectral analysis for off-axis detection
- [ ] Create QualityStatus enum and detection logic
- [ ] Add visual feedback to GUI (TUI being deprecated)
- [ ] Test with real usage (moving around, varying distance)

**Output:** Real-time warnings when audio quality degrades

---

### Phase 2: Speech Quality Scoring (1-2 days)
**Goal:** Add PESQ/STOI scoring for objective quality metrics

- [ ] Integrate PESQ via PyO3
- [ ] Add STOI for intelligibility
- [ ] Create background thread for scoring
- [ ] Display scores in UI
- [ ] Log scores for analysis

**Output:** Numerical quality scores updated every 2-3 seconds

---

### Phase 3: Multi-Mic Capture (1 week, research-heavy)
**Goal:** Support dual-mic setup with beamforming

- [ ] Verify CPAL can open multiple devices simultaneously
- [ ] Implement audio mixer for combining streams
- [ ] Test synchronization between devices
- [ ] Prototype delay-and-sum beamforming
- [ ] Evaluate Python beamforming libraries
- [ ] Compare single-mic vs dual-mic quality

**Output:** Option to use 2 mics for better quality and directionality

---

### Phase 4: Advanced Features (Future)
- [ ] Automatic gain control (AGC)
- [ ] Noise suppression
- [ ] Echo cancellation (if needed)
- [ ] Directional beamforming with 3+ mics
- [ ] ML-based quality prediction

---

## Technical Integration Points

### Existing Code Modifications

**1. Audio callback** (`crates/coldvox-audio/src/capture.rs:429-452`)
```rust
let handle_i16 = move |i16_data: &[i16]| {
    // Existing code...
    watchdog.feed();

    // NEW: Quality analysis
    let quality = quality_monitor.analyze(i16_data);
    if quality.needs_warning() {
        quality_tx.send(quality)?;  // Send to UI thread
    }

    // Existing code...
    audio_producer.lock().write(i16_data);
};
```

**2. New crate**: `crates/coldvox-audio-quality`
```
src/
  monitor.rs       - AudioQualityMonitor
  metrics.rs       - RMS, peak, spectral features
  scorer.rs        - PESQ/STOI integration
  types.rs         - QualityStatus, thresholds
```

**3. UI updates** (GUI - TUI being phased out)
- Add status bar widget
- Add VU meter widget
- Add spectrum analyzer widget
- Color-coded warnings

**4. Telemetry** (`crates/coldvox-telemetry`)
- Add quality metrics
- Log quality scores
- Track warnings over time

---

## Performance Considerations

**Real-time requirements:**
- Audio callback must complete in < 32ms (512 samples @ 16kHz)
- RMS/peak calculation: ~0.01ms ✅
- FFT (512 samples): ~0.5-1ms ✅
- PESQ/STOI: ~100-200ms ❌ (must be on separate thread)

**Solution:**
- Run RMS/peak/FFT in audio callback (fast)
- Send audio to background thread for PESQ/STOI
- Update UI with 1-2 second lag for quality scores
- Warnings appear immediately for level/clipping/off-axis

---

## HyperX QuadCast Optimization

**Recommended settings:**
- **Polar pattern**: Cardioid (front-facing, rejects sides/back)
- **Gain**: Start at 50%, adjust based on RMS feedback
- **Distance**: 1-3 feet (closer = more bass, farther = more room noise)
- **Positioning**: Mic should point at your mouth, not top of head

**Why Cardioid?**
- Rejects sound from behind/sides by 15-20 dB
- Easier to detect off-axis (high-freq rolloff is pronounced)
- Best for single-person speech in noisy room

**Gain dial location:** Bottom of QuadCast (rotate clockwise to increase)

---

## References

**Audio Quality Analysis:**
- [meter (Rust dBFS)](https://github.com/cgbur/meter)
- [spectrum-analyzer (Rust FFT)](https://crates.io/crates/spectrum-analyzer)
- [audio-visualizer (Rust)](https://crates.io/crates/audio-visualizer)
- [audio-quality-analyzer (Python PESQ/STOI)](https://github.com/yashvyas7/audio-quality-analyzer)
- [PESQ PyPI](https://pypi.org/project/pesq/)
- [pystoi GitHub](https://github.com/mpariente/pystoi)

**Multi-Mic/Beamforming:**
- [WASAPI docs](https://learn.microsoft.com/en-us/windows/win32/coreaudio/wasapi)
- [WASAPI mic support PR](https://github.com/joncampbell123/dosbox-x/pull/6041)
- [acoustic-array-tools (Rust)](https://github.com/mcbridejc/acoustic-array-tools)
- [SpeechBrain beamforming](https://speechbrain.readthedocs.io/)
- [Direction estimation research](https://arxiv.org/html/2507.03466v1)
- [Directivity measurement](https://acousticfrontiers.com/blogs/articles/speaker-directivity-off-axis-response-theory-and-measurement-techniques)

**HyperX QuadCast:**
- [Product page](https://row.hyperx.com/products/hyperx-quadcast-usb-microphone)
- [User manual](https://media.kingston.com/support/downloads/HyperX_QuadCast_Microhone_Manual.pdf)

---

## Success Criteria

**Phase 1 complete when:**
- ✅ User gets real-time warnings when audio quality degrades
- ✅ System automatically detects: too quiet, clipping, off-axis
- ✅ Visual feedback shows current quality status
- ✅ No manual configuration required

**Phase 2 complete when:**
- ✅ PESQ/STOI scores displayed in real-time
- ✅ Quality trends logged for analysis
- ✅ Scores correlate with subjective quality

**Phase 3 complete when:**
- ✅ Dual-mic capture working on Windows
- ✅ Beamforming improves quality vs single mic
- ✅ Off-axis rejection measurably better

**End state:** User moves around freely, system automatically warns when audio quality drops, suggests corrections, and uses multiple mics (if available) to maintain quality.
