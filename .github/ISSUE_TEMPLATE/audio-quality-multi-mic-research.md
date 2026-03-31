---
name: Audio Quality & Multi-Microphone Capture Research
about: Investigate speech recognition quality issues and explore multi-mic capabilities
title: 'Audio Quality & Multi-Microphone Capture Research'
labels: enhancement, research
assignees: ''
---

## Overview

Investigate and implement improvements for speech recognition quality and explore multi-microphone capture capabilities.

## Background

Current setup:
- **Microphone**: HyperX QuadCast with 4 polar patterns (Stereo, Omnidirectional, Cardioid, Bidirectional)
- **Use case**: Area mic capture (not close-talk), need better feedback and quality
- **Problem**: Poor speech recognition quality - unclear if it's volume, positioning, or audio quality issue

## 1. Speech Recognition Quality Investigation

### Current Concerns
- Unclear audio quality vs volume vs positioning
- Using area mics (not close-talk)
- Need diagnostic feedback for mic placement and audio quality

### Recommended Setup for HyperX QuadCast
Per [HyperX documentation](https://row.hyperx.com/products/hyperx-quadcast-usb-microphone):
- **Polar pattern**: Cardioid (best for single-person speech)
- **Distance**: At least 1 foot to avoid plosives (p-, t-, k- sounds) and fricatives (f-, th- sounds)
- **Gain control**: Adjust dial at bottom of QuadCast

### Action Items
- [ ] Add gain/volume monitoring to detect "too quiet" audio
- [ ] Implement off-axis detection to warn when speaker is outside mic's polar pattern
- [ ] Add RMS/peak level visualization to help with positioning

---

## 2. Multi-Microphone Capture Research

### Windows WASAPI Support
✅ **WASAPI supports multiple simultaneous microphone sources** ([Microsoft docs](https://learn.microsoft.com/en-us/windows/win32/coreaudio/wasapi))

Recent developments (Jan 2026):
- [Pull request #6041](https://github.com/joncampbell123/dosbox-x/pull/6041) added WASAPI microphone input with:
  - Circular buffer for capture
  - Sample rate conversion (48kHz → various rates)
  - Format conversion support

**Technical approach**:
- Open multiple `IAudioClient` instances (one per mic)
- Each in shared mode
- Mix streams in dedicated mixer thread

### Beamforming Libraries

**Rust ecosystem** (limited mature options):
- [acoustic-array-tools](https://github.com/mcbridejc/acoustic-array-tools) - Experimental beamforming for microphone arrays
  - PDM → PCM conversion
  - FFT for frequency spectra
  - Delay-and-sum in frequency domain
  - Status: Working/experimental, not production-ready

**Python ecosystem** (mature):
- [SpeechBrain multi-microphone beamforming](https://speechbrain.readthedocs.io/en/v1.0.2/tutorials/preprocessing/multi-microphone-beamforming.html)
- [MicArrayBeamforming](https://github.com/MiguelBlancoGalindo/MicArrayBeamforming)
- Pyroomacoustics (widely used)

**Considerations**:
- Most production beamforming is in Python/MATLAB
- Rust implementations are experimental
- Could use PyO3 to call Python beamforming libraries

### Spatial Audio Analysis
From [recent research](https://arxiv.org/html/2507.03466v1) on direction estimation:
- Microphone arrays can detect sound source direction
- Spherical arrays with 32 elements can steer beams in 60 directions
- Metrics: Directivity Factor Q, half-power Beam Width BW

### Action Items
- [ ] Research feasibility of multi-mic capture with current CPAL setup
- [ ] Prototype dual-mic capture (if user has 2 QuadCast mics or similar)
- [ ] Evaluate Python beamforming libraries via PyO3 integration
- [ ] Consider simple delay-and-sum beamforming for 2-mic setup

---

## 3. Audio Quality Feedback & Visualization

### Rust Crates for Real-Time Visualization

**[meter](https://github.com/cgbur/meter)** - CLI level meter in Rust
- Displays microphone gain in dBFS
- Helps detect clipping
- Useful for knowing when gain is set appropriately

**[spectrum-analyzer](https://crates.io/crates/spectrum-analyzer)**
- FFT-based frequency spectrum
- `no_std` library with `alloc`
- Real-time audio visualization
- Fast and easy to use

**[audio-visualizer](https://crates.io/crates/audio-visualizer)**
- Waveform and spectrum visualization
- GUI window with live audio data
- Good for debugging audio samples
- [Tutorial available](https://phip1611.de/blog/live-audio-visualization-with-rust-in-a-gui-window/)

### Python Libraries for Quality Assessment

**[audio-quality-analyzer](https://github.com/yashvyas7/audio-quality-analyzer)** - Comprehensive toolkit
- **PESQ** (Perceptual Evaluation of Speech Quality): ITU-T P.862 standard, 1-5 scale
- **STOI** (Short-Time Objective Intelligibility): 0-1 scale (1 = perfect)
- Spectral distances and perceptual features
- Supports single-file and comparative analysis

**[speechmetrics](https://github.com/aliutkus/speechmetrics)** - Unified wrapper
- MOSNet, BSSEval, STOI, PESQ, SRMR, SISDR
- Single interface for multiple metrics

**Individual metrics**:
- [PESQ](https://pypi.org/project/pesq/) - Requires 16kHz or 8kHz sample rate
- [pystoi](https://github.com/mpariente/pystoi) - Short-Time Objective Intelligibility
- [pysepm](https://github.com/schmiph2/pysepm) - Performance metrics from Loizou's Speech Enhancement book

### Directivity & Off-Axis Detection
From [speaker directivity research](https://acousticfrontiers.com/blogs/articles/speaker-directivity-off-axis-response-theory-and-measurement-techniques):
- Measure impulse response at various angles
- Frequency-dependent directivity patterns
- Can detect when sound source is off-axis

### Action Items
- [ ] Integrate `meter` crate for real-time dBFS display
- [ ] Add spectrum analyzer visualization (frequency domain)
- [ ] Implement RMS/peak level monitoring with configurable thresholds
- [ ] Prototype PESQ/STOI integration via PyO3 for quality scoring
- [ ] Create visual feedback system:
  - "Too quiet" warning (RMS < threshold)
  - "Clipping" warning (peak > -1 dBFS)
  - "Off-axis" detection (spectral analysis of high-frequency rolloff)
  - Real-time VU meter in TUI

---

## 4. Implementation Roadmap

### Phase 1: Basic Monitoring (Quick Wins)
1. Add RMS/peak level calculation to existing audio pipeline
2. Display dBFS levels in TUI dashboard
3. Add configurable thresholds for "too quiet" warnings
4. Log audio quality metrics to help diagnose issues

### Phase 2: Visualization
1. Integrate `spectrum-analyzer` crate
2. Add real-time frequency spectrum to TUI
3. Create VU meter widget in GUI
4. Add visual indicators for:
   - Input level (good/low/clipping)
   - Frequency response (detect off-axis via high-freq rolloff)

### Phase 3: Quality Assessment
1. Prototype PESQ integration via PyO3
2. Add STOI for intelligibility scoring
3. Provide real-time feedback: "Speech quality: 4.2/5 (PESQ)"
4. Log quality metrics for post-analysis

### Phase 4: Multi-Mic (Research)
1. Verify CPAL can open multiple devices simultaneously
2. Test dual QuadCast capture
3. Prototype simple delay-and-sum beamforming
4. Evaluate Python beamforming libraries

---

## 5. Technical Considerations

### Current Pipeline Integration Points
- **Audio capture** (`crates/coldvox-audio/src/capture.rs:429-452`): Add level metering in callback
- **Ring buffer** (`crates/coldvox-audio/src/ring_buffer.rs`): Already non-blocking, ready for multi-source
- **TUI** (`crates/app/src/bin/tui_dashboard.rs`): Add audio quality widgets
- **Telemetry** (`crates/coldvox-telemetry`): Add audio quality metrics

### Performance Concerns
- PESQ/STOI are computationally expensive (not real-time capable)
- Run quality assessment on separate thread
- Use ring buffer to pass audio to quality assessment task
- Display results with 1-2 second lag

### Multi-Mic Challenges
- Synchronization between streams (clock drift)
- Latency differences between devices
- Mixer thread required to combine streams
- CPAL may require separate `Stream` instances per device

---

## References

**Multi-Microphone/Beamforming**:
- [Acoustic Beamforming Notes](https://jeffmcbride.net/acoustic-beamforming/)
- [acoustic-array-tools (Rust)](https://github.com/mcbridejc/acoustic-array-tools)
- [SpeechBrain Beamforming](https://speechbrain.readthedocs.io/en/v1.0.2/tutorials/preprocessing/multi-microphone-beamforming.html)
- [Direction Estimation with Microphone Arrays](https://arxiv.org/html/2507.03466v1)

**Windows Multi-Mic**:
- [WASAPI Documentation](https://learn.microsoft.com/en-us/windows/win32/coreaudio/wasapi)
- [WASAPI Microphone Support PR](https://github.com/joncampbell123/dosbox-x/pull/6041)

**Audio Quality/Visualization**:
- [meter (Rust CLI level meter)](https://github.com/cgbur/meter)
- [spectrum-analyzer (Rust)](https://crates.io/crates/spectrum-analyzer)
- [audio-visualizer (Rust)](https://crates.io/crates/audio-visualizer)
- [Live Audio Visualization Tutorial](https://phip1611.de/blog/live-audio-visualization-with-rust-in-a-gui-window/)

**Quality Metrics**:
- [audio-quality-analyzer (Python PESQ/STOI)](https://github.com/yashvyas7/audio-quality-analyzer)
- [speechmetrics (Python)](https://github.com/aliutkus/speechmetrics)
- [PESQ PyPI](https://pypi.org/project/pesq/)
- [pystoi](https://github.com/mpariente/pystoi)

**HyperX QuadCast**:
- [QuadCast Product Page](https://row.hyperx.com/products/hyperx-quadcast-usb-microphone)
- [QuadCast User Manual](https://media.kingston.com/support/downloads/HyperX_QuadCast_Microhone_Manual.pdf)

---

## Next Steps

1. **Immediate**: Add basic RMS/peak level monitoring (Phase 1)
2. **Short-term**: Integrate spectrum analyzer visualization (Phase 2)
3. **Medium-term**: Prototype PESQ/STOI quality assessment (Phase 3)
4. **Research**: Multi-mic capture feasibility study (Phase 4)

**Priority**: Start with Phase 1 (basic monitoring) to diagnose current speech recognition issues before investing in multi-mic infrastructure.
