# ColdVox

Minimal root README. Full developer & architecture guide: see [`CLAUDE.md`](CLAUDE.md).

## Overview
ColdVox is a modular Rust workspace providing real‑time audio capture, VAD, STT (Vosk), and cross‑platform text injection.

## Quick Start
```bash
cargo run --features vosk         # Run with Vosk STT (requires model)
cargo run                         # Run without STT (VAD + pipeline)
```

## Vosk Model
Included small English model at `models/vosk-model-small-en-us-0.15/`.
Integrity checks: `sha256sum -c models/vosk-model-small-en-us-0.15/SHA256SUMS`.
More detail: `THIRDPARTY.md` and `crates/coldvox-stt-vosk/src/model.rs`.

## Slow / Environment-Sensitive Tests
Some end‑to‑end tests exercise real injection & STT. Gate them locally by setting an env variable (planned):
```bash
export COLDVOX_SLOW_TESTS=1
cargo test -- --ignored
```
Headless behavior notes: see [`docs/text_injection_headless.md`](docs/text_injection_headless.md).

## License
Dual-licensed under MIT or Apache-2.0. See `LICENSE-MIT` and `LICENSE-APACHE` if present, else crate-level manifests.
