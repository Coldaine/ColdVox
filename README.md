# ColdVox
> ⚠️ **Internal Alpha** - This project is in early development and not ready for production use.
## Development
 - Install Rust (stable) and required system dependencies for your platform.
 - Use the provided scripts in `scripts/` to help with local environment setup.

### Developer Git Hooks (optional but recommended)

To reduce the chance of CI failures from formatting, this repository includes a small pre-commit hook that runs `cargo fmt --all` before each commit and blocks the commit if `rustfmt` makes changes. To enable it for your local clone:

```bash
cd <repo-root>
./scripts/install-githooks.sh
```

This copies the hooks from `.githooks/` into `.git/hooks/` and makes them executable. You can remove or modify the hook if you want a different behavior.

# ColdVox

> ⚠️ **Internal Alpha** - This project is in early development and not ready for production use.

Minimal root README. Full developer & architecture guide: see [`CLAUDE.md`](CLAUDE.md).

## Overview
ColdVox is a modular Rust workspace providing real‑time audio capture, VAD, STT (Vosk), and cross‑platform text injection.

## Quick Start

**For Voice Dictation (Recommended):**
```bash
# Run with default Vosk STT and text injection (model auto-discovered)
cargo run --features text-injection

# With specific microphone device
cargo run --features text-injection -- --device "HyperX QuadCast"

# TUI Dashboard with controls
cargo run --bin tui_dashboard --features tui
```

**Other Usage:**
```bash
# VAD-only mode (no speech recognition)
cargo run

# Test microphone setup
cargo run --bin mic_probe -- list-devices
```

> Audio dumps: The TUI dashboard now records raw audio to `logs/audio_dumps/` by default. Pass `--dump-audio=false` to disable persistent capture.

**Note on Defaults**: Vosk STT is now the default feature (enabled automatically), ensuring real speech recognition in the app and tests. This prevents fallback to the mock plugin, which skips transcription. Override with `--stt-preferred mock` or env `COLDVOX_STT_PREFERRED=mock` if needed for testing. For other STT backends (e.g., Whisper), enable their features and set preferred accordingly.

### Vosk Model Setup
- **Small Model** (~40MB, included): Located at `models/vosk-model-small-en-us-0.15/`
- **Auto-Discovery**: Model automatically found when running from project root
- **Manual Path**: Set `VOSK_MODEL_PATH` for custom locations if needed
- **Verification**: `sha256sum -c models/vosk-model-small-en-us-0.15/SHA256SUMS`

## How It Works
1. **Always-on pipeline**: Audio capture, VAD, STT, and text-injection buffering run continuously by default. Raw 16 kHz mono audio is recorded to `logs/audio_dumps/` for later review.
2. **Voice activation (default)**: The Silero VAD segments speech automatically—no hotkey required.
3. **Push-to-talk (preview inject)**: Hold `Super+Ctrl` to stream buffered text into the preview/injection window when you need manual control. Release to stop feeding new text.

More detail: See [`CLAUDE.md`](CLAUDE.md) for full developer guide.

## Slow / Environment-Sensitive Tests
Some end‑to‑end tests exercise real injection & STT. Gate them locally by setting an env variable (planned):
```bash
export COLDVOX_SLOW_TESTS=1
cargo test -- --ignored
```
Headless behavior notes: see [`docs/text_injection_headless.md`](docs/text_injection_headless.md).

## License
Dual-licensed under MIT or Apache-2.0. See `LICENSE-MIT` and `LICENSE-APACHE` if present, else crate-level manifests.
