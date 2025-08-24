# Enhanced STT Project Phased Plan

## Overview
Building a robust, real-time audio processing pipeline for STT with focus on **reliability over performance**. This enhanced plan adds concrete error handling, recovery mechanisms, and detailed testing scenarios while maintaining the phased approach for incremental validation.

## Core Principles
- **Fail gracefully**: Every component must handle failures without crashing
- **Recover automatically**: Transient failures should self-heal
- **Test in isolation**: Each phase validates independently before integration
- **Simple concurrency**: Single-producer-single-consumer where possible
- **Defensive coding**: Assume external components will fail

## Audio Format Specification
**Standard Internal Format** (all components must convert to this):
- Sample Rate: 16000 Hz
- Bit Depth: 16-bit signed integer (i16)
- Channels: Mono (single channel)
- Endianness: Little-endian (native on x86_64)
- Frame Size: 320 samples (20ms at 16kHz)
- Buffer Format: Contiguous Vec<i16>

**Conversion Rules**:
- Stereo → Mono: Average L+R channels
- Higher sample rates: Resample with linear interpolation (quality not critical)
- 24/32-bit → 16-bit: Simple truncation (we're not audiophiles)
- Float → i16: Multiply by 32767 and clamp

## Thread Model & Data Flow
**Simple Producer-Consumer Pattern**:
```
[Mic Thread] → RingBuffer → [Processing Thread] → Chunks → [Output]
     ↓                              ↓
[Error Queue]                 [Error Queue]
```

- **Mic Thread**: Owns audio device, writes to ring buffer
- **Processing Thread**: Reads from buffer, runs VAD, produces chunks
- **Main Thread**: Coordinates, monitors health, handles errors
- **Communication**: Lock-free ring buffer + mpsc channels for errors

## Enhanced Config Schema
```rust
struct Config {
    // Audio
    window_ms: u32,              // Default: 500
    overlap_fraction: f32,       // Default: 0.5
    frame_ms: u32,              // Default: 20 (don't change unless you know why)
    
    // VAD
    speech_threshold: f32,       // Default: 0.6
    min_speech_ms: u32,         // Default: 200
    silence_debounce_ms: u32,   // Default: 300
    max_chunk_ms: u32,          // Default: 10000
    
    // Reliability
    mic_timeout_ms: u32,        // Default: 5000 (restart if no data)
    max_retries: u32,           // Default: 3
    retry_delay_ms: u32,        // Default: 1000
    buffer_overflow_policy: BufferPolicy, // Default: DropOldest
    
    // Testing
    save_audio: bool,           // Default: false (save raw audio for debugging)
    inject_noise: bool,         // Default: false (add white noise for testing)
    simulate_failures: bool,    // Default: false (random failures for resilience testing)
}

enum BufferPolicy {
    DropOldest,   // Overwrite old data (preferred)
    DropNewest,   // Reject new data
    Panic,        // Fail loudly (debug only)
}
```

---

## Phase 0: Foundation & Safety Net (NEW)
**Goal**: Set up error handling infrastructure and health monitoring before any audio processing.

**Deliverables**:
- Error types enum covering all failure modes
- Health monitor that tracks component status
- Graceful shutdown handler (Ctrl-C)
- Panic handler that logs before exit
- Simple state machine for app lifecycle

**Test (foundation_probe)**:
```bash
cargo run --bin foundation_probe -- --simulate-panics --simulate-errors
```
- Verify panic handler logs properly
- Test graceful shutdown on Ctrl-C
- Simulate various error types and verify recovery
- Check state transitions (Init → Running → Stopping → Stopped)

**Success Criteria**:
- Clean shutdown within 1 second
- All panics are caught and logged
- Error recovery attempts are logged

---

## Phase I: Microphone Capture with Recovery
**Goal**: Reliable mic capture that handles device disconnection/reconnection.

**Key Additions**:
- Device enumeration with fallback to default
- Automatic reconnection on failure
- Silence detection (all zeros = probable disconnect)
- Watchdog timer (restart if no frames for 5 seconds)

**Error Handling**:
```rust
enum MicError {
    DeviceNotFound,      // → Try default device
    DeviceDisconnected,  // → Wait and retry
    FormatUnsupported,   // → Fallback to closest supported
    BufferOverflow,      // → Log and continue
    UnknownError,        // → Restart capture
}
```

**Test Scenarios (mic_probe_enhanced)**:
1. **Normal Operation**: 2 minutes continuous capture
2. **Unplug Test**: Manually unplug/replug mic during capture
3. **Format Mismatch**: Request unsupported format, verify fallback
4. **Silence Detection**: Cover mic, verify detection of dead stream
5. **Multiple Devices**: Switch between mics if available

**Recovery Test Commands**:
```bash
# Test disconnect recovery
cargo run --bin mic_probe -- --duration 120 --expect-disconnect

# Test with specific device
cargo run --bin mic_probe -- --device "USB Microphone" --fallback-to-default

# Test silence detection
cargo run --bin mic_probe -- --silence-threshold 100 --silence-timeout 2000
```

---

## Phase II: Robust Ring Buffer
**Goal**: Lock-free ring buffer that handles over/underflow gracefully.

**Key Additions**:
- Overflow detection with configurable policy
- Underflow handling (return silence, don't block)
- Continuity counter to detect drops
- Stats tracking (drops, overflows, utilization)

**Implementation Details**:
```rust
struct RingBuffer {
    data: Vec<i16>,
    write_pos: AtomicUsize,
    read_pos: AtomicUsize,
    drops: AtomicU64,
    overflows: AtomicU64,
}

impl RingBuffer {
    fn write(&self, samples: &[i16]) -> Result<(), BufferError> {
        // Check space, apply overflow policy
        // Update continuity counter
    }
    
    fn read_window(&self, size: usize) -> AudioWindow {
        // Return window with metadata
        // Handle underflow by padding with zeros
    }
}
```

**Test Scenarios (buffer_probe_enhanced)**:
1. **Overflow Handling**: Write faster than read, verify policy
2. **Underflow Handling**: Read faster than write, verify silence padding
3. **Continuity Check**: Verify sequential sample counting
4. **Concurrent Access**: Stress test with rapid read/write
5. **Stats Accuracy**: Verify drop/overflow counts

---

## Phase III: VAD with Fallback
**Goal**: VAD integration with fallback to simple energy-based detection if ML model fails.

**Key Additions**:
- Fallback to energy-based VAD if model fails to load
- Smoothing filter for probability outputs (reduce flicker)
- Configurable pre-buffering (include audio before speech onset)
- VAD health check (detect stuck/frozen model)

**Fallback VAD**:
```rust
struct EnergyVAD {
    threshold: f32,
    window_energy: VecDeque<f32>,
}

impl EnergyVAD {
    fn detect(&mut self, samples: &[i16]) -> f32 {
        let energy = calculate_rms(samples);
        self.window_energy.push_back(energy);
        if self.window_energy.len() > 10 {
            self.window_energy.pop_front();
        }
        let smooth_energy = self.window_energy.iter().sum::<f32>() / self.window_energy.len() as f32;
        if smooth_energy > self.threshold { 1.0 } else { 0.0 }
    }
}
```

**Test Scenarios (vad_probe_enhanced)**:
1. **Model Failure**: Delete model file, verify fallback activates
2. **Probability Smoothing**: Rapid speech/silence, verify debouncing
3. **Pre-buffering**: Verify audio before speech onset is captured
4. **Energy VAD**: Test fallback with various noise levels
5. **Health Check**: Simulate frozen VAD, verify detection

---

## Phase IV: Intelligent Chunking
**Goal**: Smart chunk assembly with overlap handling and metadata.

**Key Additions**:
- Overlap windows (50-100ms) with crossfade
- Chunk metadata (start/end timestamps, confidence, duration)
- Handling rapid on/off speech (minimum gap between chunks)
- Force-flush on max duration
- Chunk validation (minimum useful length)

**Chunk Metadata**:
```rust
struct Chunk {
    audio: Vec<i16>,
    start_time: SystemTime,
    end_time: SystemTime,
    duration_ms: u32,
    confidence: f32,        // Average VAD confidence
    forced_end: bool,       // Hit max duration?
    gap_before_ms: Option<u32>, // Time since last chunk
}
```

**Test Scenarios (chunker_probe_enhanced)**:
1. **Natural Speech**: Read a paragraph, verify sentence boundaries
2. **Rapid Toggle**: Fast talking with brief pauses
3. **Long Utterance**: Talk continuously for >10 seconds
4. **Background Speech**: TV/radio in background
5. **Cough/Laugh Test**: Non-speech sounds between words

**Test Commands**:
```bash
# Test with specific audio patterns
cargo run --bin chunker_probe -- --test-pattern natural-speech
cargo run --bin chunker_probe -- --test-pattern rapid-toggle
cargo run --bin chunker_probe -- --min-gap-ms 100 --max-chunk-ms 5000
```

---

## Phase V: Stress Testing & Edge Cases
**Goal**: Verify system stability under adverse conditions.

**Test Scenarios**:
1. **CPU Stress**: Run with CPU at 100% (stress tool)
2. **Memory Pressure**: Run with limited memory
3. **Rapid Config Changes**: Modify thresholds during operation
4. **Disk Full**: Verify graceful handling when logging fails
5. **Clock Jumps**: System time changes (NTP sync)
6. **Permission Errors**: Revoke mic permissions mid-run
7. **Zombie Threads**: Simulate hung processing thread

**Chaos Testing Script**:
```bash
#!/bin/bash
# chaos_test.sh

# Start the app
cargo run --release &
APP_PID=$!

sleep 5

# CPU stress
stress --cpu 8 --timeout 30 &

# Memory stress  
stress --vm 2 --vm-bytes 1G --timeout 30 &

# Kill random threads
for i in {1..5}; do
    THREAD=$(ps -T -p $APP_PID | tail -n +2 | shuf -n 1 | awk '{print $2}')
    kill -STOP $THREAD
    sleep 2
    kill -CONT $THREAD
done

# Change system time
sudo date -s "2 hours"
sleep 5
sudo ntpdate pool.ntp.org

wait $APP_PID
```

---

## Phase VI: Integration & Polish
**Goal**: Full pipeline with monitoring and debugging tools.

**Additions**:
- Web UI for real-time monitoring (optional)
- Audio recording for debugging
- Config hot-reload
- Metrics export (Prometheus format)
- Debug commands via stdin

**Debug Interface**:
```
Commands:
  status    - Show component health
  stats     - Show performance stats
  toggle X  - Toggle component X on/off
  reload    - Reload config file
  save      - Save last 30s of audio
  quit      - Graceful shutdown
```

---

## Testing Philosophy

### For Each Phase
1. **Happy Path Test**: Everything works as expected
2. **Failure Test**: Primary component fails
3. **Recovery Test**: Component recovers from failure
4. **Stress Test**: High load/adverse conditions
5. **Edge Case Test**: Boundary conditions

### Test Output Format
```
[2024-01-01 12:00:00] TEST: Starting mic_probe
[2024-01-01 12:00:01] INFO: Mic opened: USB Microphone
[2024-01-01 12:00:02] WARN: Silence detected for 2000ms
[2024-01-01 12:00:03] ERROR: Device disconnected
[2024-01-01 12:00:04] INFO: Attempting reconnection (1/3)
[2024-01-01 12:00:05] SUCCESS: Reconnected to: USB Microphone
[2024-01-01 12:00:10] STATS: Frames: 500, Drops: 2, Uptime: 10s
[2024-01-01 12:00:10] TEST: Complete - PASS
```

---

## What "Benchmarking Framework" Meant (You Can Ignore This)
Since you don't care about performance, skip this. But FYI, I meant establishing baseline metrics (latency, CPU usage) to detect if future changes make things significantly worse. Not about optimization, just avoiding accidental performance disasters.

---

## Timeline (Realistic)
- Phase 0: 2-3 days (error handling foundation)
- Phase I: 2-3 days (robust mic capture)
- Phase II: 1-2 days (ring buffer)
- Phase III: 3-4 days (VAD with fallback)
- Phase IV: 2-3 days (chunking logic)
- Phase V: 2-3 days (stress testing)
- Phase VI: 2-3 days (integration)

**Total**: 3-4 weeks for a robust "it just works" system

---

## Success Metrics
- Runs for 24 hours without crashing
- Handles mic disconnect/reconnect gracefully
- Recovers from all transient failures
- Produces usable chunks for STT
- No memory leaks
- Clean shutdown always

This plan prioritizes reliability and debuggability over performance. Each phase can be tested in isolation with clear pass/fail criteria.