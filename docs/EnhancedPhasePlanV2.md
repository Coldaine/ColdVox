Model: GitHub Copilot
Timestamp: 2025-08-24

# Enhanced STT Project Plan V2

This V2 folds in resilience upgrades decided after the critique: monotonic timing, safer resampling, explicit overflow policies with metrics, stronger VAD fallback, equal‑power chunking, structured logging, and controlled recovery backoff.

## Notable changes from V1
- Use monotonic time (Instant) for all durations/intervals; convert to SystemTime only for logging/metadata.
- Fractional-phase linear resampler to avoid drift; robust sample format conversion with clamping and NaN/Inf handling.
- Silence/disconnect detection via energy over N frames with debounce; “all‑zero” treated as a hint, not sole signal.
- RingBuffer: power‑of‑two capacity, index masking, explicit atomic orderings (AcqRel), stats and overflow policy surfaced via metrics.
- Backoff: capped exponential with jitter for mic/VAD recovery loops.
- VAD fallback: short noise-floor calibration; stuck‑model detection and auto fallback.
- Chunking: equal‑power crossfade; pre‑roll/post‑roll; min‑gap/min‑chunk enforcement; defined confidence calculation.
- Observability: structured, rate‑limited logs and a concrete Prometheus metrics set.

---

## Core principles (unchanged)
- Fail gracefully; recover automatically; test each phase in isolation; simple single‑producer/single‑consumer; defensive coding.

## Timekeeping
- Use std::time::Instant for all timers, watchdogs, and durations.
- Maintain a TimeSource trait for tests; convert to SystemTime only when emitting user‑facing timestamps.

## Audio format specification
- Internal: 16kHz, 16‑bit signed (i16), mono, little‑endian, 320‑sample frames (20ms), contiguous Vec<i16>.
- Conversion rules:
  - Stereo → mono: average L+R.
  - Higher rates → 16kHz: fractional‑phase incremental linear resampler; carry fractional step across calls to avoid drift.
  - 24/32‑bit → 16‑bit: clamp to i16 range.
  - Float → i16: multiply by 32767, clamp; treat NaN/Inf as 0.

## Thread model and data flow
[Mic Thread] → RingBuffer → [Processing Thread] → Chunks → [Output]
  ↓                              ↓
[Error Queue]               [Error Queue]

- Mic thread owns device; processing thread runs VAD and chunking; main thread orchestrates and monitors.
- Communication: lock‑free ring buffer for audio; mpsc channels for errors and control; a shared Shutdown token.

## Config schema (normalized on load)
```rust
struct Config {
    // Audio windows
    window_ms: u32,               // default 500
    overlap_fraction: f32,        // default 0.5
    frame_ms: u32,                // default 20 (320 samples)

    // VAD
    speech_threshold: f32,        // default 0.6 (smoothed)
    min_speech_ms: u32,           // default 200
    silence_debounce_ms: u32,     // default 300
    max_chunk_ms: u32,            // default 10000
    pre_roll_ms: u32,             // default 200
    post_roll_ms: u32,            // default 120
    min_gap_ms: u32,              // default 120
    min_chunk_ms: u32,            // default 250

    // Reliability
    mic_timeout_ms: u32,          // default 5000
    max_retries: u32,             // default 3
    retry_backoff_initial_ms: u32,// default 250
    retry_backoff_max_ms: u32,    // default 3000
    retry_jitter_fraction: f32,   // default 0.2
    buffer_overflow_policy: BufferPolicy, // default DropOldest

    // Observability
    log_level: String,            // default "info"
    log_json: bool,               // default true
    log_rate_limit_per_sec: u32,  // default 10
    metrics_enabled: bool,        // default true
    metrics_port: u16,            // default 9900

    // Testing and fixtures
    save_audio: bool,             // default false
    inject_noise: bool,           // default false
    simulate_failures: bool,      // default false

    // Platform
    prefer_backend: Option<String>, // e.g., Some("alsa"), default None
}

enum BufferPolicy { DropOldest, DropNewest, Panic }
```

Normalization on load: precompute all sample counts (e.g., frame_samples, pre_roll_samples) to avoid repeated conversions.

## Ring buffer (Phase II)
- Power‑of‑two capacity; index masking to avoid modulo cost.
- Atomic ordering: producers use fetch_add with Release; consumers load with Acquire; use a continuity counter.
- Underflow → zero‑padding; overflow policy applied per config.
- Stats: drops_total, overflows_total, utilization (current fill/capacity), continuity_gaps.

## VAD with fallback (Phase III)
- Primary: model‑based VAD (e.g., Silero). Fallback: energy‑based VAD with a 1–2s baseline noise calibration on startup.
- Smoothing: EMA over last N windows to reduce flicker.
- Stuck detection: if variance in VAD output < epsilon for > X seconds, reload model or switch to fallback.

## Chunking (Phase IV)
- Equal‑power crossfade over overlap region.
- Pre‑roll and post‑roll included to preserve phonemes.
- Enforce min_gap_ms and min_chunk_ms; force‑flush at max_chunk_ms.
- Confidence = average of smoothed VAD within chunk; record forced_end and gap_before_ms.

## Observability
- Structured JSON logs with rate limiting and rotation.
- Prometheus metrics (examples):
  - audio_drops_total, audio_overflows_total, ringbuffer_utilization,
  - vad_confidence_avg, vad_fallback_active,
  - chunks_total, chunk_duration_ms_bucket,
  - mic_reconnects_total, watchdog_resets_total, uptime_seconds.

## Recovery and shutdown
- Backoff: capped exponential with jitter; track consecutive failures and apply cooldown.
- Shutdown: shared token; bounded joins with timeout; log forced aborts.

## ONNX runtime probing (Phase III)
- Deterministic search order for native libs; explicit CLI override; log resolved runtime path/version at startup.

---

## Phases and deliverables

### Phase 0: Foundation & Safety Net
Deliverables:
- Error enums; health monitor; Ctrl‑C graceful shutdown; panic hook that logs; Instant‑based timers; structured logging; config via clap+serde with precedence CLI > env > file; backoff utility; basic metrics facade.
Tests: foundation_probe with simulated errors/panics; verify clean shutdown ≤ 1s and recovery attempts logged.

### Phase I: Microphone capture with recovery
Deliverables:
- Device enumeration with default fallback; robust format negotiation; fractional‑phase resampler; silence/dead‑stream detection using energy; watchdog timer; reconnection loop with backoff.
Tests: normal capture, unplug/replug, format mismatch fallback, silence detection, multi‑device switching.

### Phase II: Robust Ring Buffer
Deliverables:
- Lock‑free buffer with policies and stats; continuity counters; underflow zero‑padding; property tests for concurrency/wrap‑around.
Tests: overflow/underflow handling; concurrent stress; stats accuracy.

### Phase III: VAD with fallback
Deliverables:
- Model load with health checks; fallback EnergyVAD with baseline calibration; smoothing; stuck detection and auto fallback; ONNX runtime probe.
Tests: model missing → fallback; debouncing and smoothing; pre‑buffering; noise robustness; stuck detection.

### Phase IV: Intelligent chunking
Deliverables:
- Overlap with equal‑power crossfade; pre/post‑roll; metadata and confidence; min‑gap/min‑chunk; force‑flush.
Tests: natural speech, rapid toggles, long utterances, background speech, non‑speech transients.

### Phase V: Stress testing & edge cases
Deliverables:
- CPU/memory pressure; rapid config changes; disk‑full logging behavior; time jumps; permission errors; simulated hung thread; acceptance thresholds.
Tests: chaos script; 24h endurance with stable memory/thread counts and bounded drops/overflows.

### Phase VI: Integration & polish
Deliverables:
- Metrics export (HTTP Prometheus); optional web UI; audio recording for debugging; config hot‑reload; stdin debug commands.
Tests: status/stats/toggle/reload/save/quit flows.

---

## Acceptance criteria (refined)
- 24h run: memory/thread counts stable (±1%); no crashes; clean shutdown.
- Mic disconnect → reconnect within ≤ 3 retries and ≤ 10s wall time.
- VAD fallback within < 200ms after failure; single ERROR with subsequent INFO heartbeats.
- Chunk quality: ≥95% boundaries within ±50ms of golden references on fixtures.

## CI and testability
- Synthetic audio fixtures and goldens for chunk boundaries and confidence.
- Property tests for ring buffer; seeded fault injection for mic read, VAD predict, chunk enqueue.
- Short smoke runs of foundation/buffer/vad probes on PRs; publish metrics snapshots as artifacts.

## Next steps
1) Scaffold Phase 0 crates/modules: config (clap+serde), logging, time source, shutdown token, backoff, metrics facade.
2) Add synthetic fixtures and a minimal ring buffer test harness.
3) Integrate mic capture with resampler and watchdogs.
