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
ColdVox is a modular Rust workspace providing real‑time audio capture, VAD, STT (Faster-Whisper), and cross‑platform text injection.

## Quick Start

**For Voice Dictation (Recommended):**
```bash
# Run with default Faster-Whisper STT and text injection (model auto-discovered)
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

**Note on Defaults**: Faster-Whisper STT is the default feature (enabled automatically), ensuring real speech recognition in the app and tests. This prevents fallback to the mock plugin, which skips transcription. Override with `--stt-preferred mock` or env `COLDVOX_STT_PREFERRED=mock` if needed for testing. For other STT backends, enable their features and set preferred accordingly.

### Whisper Model Setup
- **Python Package**: Install the `faster-whisper` Python package via pip
- **Models**: Whisper models are automatically downloaded on first use
- **Model Identifiers**: Use standard Whisper model names (e.g., "tiny.en", "base.en", "small.en", "medium.en")
- **Manual Path**: Set `WHISPER_MODEL_PATH` to specify a model identifier or custom model directory
- **Common Models**:
  - "tiny.en" (~39MB) - Fastest, lower accuracy
  - "base.en" (~142MB) - Good balance of speed and accuracy
  - "small.en" (~466MB) - Better accuracy
  - "medium.en" (~1.5GB) - High accuracy

## How It Works
1. **Always-on pipeline**: Audio capture, VAD, STT, and text-injection buffering run continuously by default. Raw 16 kHz mono audio is recorded to `logs/audio_dumps/` for later review.
2. **Voice activation (default)**: The Silero VAD segments speech automatically—no hotkey required.
3. **Push-to-talk (preview inject)**: Hold `Super+Ctrl` to stream buffered text into the preview/injection window when you need manual control. Release to stop feeding new text.

More detail: See [`CLAUDE.md`](CLAUDE.md) for full developer guide.

### Future Vision (Experimental)
- We're actively exploring an **always-on intelligent listening** architecture that keeps a lightweight listener running continuously and spins up tiered STT engines on demand.
- This speculative work includes decoupled listening/processing threads, dynamic STT memory management, and context-aware activation.
- Read the full experimental plan in [`docs/architecture.md`](docs/architecture.md#coldvox-future-vision). Treat it as research guidance—not a committed roadmap.

## Slow / Environment-Sensitive Tests
Some end‑to‑end tests exercise real injection & STT. Gate them locally by setting an env variable (planned):
```bash
export COLDVOX_SLOW_TESTS=1
cargo test -- --ignored
```
Headless behavior notes: see [`docs/text_injection_headless.md`](docs/text_injection_headless.md).

## License
Dual-licensed under MIT or Apache-2.0. See `LICENSE-MIT` and `LICENSE-APACHE` if present, else crate-level manifests.
