## Phase 3 — Voice Activity Detection (VAD)

Status: Planned (Phase 2 complete with rtrb ring buffer)

### Scope
- Integrate Silero V5 VAD (vendored ONNX crate) on 16 kHz mono audio
- Emit simple SpeechStart/SpeechEnd events with basic debouncing
- Process audio from ring buffer without blocking capture

Non‑Goals (Phase 3):
- Fallback VAD mechanisms
- Pre/post-roll buffering
- Health monitoring or auto-recovery
- STT integration (Phase 4)

---

## Dependencies
- Phase 2 ring buffer: `crates/app/src/audio/ring_buffer.rs` (rtrb‑based)
- Audio format: 16 kHz, mono, i16; 320-sample frames (20ms)
- Vendored VAD crate: `Forks/ColdVox-voice_activity_detector` (Silero V5, 512-sample windows)

## Architecture
Callback (CPAL) → Ring buffer (rtrb) → VAD Task → Events (SpeechStart/SpeechEnd)

Notes:
- VAD task reads from ring buffer in batches (non-blocking)
- Buffer 512 samples for Silero window requirements
- Simple speech/silence state tracking with debounce timers

## Contracts

Inputs:
- PCM i16, 16 kHz, mono frames from ring buffer

Outputs:
- `VadEvent::SpeechStart { frame_index: u64 }`
- `VadEvent::SpeechEnd { frame_index: u64 }`

Error handling:
- VAD init failure → log error, continue without VAD
- Runtime errors → log and skip frame

Success criteria:
- Detects speech start/end on test audio
- No capture thread blocking
- Processes frames in real-time

## Silero VAD integration
- Sample rate: 16 kHz mono
- Window: 512 samples (32ms) per prediction
- Thresholds: speech_on=0.5, speech_off=0.35
- Debounce: min_speech=250ms, min_silence=300ms
- Energy gate: Skip Silero inference if frame energy < -40 dBFS (silence optimization)
- Simple state: SILENCE → SPEECH (on threshold) → SILENCE (off threshold + duration)

## Configuration
Hardcoded defaults (can make configurable later if needed):
- Speech threshold: 0.5
- Silence threshold: 0.35
- Min speech duration: 250ms
- Min silence duration: 300ms
- Energy gate threshold: -40 dBFS

## Metrics
Basic counters only:
- `speech_segments_total`
- `vad_frames_processed`
- `vad_frames_gated` (skipped due to low energy)
- `vad_errors` (if any)

## Implementation plan
1) VAD module structure
   - `crates/app/src/audio/vad/mod.rs` with `VadProcessor` and event types
   - Simple Silero wrapper using vendored crate

2) Processing task
   - Spawn task reading from ring buffer
   - Accumulate 512-sample windows
   - Calculate frame energy (RMS in dBFS)
   - Skip Silero if below energy gate threshold
   - Run Silero inference on active frames
   - Track state with debounce timers
   - Send events via mpsc channel

3) Integration
   - Wire into main.rs after ring buffer setup
   - Log events to verify operation

4) Testing
   - Test with captured_audio.wav
   - Verify real-time processing
   - Check speech/silence detection accuracy

## Test cases
- Normal speech: detect start/end correctly
- Background noise: avoid false positives
- Missing ONNX model: handle gracefully (skip VAD)
- Real-time performance: keep up with audio stream

## Known issues
- 512-sample windows don't align with 320-sample frames → need accumulation buffer
- ONNX runtime adds ~50MB to binary size
- First inference may be slow (model loading)

## Summary
Phase 3 adds basic voice activity detection using Silero V5. It reads audio from the ring buffer, detects speech/silence transitions with debouncing, and emits events for downstream processing.