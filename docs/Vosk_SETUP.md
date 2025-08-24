# Vosk STT (Local) — Setup Notes

This repo can use Vosk for local streaming STT behind a feature flag.

## Enable (tentative)
- Add the `vosk` crate and feature gate in `crates/app/Cargo.toml` (to be wired):
  - `[features] vosk = []`
  - `vosk = "*"` (crate version TBD)
- Build with: `cargo build -F vosk`

## Model
- Download a 16 kHz model (e.g., small English) from the Vosk site.
- Point the app to `--vosk-model /path/to/model` (to be added).

## Audio format
- PCM S16LE, mono, 16 kHz. The capture pipeline already targets this.

## Next steps
- Wire the transcriber into the consumer path after VAD/chunking or in a simple probe.
- Emit partial results during speech; finalize on VAD SpeechEnd.

This is a stub until we pin the crate and API surface.
