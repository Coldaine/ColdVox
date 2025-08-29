## Phase 3 — Voice Activity Detection (VAD)

Status: Planned (Phase 2 complete with rtrb ring buffer)

### Scope
- Integrate Silero V5 VAD (vendored ONNX crate) on 16 kHz mono audio
- Implement progressive energy VAD system (starting with Level 1 gating)
- Emit simple SpeechStart/SpeechEnd events with basic debouncing
- Process audio from ring buffer without blocking capture

Non‑Goals (Phase 3):
- Full energy VAD implementation (Level 4) - progressive enhancement later
- Pre/post-roll buffering - add when needed
- Health monitoring or auto-recovery - keep it simple
- STT integration (Phase 4)

---

## Dependencies
- Phase 2 ring buffer: `crates/app/src/audio/ring_buffer.rs` (rtrb‑based)
- Audio format: 16 kHz, mono, i16; 320-sample frames (20ms)
- Vendored VAD crate: `Forks/ColdVox-voice_activity_detector` (Silero V5, 512-sample windows)
- Reference spec: Energy VAD algorithm (see Level 3-4 implementation in `src/vad/level3.rs`)

## Architecture
Callback (CPAL) → Ring buffer (rtrb) → VAD Task → Events (SpeechStart/SpeechEnd)

Components:
- **Energy VAD**: Progressive system starting with Level 1 (simple gate)
- **Silero VAD**: ML-based detection for frames passing energy gate
- **State Machine**: Simple SILENCE/SPEECH with debounce timers

Notes:
- VAD task reads from ring buffer in batches (non-blocking)
- Buffer 512 samples for Silero window requirements
- Energy gate reduces Silero inference calls by ~60-80% in typical scenarios

## Energy VAD Levels (Progressive Enhancement)

### Level 1: Basic Energy Gate (Phase 3 MVP)
- Simple RMS energy calculation
- Fixed threshold: -40 dBFS
- Binary decision: process/skip frame
- No state tracking

```rust
fn energy_gate(frame: &[i16]) -> bool {
    let rms = calculate_rms_dbfs(frame);
    rms > -40.0
}
```

### Level 2: Adaptive Threshold (Future)
- RMS energy with dBFS calculation
- EMA noise floor tracking
- Relative threshold: floor + 9dB
- Simple hysteresis: on/off thresholds

### Level 3: Debounced State Machine (Future)
- All of Level 2
- State tracking (SILENCE/SPEECH)
- Minimum duration requirements
- Debounce timers (200ms speech, 400ms silence)
- Can operate standalone without ML VAD

### Level 4: Full Energy VAD (Future)
- All of Level 3
- Pre-emphasis filter (0.97)
- High-pass filter (100Hz)
- ZCR gating (optional)
- Pre/post-roll buffering
- Clipping detection
- Full implementation in `src/vad/level3.rs`

## Module Interface

```rust
// Trait that all energy VAD levels implement
pub trait EnergyVAD: Send {
    fn process_frame(&mut self, frame: &[i16]) -> VadDecision;
    fn reset(&mut self);
    fn metrics(&self) -> EnergyMetrics;
}

pub enum VadDecision {
    Silent,
    Active,
    Unknown,  // Used during warmup
}

// Main VAD processor
pub struct VadProcessor {
    silero: Option<SileroVAD>,
    energy: Box<dyn EnergyVAD>,  // Start with Level1
    state: VadState,
    accumulator: Vec<i16>,  // For 512-sample windows
}
```

## Contracts

Inputs:
- PCM i16, 16 kHz, mono frames from ring buffer

Outputs:
- `VadEvent::SpeechStart { frame_index: u64 }`
- `VadEvent::SpeechEnd { frame_index: u64 }`

Error handling:
- VAD init failure → log error, continue with energy VAD only (if Level 3+)
- Runtime errors → log and skip frame

Success criteria:
- Detects speech start/end on test audio
- No capture thread blocking
- Processes frames in real-time
- Energy gating reduces CPU usage measurably

## VAD Processing Pipeline

1. **Energy VAD** (Level 1 for Phase 3)
   - Calculate RMS energy in dBFS
   - Gate at -40 dBFS (configurable)
   - Skip Silero for silent frames
   - Track gating metrics

2. **Silero VAD** (for active frames)
   - Window: 512 samples (32ms) per prediction
   - Thresholds: speech_on=0.5, speech_off=0.35
   - Returns probability [0.0, 1.0]

3. **State Machine**
   - States: SILENCE → SPEECH → SILENCE
   - Debounce: min_speech=250ms, min_silence=300ms
   - Generate events on state transitions

## Configuration

Starting configuration (Phase 3 with Level 1):
```rust
VadConfig {
    energy_level: EnergyVadLevel::Basic,
    energy_threshold_dbfs: -40.0,
    silero_speech_threshold: 0.5,
    silero_silence_threshold: 0.35,
    min_speech_duration_ms: 250,
    min_silence_duration_ms: 300,
}
```

## File Structure

```
crates/app/src/audio/vad/
├── mod.rs           # VadProcessor, traits, common types
├── energy/
│   ├── mod.rs       # EnergyVAD trait and factory
│   ├── level1.rs    # Basic energy gate
│   ├── level2.rs    # Adaptive threshold (future)
│   ├── level3.rs    # Debounced state machine (future)
│   └── level4.rs    # Full implementation (future)
└── silero.rs        # Silero wrapper
```

## Metrics
- `speech_segments_total`
- `vad_frames_processed`
- `vad_frames_gated` (skipped due to energy gate)
- `energy_gate_efficiency` (% frames gated)
- `vad_errors` (if any)

## Implementation Plan

1) Energy VAD module
   - Create `crates/app/src/audio/vad/energy/` structure
   - Implement Level 1 basic gate
   - Define `EnergyVAD` trait for future levels

2) VAD processor integration
   - `crates/app/src/audio/vad/mod.rs` with `VadProcessor`
   - Integrate energy gate before Silero
   - Simple state machine for events

3) Processing task
   - Spawn task reading from ring buffer
   - Accumulate 512-sample windows
   - Apply energy gate → Silero pipeline
   - Track state with debounce timers
   - Send events via mpsc channel

4) Testing & metrics
   - Test with captured_audio.wav
   - Measure gating effectiveness
   - Verify real-time performance
   - Profile CPU usage reduction

5) Progressive enhancement path
   - Ship Phase 3 with Level 1
   - Upgrade to Level 2 when noise floor adaptation needed
   - Add Level 3 for standalone fallback capability
   - Consider Level 4 based on production requirements

## Test Cases
- Normal speech: detect start/end correctly
- Silence: high gating rate (>80%)
- Background noise: adaptive threshold (Level 2+)
- Missing ONNX model: fallback to energy VAD (Level 3+)
- Real-time performance: maintain <20ms latency

## Known Issues
- 512-sample windows don't align with 320-sample frames → need accumulation buffer
- ONNX runtime adds ~50MB to binary size
- First inference may be slow (model loading)
- Energy gate threshold may need tuning per environment

## Summary
Phase 3 implements a pragmatic VAD system with progressive energy gating and Silero ML detection. Starting with Level 1 energy gate saves significant CPU while maintaining accuracy. The modular design allows upgrading through Levels 2-4 without architectural changes, eventually reaching the full sophisticated implementation from the reference spec.