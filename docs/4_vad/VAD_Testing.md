# VAD Testing with WAV Audio

The `vad_demo` example feeds a WAV file through the current VAD engine using
the same 16 kHz, 512‑sample framing as the app. This provides a realistic,
reproducible test path.

## Test Audio File

Sample WAVs live under `crates/app/` (for example, `test_audio_16k.wav`).
You can also set `VAD_TEST_FILE=/path/to/file.wav` to use a custom recording.

## Running the Demo

From the repo root:

```bash
cargo run --example vad_demo -- [silero|level3]
```

-  Engine arg defaults to `silero` when omitted.
-  The demo resamples input to 16 kHz if needed and prints events to stdout.
