---
doc_type: architecture
subsystem: foundation
version: 1.0.0
status: draft
owners: Documentation Working Group
last_reviewed: 2025-10-19
redirect: fdn-voice-pipeline-core-design.md
---

# Moved: Voice Pipeline Core - Design

Renamed with domain short code prefix. New location:

- `docs/domains/foundation/fdn-voice-pipeline-core-design.md`

Please update any bookmarks or links.

## Components

1. Audio Capture & Device Management
2. Voice Activity Detection (VAD)
3. Speech-to-Text (STT)
4. Text Injection

## Key Design Decisions

### 1. Audio Capture Architecture

**Approach**: Multi-stage pipeline with device-native capture and downstream resampling.

**Code paths**:
- `AudioCaptureThread` runs on a dedicated OS thread to maintain CPAL stream stability
- Captures audio in device's native format (any sample rate, channels, bit depth)
- Converts all formats to 16-bit signed integers immediately in the capture callback
- `FrameReader` reads from lock-free SPSC ring buffer (`AudioRingBuffer`)
- `AudioChunker` resamples to 16 kHz mono using high-quality Rubato resampler
- Produces fixed 512-sample frames (32ms at 16 kHz) for downstream processing

**Why this way**:
- Device compatibility: Don't force specific sample rates
- Audio quality: High-quality resampling off audio thread
- Simplicity: Fixed 512-sample frames for VAD/STT
- Performance: Lock-free ring buffer avoids blocking

**Trade-offs**: +32ms latency, resampler memory overhead

### 2. Voice Activity Detection

**Approach**: Silero V5 ONNX-based ML model only (no fallback).

**Code paths**:
- `SileroEngine` wraps external `voice_activity_detector` crate
- Processes 512-sample windows at 16 kHz
- Emits `VadEvent::{SpeechStart, SpeechEnd}` with timestamps
- Debouncing logic with configurable thresholds:
  - Speech threshold: 0.1 (probability)
  - Min speech duration: 100ms
  - Min silence duration: 500ms

**Why this way**:
- ML detection >> energy-based methods
- 500ms silence stitches natural pauses (issue #61)
- Single implementation = simpler code
- Silero reliable across diverse conditions

**Trade-offs**: Higher CPU than energy VAD, +500ms end-of-utterance latency

**No fallback VAD**: Previous Level3 energy VAD removed due to poor accuracy. If Silero fails, issue is usually in audio capture, not VAD.

### 3. Speech-to-Text Processing

**Approach**: Plugin architecture, Vosk primary offline engine.

**Code paths**:
- `SttPluginManager` handles plugin lifecycle, failover, and garbage collection
- `PluginSttProcessor` manages utterance state and audio buffering
- Supports two activation modes:
  - **VAD mode**: Automatic activation on speech detection
  - **Hotkey mode**: Manual push-to-talk activation (default)
- Two processing strategies:
  - **Incremental**: Streams audio to STT as it arrives (lower latency)
  - **Batch**: Buffers complete utterance before processing (better accuracy)
- Plugin failover after consecutive errors (configurable threshold)
- Automatic model unloading after idle period (garbage collection)

**Why this way**:
- Plugins: Future engines (Whisper, cloud)
- Failover: Production reliability
- GC: Prevent model memory bloat
- Hotkey default: Reduce false activations

**Trade-offs**: Plugin abstraction complexity, GC reload latency

### 4. Text Injection

**Approach**: Platform-aware backends with runtime fallback.

**Code paths**:
- Build-time platform detection (`crates/app/build.rs`)
- Linux: AT-SPI, wl-clipboard, ydotool (Wayland), kdotool (X11)
- Windows/macOS: Enigo library
- Runtime availability testing for each backend
- Adaptive strategy based on app-specific success rates
- Automatic clipboard restoration after paste operations

**Why this way**:
- No universal injection method exists
- Build-time platform detection = less runtime overhead
- Success tracking improves over time
- Clipboard restore prevents data loss

**Trade-offs**: Platform-specific complexity, external tool deps

**No runtime enable/disable**: Hardcoded enabled when compiled with feature. Runtime toggle needs state management; compile-time choice sufficient for now.

## Data Flow

```
Audio Device (native format)
    ↓ (CPAL callback, dedicated thread)
AudioCapture (convert to i16)
    ↓ (SPSC ring buffer)
FrameReader (device-native i16 frames)
    ↓
AudioChunker (resample to 16kHz mono, 512-sample frames)
    ↓ (broadcast channel)
    ├─→ VadProcessor (Silero)
    │       ↓ (VAD events)
    │   SessionManager
    │       ↓ (Session events)
    └─→ SttProcessor
            ↓ (Transcription events)
        TextInjection
            ↓
        Target Application
```

## Performance

**Latency** (typical):
- Capture → ring buffer: ~10ms
- Resampling: ~32ms
- VAD: ~5ms/frame
- STT: 50-200ms
- Injection: 50-250ms
- **Total**: 150-500ms

**Memory**:
- Audio buffers: ~256KB
- Vosk model: 40-100MB
- Silero VAD: ~8MB
- **Baseline**: 60-120MB

**CPU**:
- Idle: <5%
- Active speech: 30-75% (model dependent)

## Error Handling

**Audio**: 5s watchdog → auto-restart with device fallback

**VAD**: Errors logged, malformed events discarded, state reset on consecutive errors

**STT**: Failover after 5 errors (default), 10s cooldown before retry

**Injection**: Per-backend success tracking, automatic fallback, fail-fast mode for testing

## Configuration

Priority: CLI args > env vars (`COLDVOX_*`) > `config/default.toml`

## Testing

- **Unit**: Component isolation with mocks
- **Integration**: Full pipeline with WAV files
- **E2E**: Real audio verification, mode switching, device fallback

## Future Direction

See `docs/architecture.md`: always-on listening, tiered STT, predictive loading

## Key Files

- Requirements: `requirements.md`
- Pipeline construction: `crates/app/src/runtime.rs`
- Audio: `crates/coldvox-audio/src/capture.rs`, `chunker.rs`
- VAD: `crates/coldvox-vad-silero/src/silero_wrapper.rs`
- STT: `crates/app/src/stt/plugin_manager.rs`, `processor.rs`
- Injection: `crates/coldvox-text-injection/src/manager.rs`
