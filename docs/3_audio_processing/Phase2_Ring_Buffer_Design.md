# Phase 2 Design — Buffering Only (Lock-free Ring Buffer semantics)

**Last Updated:** 2025-08-24 (Post live audio testing)
**Status:** ✅ COMPLETED - Implemented using rtrb library

## Scope (and Non‑Goals)
- In scope: buffering behavior between the mic callback and processing; overflow/underflow policies; continuity tracking; telemetry; configuration.
- Not in scope: VAD integration, window slicing policy, or chunking/utterance assembly. Those arrive in later phases.

## Current State (baseline)
- Producer thread: CPAL callback creates `AudioFrame { samples: Vec<i16>, timestamp, sample_rate, channels }`.
- Buffer: crossbeam bounded channel with capacity 100 frames.
  - ~20 ms per frame → ~2 s of frame buffering.
  - Lock‑free, MPSC; try_send on callback thread.
  - Overflow policy in code: DropNewest (incoming frames are dropped on full queue) with stats increment and warning log.
  - **Live test results (2025-08-24):** 
    - 0 drops under normal load (10 seconds, 233 frames captured)
    - 159,033 samples processed without loss
    - Peak volume 16.2%, proving full audio pipeline functionality
    - Capacity sufficient for real-world conditions
- Stats: frames captured / frames dropped; active/silent classification; watchdog monitoring.

This already satisfies "lock‑free ring buffer" goals for the cross‑thread boundary using a proven primitive.

## Gaps to close in Phase 2
1. Make buffer capacity and policy explicit and configurable.
2. Track continuity (monotonic frame sequence) to positively detect silent drops.
3. Define underflow semantics for consumers that expect fixed‑size pulls (pad with silence rather than block or error).
4. Add buffer utilization telemetry to catch early backpressure.

No change to fundamental threading or to CPAL callback behavior; no additional consumers introduced.

## Requirements (from EnhancedPlan Phase II, mapped to repo reality)
- Overflow detection: log + counter (already present). Expose policy in config.
- Overflow policy options:
  - DropNewest (current, default)
  - DropOldest (optional; see Implementation Options)
  - Panic (debug only)
- Underflow handling: when a consumer pulls fixed frames/windows and the queue is temporarily empty, return zero‑padded data (non‑blocking) if the call site requests non‑blocking semantics.
- Continuity counter: add `seq_no: u64` incremented by the producer for each emitted frame.
- Telemetry: gauges for queue length (approximate), last frame age, and drop rate.

## Design decisions

### Option A: Keep Crossbeam (Current Implementation)
- **Status:** Working with 0 drops in live testing
- **Pros:** Battle-tested, simple, already implemented
- **Cons:** Per-frame allocations, message passing overhead

### Option B: Replace with True Ring Buffer (Recommended for Phase 2)
- **Replace crossbeam channel with a lock-free ring buffer** for direct sample storage:
  ```rust
  struct RingBuffer {
      samples: Box<[i16; BUFFER_SIZE]>,  // Pre-allocated continuous memory
      write_pos: AtomicUsize,
      read_pos: AtomicUsize,
  }
  ```
- **Benefits:**
  - Zero allocations in audio callback (critical for real-time)
  - Lower latency (no message passing)
  - Better cache locality
  - Direct memory writes from callback
- **Trade-offs:**
  - More complex implementation
  - Need to track metadata (timestamps) separately
  - Requires careful lock-free programming

### Final Implementation Decision
**Phase 2 has been completed using the rtrb (Real-Time Ring Buffer) library**, which provides:
- Zero allocations in audio callback
- Lock-free SPSC ring buffer optimized for audio
- Producer/consumer split for thread safety
- Real-time guarantees for audio processing

Implementation can be found in `crates/app/src/audio/ring_buffer.rs`

## Public contracts (narrow)
- Producer (unchanged except for sequence):
  - Inputs: `Vec<i16>` samples per frame (preferred 16 kHz mono ~20 ms).
  - Outputs: `AudioFrame { seq_no, samples, timestamp, sample_rate, channels }` via crossbeam::Sender.
  - Error modes: on overflow, apply configured policy; never block callback.
- Consumer adapter (optional helper for downstream readers):
  - `pull_fixed(count: usize, non_blocking: bool) -> Vec<i16>`
    - On underflow and `non_blocking=true`, returns available data padded with zeros.
    - On `non_blocking=false`, blocks on the channel recv to accumulate exact `count` samples.
  - Exposes last observed `seq_no` and counts gaps.

## Implementation approach

### Phase 2.1: Enhance current crossbeam implementation
- Add `seq_no` to `AudioFrame` for continuity tracking
- Make capacity configurable via `AudioConfig`
- Add metrics for buffer utilization
- Implement consumer adapter for fixed-size reads

### Phase 2.2: Implement true ring buffer
```rust
// Core ring buffer structure
struct AudioRingBuffer {
    buffer: Box<[i16]>,           // Pre-allocated sample storage
    capacity: usize,               // Total samples (power of 2)
    write_pos: AtomicUsize,        // Writer position
    read_pos: AtomicUsize,         // Reader position
    overflow_policy: OverflowPolicy,
}

// Direct write from audio callback
impl AudioRingBuffer {
    fn write_samples(&self, samples: &[i16]) -> Result<(), BufferFull> {
        // Lock-free write with configurable overflow handling
    }
    
    fn read_samples(&self, count: usize) -> Vec<i16> {
        // Lock-free read with underflow padding
    }
}
```

The ring buffer replaces the crossbeam channel entirely, with audio callback writing directly to pre-allocated memory.

## Telemetry additions
- Counters: frames_captured, frames_dropped, continuity_gaps.
- Gauges: queue_len (approximate), last_frame_age_ms, drop_rate_1m.
- Logs: single warn per burst (rate‑limited) “buffer full, dropping N frames”.

## Config additions (Phase 2)
- `buffer_capacity_frames` (default: 100)
- `buffer_overflow_policy` (enum: DropNewest [default], DropOldest, Panic)
- `consumer_underflow_pad` (bool, default: true)

## Tests (Phase 2 only)

### Live Hardware Test (Primary)
- **10-second audio capture to WAV** with real-time volume analysis
- Validates: buffer performance, zero drops, format conversion, end-to-end pipeline
- Already proven: 0 drops with 159,033 samples processed

### Ring Buffer Specific Tests
- **Wrap-around test:** Fill buffer to capacity, verify circular overwrite behavior
- **Concurrent access test:** Simultaneous read/write at high frequency
- **Overflow policies:** DropOldest (overwrite), DropNewest (reject), Panic (debug)
- **Underflow behavior:** Read when empty returns zeros or blocks based on mode
- **Power-of-2 masking:** Verify index calculations with buffer sizes 2^n

## Success criteria ✅ ACHIEVED
- ✅ No changes to app threading; callback remains non‑blocking.
- ✅ Ring buffer provides real-time guarantees
- ✅ Producer/consumer split allows separate thread ownership
- ✅ Overflow handling with configurable policies
- ✅ Unit tests validate basic functionality
- 📋 **Pending:** Integration with telemetry and metrics (Phase 3 work)

## Migration & rollout
- Add `seq_no` to `AudioFrame` struct and initialize in producer.
- Plumb config for capacity/policy; retain default behavior.
- Add consumer adapter utility (internal module) and unit tests.
- Expose new metrics in `BasicMetrics`.

## Out of scope (future phases)
- Fixed 512‑sample windowing for VAD (Phase 3 will consume the buffer via the adapter).
- Chunking/utterance assembly and smoothing.

---

Notes:
- The existing 2‑second buffer proved sufficient in testing; Phase 2 focuses on visibility and explicit semantics rather than rewriting a proven pipe.
- If future workloads show frequent DropNewest incidents, revisit ArrayQueue (DropOldest) or increase capacity based on measured occupancy percentiles.
