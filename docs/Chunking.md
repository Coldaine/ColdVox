# Audio Chunking and Centralized Resampling

This note describes the fixed-size frame chunker introduced in `crates/app/src/audio/chunker.rs`.

## Purpose

CPAL delivers input in variable callback sizes and device‑native sample
rates, but our downstream (VAD + STT) requires normalized, fixed frames:

-  Sample rate: 16 kHz
-  Frame size: 512 samples
-  Non-overlapping hop (512)

The chunker converts arbitrary device‑native `AudioCapture` frames into
exact 512‑sample frames suitable for the VAD/STT pipeline, handling
downmix + resample centrally.

## Contract

-  Input: Device‑native mono/stereo i16 PCM, read via `FrameReader` from the
	rtrb ring buffer as `audio::capture::AudioFrame`.
-  Output: Non-overlapping frames of exactly 512 samples, delivered as
	`audio::vad_processor::AudioFrame` (data + timestamp_ms).
-  Timestamps: Derived from the emitted-sample cursor at the configured
	sample rate (not from wall-clock), matching Silero’s expectation.
-  Resampling & Downmix: Centralized in the chunker. The chunker downmixes
	stereo→mono (averaging) and resamples to 16 kHz using a streaming sinc
	resampler. Capture writes device‑native data; all consumers receive 16 kHz
	mono 512‑sample frames.
-  Overlap: Not supported initially. If overlap is introduced later,
   timestamp math in both the chunker and Silero wrapper should be revisited.

## Design

-  Internal buffer: `VecDeque<i16>` accumulates samples. Incoming device
	samples are downmixed (if needed) and resampled to 16 kHz before chunking.
-  Emission: Pops exactly 512 samples per output frame; updates a
	`samples_emitted` counter to compute
	`timestamp_ms = samples_emitted * 1000 / sample_rate_hz`.
-  Backpressure: Output uses tokio broadcast; if there are no subscribers,
   send returns Err and we warn once per burst to avoid log spam.
-  Threading: Runs as a tokio task; drains frames from
	`FrameReader::read_frame()` in a loop and sleeps briefly when no data is
	available.

## Usage

1.  Start `AudioCaptureThread::spawn(...)` to feed the ring buffer and expose
	a device config broadcast.
2.  Build a `FrameReader::new(consumer, device_cfg.sample_rate,
	device_cfg.channels, cap, Some(metrics))`.
3.  Create a broadcast channel for `vad_processor::AudioFrame` and build
	`AudioChunker::new(...)` with `ChunkerConfig { frame_size_samples: 512,
	sample_rate_hz: 16000, resampler_quality }`.
4.  Wire device config updates via `.with_device_config(
	device_config_rx.resubscribe()
	)` so the chunker adjusts if the input rate/channels change.
5.  Spawn the chunker and subscribe in downstream processors (VAD/STT) via
	`audio_tx.subscribe()`.

## Edge Cases

-  Short or silent callbacks: Just buffer until 512 samples are available.
-  Stream stalls: The chunker loop yields with a short sleep when no data is
	available; no frames are emitted during stalls.
-  Channel disconnects: If the input channel disconnects, the chunker logs
	and exits.
-  Mismatched input format: Handled here. The chunker reconfigures its
	resampler if the input device sample rate changes (e.g., after a capture
	restart). Downmix is always applied when channels > 1.

## Resampler Quality

The chunker supports `ResamplerQuality` presets: `fast`, `balanced`
(default), and `quality`. Choose at runtime via CLI:

```bash
cargo run -- --resampler-quality fast   # or balanced / quality
```

## Future Extensions

-  Optional overlap (e.g., 50%) with consistent timestamp logic.
-  Capture-time resampling is deprecated. Centralize resampling/downmixing in
	the chunker for consistency and reduced callback load. If an emergency
	fallback is ever needed, gate it behind a feature flag.
-  Metrics: counters for frames produced, drops, and warnings.
-  Pre-roll tap for PTT (reusing the same accumulation buffer with a
	time-based window).
