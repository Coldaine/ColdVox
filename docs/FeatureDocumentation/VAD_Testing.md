# VAD Testing with WAV Audio

The `test_silero_wav` example feeds a WAV file through the Silero VAD engine using
the same 16 kHz, 512‑sample framing as the app. This provides a realistic,
reproducible test path.

## Test Audio File

Sample WAVs live under `crates/app/` (for example, `test_audio_16k.wav`).
You can also set `TEST_WAV=/path/to/file.wav` to use a custom recording.

## Running the Demo

From the repo root:

```bash
cargo run --features examples --example test_silero_wav
```

The demo resamples input to 16 kHz if needed and prints events to stdout.
