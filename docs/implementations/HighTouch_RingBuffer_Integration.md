# High‑Touch Ring Buffer Integration Plan

Last updated: 2025-08-24
Status: Proposed (Phase 2+)
Owners: App/audio

## Overview

This document proposes a higher‑touch integration of a true sample‑level, lock‑free ring buffer (rtrb) in the audio capture path. It replaces the current per‑frame crossbeam queue with a preallocated ring plus a reader adapter. The goal is to reduce real‑time (RT) callback work and allocations, improve latency/jitter, enable continuity/underflow semantics, and prepare for VAD windowing.

## Why not just swap the queue?

A minimal swap (crossbeam → rtrb over `AudioFrame`) is low risk and quick but keeps:
- Per-callback Vec allocation and copying
- Wall‑clock–based timestamps and no continuity tracking
- No underflow policy (pad/block/partial) for fixed-size consumers
- Limited telemetry (no buffer utilization)

Those gaps matter as we integrate VAD (fixed 512‑sample windows), target stable latency on busy systems, and want insights into backpressure.

## Goals

- Zero allocations in the CPAL callback; strictly non‑blocking
- Deterministic sample pacing and continuity detection (seq_no, gap counts)
- Configurable underflow/overflow behavior
- Useful buffer telemetry (utilization, drops, last frame age)
- Reader API that yields fixed‑size frames/windows for downstream processing (VAD‑ready)

## Non‑Goals

- Changing device negotiation, watchdog, or error recovery fundamentals
- Implementing VAD itself (done later); this only improves the buffering layer

## Design summary

- Producer (RT thread):
  - Mix/convert to mono as we do today
  - Write samples directly into an rtrb SPSC ring (`i16`), using `write_chunk` to avoid extra copies
  - No `Vec` allocation, no message passing
  - On overflow, apply configured policy (default DropNewest) and increment counters

- Consumer (non‑RT thread) via a `FrameReader` adapter:
  - Pull exactly N samples per call (e.g., 320 for 20 ms @ 16 kHz, or 512 for VAD)
  - Underflow modes:
    - PadWithSilence (default): return N samples, padding with zeros
    - NonBlockingPartial: return what’s available (len ≤ N)
    - BlockUntilFull: spin/await until N available (non‑RT only)
  - Track `seq_no` and continuity gaps; timestamps advance by sample clock, not wall‑clock bursts
  - Run silence detection off the callback thread

- Telemetry:
  - Counters: samples_written, samples_dropped, frames_emitted, underflow_events, continuity_gaps
  - Gauges: buffer_utilization (approx used/total), last_frame_age_ms, drop_rate

## APIs and contracts

- New internal types in `audio`:
  - `AudioRingBuffer` (already present for samples) with producer/consumer halves
  - `FrameReader` wrapping the consumer:
    - `next_frame(frame_len: usize, mode: UnderflowMode) -> AudioFrame`
    - Emits `AudioFrame { seq_no, samples, timestamp, sample_rate, channels }`

- Config additions (`AudioConfig`):
  - `buffer_capacity_seconds: f32` (default: 2.0)
  - `buffer_overflow_policy: OverflowPolicy` = { DropNewest (default), Panic (debug) }
  - `consumer_underflow_mode: UnderflowMode` = { PadWithSilence (default), NonBlockingPartial, BlockUntilFull }

- Behavior:
  - Producer never blocks; all policies must be O(1) in the callback
  - Reader controls underflow surfaces to consumers, keeping call sites simple

## Timestamping and continuity

- Maintain `total_samples_emitted` and a `last_frame_end_time: Instant`
- For each emitted frame of length N at sample rate R:
  - `frame_start = last_frame_end_time`
  - `frame_end = frame_start + N/R`
  - Update `last_frame_end_time = frame_end`
- This decouples timestamping from callback burstiness and remains monotonic
- `seq_no` increments per emitted frame; detect gaps via drop/underflow accounting

## Overflow and underflow policies

- Overflow (producer):
  - DropNewest (default): skip writing new samples, increment counters, rate‑limit warn
  - Panic (debug builds only): immediate fail for diagnostics

- Underflow (consumer):
  - PadWithSilence: produce exactly N samples with zero padding; increments underflow counter
  - NonBlockingPartial: return available samples (≤ N) and let caller decide
  - BlockUntilFull: loop (non‑RT) until N are available, with brief sleeps/yields

## Telemetry additions

- New counters and gauges surfaced via `telemetry/BasicMetrics` and `AudioCapture::get_stats()` mapping
- Buffer utilization estimation: `used ≈ capacity - producer.slots()`
- Expose last frame age and drop rate for operational insight

## Integration details (by file)

- `crates/app/src/audio/capture.rs`:
  - Replace crossbeam `Sender/Receiver<AudioFrame>` with `AudioRingBuffer` producer/consumer (sample‑level)
  - Add `create_reader(frame_len_samples: usize) -> FrameReader`
  - Move silence detection from the RT callback into the reader
  - Map stats: frames_captured → frames_emitted; frames_dropped → samples_dropped/frame_len

- `crates/app/src/audio/ring_buffer.rs`:
  - Keep the SPSC ring; ensure wrap‑around writes/reads are correct (fixed)
  - Optionally add helpers to query capacity/used

- `crates/app/src/bin/mic_probe.rs`:
  - Use `create_reader(320)` (20 ms @ 16 kHz) and a loop calling `next_frame`
  - Remove channel `recv()` usage once migrated

- `crates/app/src/foundation/error.rs`:
  - Extend `AudioConfig` with capacity and policy enums

- `telemetry/*`:
  - Wire new counters/gauges (optional in first pass)

## Rollout strategy

- Phase A (compatibility pump, optional):
  - Internally read from the ring and forward `AudioFrame` over a channel so `get_receiver()` users remain unchanged
  - Validate stability and telemetry in real runs

- Phase B (native reader):
  - Migrate `mic_probe` and other consumers to `FrameReader`
  - Remove the compatibility pump and channel

## Testing plan

- Unit tests:
  - Ring wrap‑around under concurrent writer/reader
  - Underflow padding and partial reads
  - Overflow policy behavior and log rate‑limiting
  - Timestamp monotonicity under varying pull sizes
  - Continuity gap detection when inducing drops

- Integration (simulated):
  - Simulate callback write bursts; verify reader produces steady frames with low jitter

- Live hardware test:
  - Reuse `mic_probe` recording; confirm zero allocations in callback (via inspection/bench), low drop rate, and stable volume display

## Risks and mitigations

- Increased complexity: keep APIs narrow; comprehensive unit tests
- Sample‑rate renegotiation: recreate ring sized to the new rate on recover; reset reader state
- Ownership discipline (SPSC): maintain single producer/consumer invariants; document clearly

## Effort estimate

- Phase A (with compatibility pump): ~0.5 day
- Phase B (native reader, telemetry, tests): ~0.5–1 day

## Acceptance criteria

- Callback path has zero dynamic allocations under normal operation
- Underflow/overflow policies enforced and observable in metrics
- Reader emits fixed‑size frames with monotonic timestamps and seq_no
- Existing probes work and report meaningful stats

## Alternatives considered

- Minimal swap to rtrb over `AudioFrame`: fastest, but retains allocations and loses most benefits
- Custom lock‑free buffer: unnecessary; rtrb already provides the needed semantics and performance

---

References:
- Code: `audio/capture.rs`, `audio/ring_buffer.rs`, `bin/mic_probe.rs`, `foundation/error.rs`
- VAD context: `Forks/ColdVox-voice_activity_detector` (expects 512‑sample windows @ 16 kHz)
