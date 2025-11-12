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
# Run with default Candle Whisper STT (pure Rust, no Python) and text injection
cargo run --features "text-injection,candle-whisper"

# Alternative: Run with Python-based Faster-Whisper STT
cargo run --features "text-injection,whisper"

# With specific microphone device
cargo run --features "text-injection,candle-whisper" -- --device "HyperX QuadCast"

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

**Note on Defaults**: The Candle Whisper backend (pure Rust, no Python dependencies) is recommended for new deployments. Enable with `--features candle-whisper`. The Python-based Faster-Whisper (`--features whisper`) remains available for compatibility. Override with `--stt-preferred mock` or env `COLDVOX_STT_PREFERRED=mock` if needed for testing. For other STT backends, enable their features and set preferred accordingly.

### Configuration (Canonical Path)
- Canonical STT selection config lives at `config/plugins.json`.
- Any legacy duplicates like `./plugins.json` or `crates/app/plugins.json` are deprecated and ignored at runtime. A warning is logged on startup if they exist. Please migrate changes into `config/plugins.json` only.
- Some defaults can also be set in `config/default.toml`, but `config/plugins.json` is the source of truth for STT plugin selection.

### STT Backend Options

**Candle Whisper (Recommended - Pure Rust):**
- No Python dependencies, 100% Rust implementation
- Enable with `--features candle-whisper`
- Model setup: Same as Python Whisper but no package installation needed
- GPU acceleration via CUDA when available
- Supported models: "openai/whisper-tiny", "openai/whisper-base.en", "openai/whisper-small.en", etc.

**Faster-Whisper (Python-based):**
- Requires Python and `faster-whisper` package installation
- Enable with `--features whisper`
- Models are automatically downloaded on first use
- Model identifiers: "tiny.en", "base.en", "small.en", "medium.en"
- Manual path: Set `WHISPER_MODEL_PATH` to specify custom model directory

**Common Model Sizes:**
- "tiny" (~39MB) - Fastest, lower accuracy
- "base" (~142MB) - Good balance of speed and accuracy
- "small" (~466MB) - Better accuracy
- "medium" (~1.5GB) - High accuracy

## How It Works
1. **Always-on pipeline**: Audio capture, VAD, STT, and text-injection buffering run continuously by default. Raw 16 kHz mono audio is recorded to `logs/audio_dumps/` for later review.
2. **Voice activation (default)**: The Silero VAD segments speech automatically—no hotkey required.
3. **Push-to-talk (preview inject)**: Hold `Super+Ctrl` to stream buffered text into the preview/injection window when you need manual control. Release to stop feeding new text.

More detail: See [`CLAUDE.md`](CLAUDE.md) for full developer guide.

### Python 3.13 and PyO3
If your system default Python is 3.13, current `pyo3` versions may warn about unsupported Python version during build. Two options:

1) Prefer Python 3.12 for development tools, or
2) Build using the stable Python ABI by exporting:

```bash
set -gx PYO3_USE_ABI3_FORWARD_COMPATIBILITY 1  # fish shell
cargo check
```

We plan to upgrade `pyo3` in a follow-up to remove this requirement.

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

## Contributing

- Review the [Master Documentation Playbook](docs/MasterDocumentationPlaybook.md).
- Follow the repository [Documentation Standards](docs/standards.md).
- Coordinate work through the [Documentation Todo Backlog](docs/todo.md).
- Assistants should read the [Assistant Interaction Index](docs/agents.md).
