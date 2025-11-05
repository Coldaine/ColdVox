# Golden Master Test Analysis and Fixes

## Problem Statement

The Golden Master test (`crates/app/tests/golden_master.rs`) was timing out and not producing the expected VAD events or transcription output. The issue came in two parts:

## Part 1: Wrong Code Was Running (FIXED)

### Root Cause

The `#[cfg(test)]` attribute was used to gate test-specific code in `runtime.rs`:
- Line 400: Device config override for tests
- Line 602: Mock injection sink wiring

In Rust, `#[cfg(test)]` is only active for:
- Unit tests within the same crate (tests in the same file or `mod tests`)
- NOT for integration tests in the `tests/` directory (which compile as separate crates)

This meant the integration test couldn't access the test-specific code paths, so:
1. The test device config override was compiled out
2. The mock injection sink wiring was compiled out
3. The runtime tried to use real audio devices (failing with timeouts)

### Solution

Removed the `#[cfg(test)]` attributes at lines 400 and 602 in `runtime.rs`. The test-specific behavior is already gated by runtime checks:
- `test_device_config` field presence
- `test_injection_sink` field presence

These runtime checks are sufficient and don't require compile-time gating.

### Commits
- bc4e0ad: "fix: Remove #[cfg(test)] gates blocking integration tests"

## Part 2: Silent Pipeline Failure (DIAGNOSED)

### Symptoms

After fixing Part 1, the test runs but times out waiting for:
1. `has_speech_end` - A SpeechEnd VAD event (always false)
2. `has_injection` - Text in the mock injection sink (always false)

### Potential Causes

Based on code analysis, several potential issues were identified:

#### 1. Timing and Initialization
- The test was starting audio streaming immediately after runtime startup
- Pipeline components (chunker, VAD) might not be fully initialized
- **Fix Applied**: Added 200ms delay before streaming starts

#### 2. Playback Mode
- The test wasn't setting `COLDVOX_PLAYBACK_MODE` environment variable
- Default is `Realtime`, which makes the test take the full audio duration
- **Fix Applied**: Set accelerated playback at 4x speed

#### 3. Logging Gaps
- Insufficient logging to understand where the pipeline stalls
- **Fix Applied**: Added comprehensive logging:
  - WAV loader start/complete
  - VAD event collection
  - Iteration-by-iteration pipeline state
  - Better timeout error messages

### Pipeline Flow

The expected flow for the test:

```
WAV File → Ring Buffer → FrameReader → Chunker → SharedAudioFrame (broadcast)
                                                         ↓
                                                    VAD Processor
                                                         ↓
                                                    VadEvent (SpeechStart, SpeechEnd)
                                                         ↓
                                                    Session Events
                                                         ↓
                                                    STT Processor
                                                         ↓
                                                    TranscriptionEvent
                                                         ↓
                                                    Mock Injection Sink
```

### Diagnostic Questions

The enhanced logging should answer:

1. **Is audio being fed into the ring buffer?**
   - Look for: "WAV streaming task started"
   - Look for: "WAV streaming completed"

2. **Is the chunker processing frames?**
   - Look for: "Audio chunker started"
   - Look for: "Chunker: Frame sent to N receivers"

3. **Is the VAD receiving frames?**
   - Look for: "VAD processor task started"
   - Look for: "VAD: Received N frames, processing active"

4. **Is the VAD detecting speech?**
   - Look for: "VAD event collected: SpeechStart"
   - Look for: "VAD event collected: SpeechEnd"

5. **Is the STT processor receiving session events?**
   - Look for STT processor activity logs

6. **Are transcription events being generated?**
   - Look for mock sink injection logs

### Next Steps for Verification

1. **Run the test in an environment with ONNX Runtime available**
   ```bash
   cargo test --test golden_master test_short_phrase_pipeline --features whisper,text-injection -- --nocapture
   ```

2. **Examine the detailed logs** to identify where the pipeline stalls

3. **Common failure points to investigate:**
   - If no VAD events: Audio isn't reaching VAD (chunker/device config issue)
   - If SpeechStart but no SpeechEnd: VAD configuration or silence detection issue
   - If VAD events but no injection: STT or mock sink wiring issue

### Comparison with Working Tests

The unit tests in `runtime.rs` (e.g., `test_unified_stt_pipeline_vad_mode`) work by:
- **Manually injecting VAD events** via `app.raw_vad_tx.send()`
- NOT relying on automatic VAD detection from audio

The Golden Master test, in contrast, expects:
- **Automatic VAD detection** from the audio stream
- End-to-end black-box testing

This is a more realistic but also more complex test scenario.

### Test Audio File

- File: `crates/app/test_data/test_11.wav`
- Duration: 3.49 seconds
- Format: 16 kHz, mono, 16-bit PCM
- Peak amplitude: 37.1% of maximum (plenty for VAD detection)
- RMS: 1589 (good audio signal)

## Commits

- bc4e0ad: "fix: Remove #[cfg(test)] gates blocking integration tests"
- 0b6ff87: "debug: Add comprehensive logging to golden master test"

## Related Files

- `crates/app/src/runtime.rs` - Main runtime initialization
- `crates/app/tests/golden_master.rs` - Integration test
- `crates/app/src/audio/wav_file_loader.rs` - WAV streaming
- `crates/app/src/audio/vad_processor.rs` - VAD processing
- `crates/coldvox-audio/src/chunker.rs` - Audio chunking
