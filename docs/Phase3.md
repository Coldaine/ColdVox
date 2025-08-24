## Phase 3 — Voice Activity Detection (VAD) with fallback and gating

Status: Planned (Phase 2 complete with rtrb ring buffer)

### Scope
- Integrate ML VAD (Silero V5 via vendored ONNX crate) on 16 kHz mono audio.
- Emit robust SpeechStart/SpeechEnd events with debounce and pre/post‑roll.
- Provide an energy‑based fallback VAD that auto‑activates on ML VAD failure.
- Add metrics and health checks for observability and recovery.

Non‑Goals (Phase 3):
- Full “chunker” and STT handoff semantics (Phase 4). A minimal feed to STT may be prototyped but isn’t required.
- Multi‑stream fan‑out or advanced diarization.

---

## Dependencies
- Phase 2 ring buffer: `crates/app/src/audio/ring_buffer.rs` (rtrb‑based, SPSC, zero‑alloc on callback path).
- Audio format: 16 kHz, mono, S16LE; 20 ms nominal capture frames.
- Vendored VAD crate: `Forks/ColdVox-voice_activity_detector` (Silero V5 ONNX runtime, 512‑sample windows @16 kHz).
- Optional fallback: Energy VAD spec in `docs/Reference/EnergyBasedVAD.md`.

## Architecture
Callback (CPAL) → Ring buffer (rtrb) → VAD Processor (consumer task)
→ State machine (debounce, hysteresis, pre/post‑roll)
→ Events: SpeechStart / SpeechEnd (+ active boolean per frame)
→ (optional) Minimal utterance feed to STT (prototype only)

Notes:
- Use the VAD crate’s iterator/stream helpers to generate the required 512‑sample windows for Silero V5. Avoid hand‑rolled hop logic.
- Keep a small internal frame queue to implement pre/post‑roll around boundaries; source frames from the ring buffer (do not block the callback).

## Contracts

Inputs:
- PCM i16, 16 kHz, mono frames drained from the ring buffer at the consumer.

Outputs (events):
- `VadEvent::SpeechStart { ts_samples: u64 }`
- `VadEvent::SpeechEnd   { ts_samples: u64 }`
- Optional per‑frame boolean `active` for monitoring.

Timing:
- `ts_samples` counts from capture start at 16 kHz (or provide Instant timestamps carried through the pipeline). Map samples to wall‑clock as needed.

Error modes:
- ML VAD init/load error (missing runtime/model) → fallback to Energy VAD.
- ML VAD runtime error or stall (no predictions > N seconds) → fallback.
- Input format mismatch → log error; Phase 1/2 ensure format already.

Success criteria:
- Stable start/stop boundaries on `captured_audio.wav` and mic.
- No callback blocking; consumer keeps up without drops at normal load.
- Clear metrics for activity, predictions, and fallback state.

## Silero VAD integration
- Sample rate: 16 kHz mono.
- Windowing: 512 samples per prediction; use crate’s provided stream/iterator to match hop/overlap.
- Post‑process: probability series → state machine with thresholds and minimal durations to reduce flapping.

### Smoothing & hysteresis
- Thresholds (starting point): on=0.5, off=0.3 with debounce durations below.
- Min durations: `min_speech_ms=200`, `min_silence_ms=400`.
- Pre/post‑roll: `pre_roll_ms=150`, `post_roll_ms=200` (sourced from recent frames in the consumer).

## Energy VAD fallback (reference defaults)
Use `docs/Reference/EnergyBasedVAD.md` defaults:
- frame_ms=20, hop_ms=10
- hpf_fc_hz=100, pre_emphasis=0.97
- ema_alpha=0.02, on_db_above_floor=9 dB, off_db_below_on=3 dB
- min_speech_ms=200, min_silence_ms=400
- pre_roll_ms=150, post_roll_ms=200
- zcr_gate≈0.10 optional

Implementation notes:
- Compute RMS dBFS per frame; maintain EMA noise floor on inactive frames; apply hysteresis and durations similar to SoX “silence”.
- Accumulate durations by hop length when using overlap.

## Configuration (add VadConfig)
- `enabled: bool` (default true)
- `engine: enum { Silero, Energy }` (default Silero, auto‑fallback to Energy)
- `silero: { p_on: f32=0.5, p_off: f32=0.3, min_speech_ms=200, min_silence_ms=400, pre_roll_ms=150, post_roll_ms=200 }`
- `energy: { frame_ms=20, hop_ms=10, hpf_fc_hz=100, pre_emphasis=0.97, ema_alpha=0.02, on_db=9.0, hyst_db=3.0, min_speech_ms=200, min_silence_ms=400, pre_roll_ms=150, post_roll_ms=200, zcr_gate=Option<f32> }`
- `health: { no_prediction_timeout_ms=3000, recovery_backoff_ms=1000, max_retries=3 }`

## Metrics & health
Metrics (extend `BasicMetrics`):
- `vad_predictions_total`, `vad_active_seconds_total`
- `vad_speech_segments_total`, `vad_current_active` (gauge)
- `vad_engine{silero|energy}` (gauge), `vad_fallback_switches_total`
- `last_vad_prediction_age_ms`, `vad_pre_roll_ms`, `vad_post_roll_ms`

Health checks:
- “No predictions” age > `no_prediction_timeout_ms` while frames flow → log warn, attempt Silero reinit; on failure, switch to Energy and mark degraded.
- Surface clear reasons in logs and a health snapshot.

## Implementation plan
1) Module scaffolding
   - `crates/app/src/audio/vad/mod.rs` with trait `VadEngine` and event types.
   - `silero.rs` implementation using vendored crate stream/iterator.
   - `energy.rs` implementation per Reference defaults.

2) Consumer wiring
   - Spawn `VadProcessor` task that drains the ring buffer, performs windowing, and emits `VadEvent`s over an internal channel.
   - Keep a small circular frame buffer to implement pre/post‑roll.

3) Config + telemetry
   - Add `VadConfig`; plumb through app config.
   - Expose metrics via `BasicMetrics`; add health monitor check.

4) Validation
   - Unit tests: energy VAD thresholds/hysteresis; silero stream window count matches expectations; pre/post‑roll logic.
   - Integration: run on `captured_audio.wav` and microphone; assert segment counts and reasonable boundaries.

5) Optional prototype: STT feed
   - Minimal: on `SpeechStart`, start a buffer; while active, append frames; on `SpeechEnd`, finalize and pass to a feature‑gated Vosk transcriber for a demo.

## Test matrix
- Happy path: conversational speech; boundaries stable.
- Noise: HVAC/fan; ensure inactivity maintained (no chattering).
- Music/background TV: verify reduced false positives; adjust ZCR gate if enabled.
- Missing ONNX runtime/model: fallback to Energy; continue emitting events.
- CPU stress: predictions continue at required cadence; health doesn’t flap.

## Risks & mitigations
- ONNX runtime issues: early init check; clear error; fallback.
- Latency growth: keep pre/post‑roll small; ensure consumer keeps up; monitor `last_vad_prediction_age_ms`.
- Sample‑rate mismatch: prefer device 16 kHz; otherwise downmix/resample in capture (out of scope here, but documented).

## Rollout
- Guarded by `vad.enabled` (default true) and `vad.engine`.
- Start with Silero; auto‑fallback to Energy on failure; log with reasons.
- Keep verbose debug logs initially; tighten after validation.

---

### Summary
Phase 3 introduces robust, observable voice activity detection with a safety‑net fallback. It consumes the Phase 2 ring buffer, emits clean start/stop events with pre/post‑roll, and prepares the ground for Phase 4 chunking and STT handoff.
