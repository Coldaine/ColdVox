# Audio Chunking for VAD

This note describes the fixed-size frame chunker introduced in `crates/app/src/audio/chunker.rs`.

## Purpose

CPAL delivers input in variable callback sizes, but our VAD (Silero) requires strictly fixed frames:
- Sample rate: 16 kHz
- Frame size: 512 samples
- Non-overlapping hop (512)

The chunker converts the arbitrary `AudioCapture` frames into exact 512-sample frames suitable for the VAD pipeline.

## Contract

- Input: 16 kHz mono i16 PCM, read via `FrameReader` from the rtrb ring buffer and represented as `audio::capture::AudioFrame`.
- Output: Non-overlapping frames of exactly 512 samples, delivered as `audio::vad_processor::AudioFrame` (data + timestamp_ms).
- Timestamps: Derived from the emitted-sample cursor at the configured sample rate (not from wall-clock), matching Silero’s expectation.
- Resampling: None. Upstream capture already normalizes to 16 kHz mono. If the input sample rate mismatches, the chunker logs a warning and proceeds without conversion.
- Overlap: Not supported initially. If overlap is introduced later, timestamp math in both the chunker and Silero wrapper should be revisited.

## Design

- Internal buffer: `VecDeque<i16>` accumulates samples until at least 512 are available.
- Emission: Pops exactly 512 samples per output frame; updates a `samples_emitted` counter to compute `timestamp_ms = samples_emitted * 1000 / sample_rate_hz`.
- Backpressure: On send failure to the output channel, the chunker logs an error and stops flushing additional ready frames in that iteration to avoid spinning.
- Threading: Runs on a dedicated thread; uses `recv_timeout(100ms)` to remain responsive to stop signals.

## Usage

1. Start `AudioCapture` to get `Receiver<capture::AudioFrame>`.
2. Create a channel `Sender<vad_processor::AudioFrame>` for the VAD processor.
3. Build and `spawn()` the `AudioChunker` with `ChunkerConfig { frame_size_samples: 512, sample_rate_hz: 16000 }`.
4. Feed the VAD processor from the chunker’s output.

## Edge Cases

- Short or silent callbacks: Just buffer until 512 samples are available.
- Stream stalls: The chunker loop times out every 100 ms to check the running flag; no frames are emitted during stalls.
- Channel disconnects: If the input channel disconnects, the chunker logs and exits.
- Mismatched input format: Logs a warning; assumes upstream will handle resampling/downmixing in future integrations.

## Future Extensions

- Optional overlap (e.g., 50%) with consistent timestamp logic.
- In-chunker resampling/downmixing as a fallback (currently avoided to keep the callback light and single-responsibility).
- Metrics: counters for frames produced, drops, and warnings.
- Pre-roll tap for PTT (reusing the same accumulation buffer with a time-based window).
