# VAD Testing with Live Audio

The `vad_demo` binary has been updated to support testing the VAD engines with a live audio file. This provides a more realistic test scenario than the previous synthetic audio generation.

## Test Audio File

A test audio file, `test_audio.wav`, is located in the `crates/app` directory. This file is a sample from the [Free Spoken Digit Dataset](https://github.com/Jakobovski/free-spoken-digit-dataset) and is in the public domain.

## Running the Demo

To run the demo, use the following command from the root of the project:

```bash
cargo run --bin vad_demo [level3|silero]
```

Replace `[level3|silero]` with the VAD engine you want to test. If no engine is specified, it will default to `level3`.

The demo will read the `test_audio.wav` file, resample it to 16kHz, and then process it through the selected VAD engine. The VAD events will be printed to the console.
