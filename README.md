# ColdVox
> ⚠️ **Internal Alpha** - This project is in early development and not ready for production use.

> **⚠️ CRITICAL**: Documentation and feature status changes quickly. See [`docs/plans/current-status.md`](docs/plans/current-status.md) for what currently works.

Minimal root README. Assistants should read [`AGENTS.md`](AGENTS.md).

## North Star

Current product and documentation direction is anchored in:

- [`docs/northstar.md`](docs/northstar.md)
- [`docs/plans/current-status.md`](docs/plans/current-status.md)
- [`docs/architecture.md`](docs/architecture.md)

## Quick Start

Status varies by STT backend and platform. For current "what works" details, see [`docs/plans/current-status.md`](docs/plans/current-status.md).

```bash
# Main app
cargo run -p coldvox-app --bin coldvox

# TUI dashboard
cargo run -p coldvox-app --bin tui_dashboard
```

Common Rust commands:

```bash
# Fast local feedback
cargo check -p coldvox-app

# Format check
cargo fmt --all -- --check
```
## Development
 - Install Rust (stable) and required system dependencies for your platform.
 - Use the provided scripts in `scripts/` to help with local environment setup.

### Developer Git Hooks

This project uses a "Zero-Latency" git hook standard powered by **[mise](https://mise.jdx.dev)** and **lint-staged**.

### Setup
1. **Install mise**: `curl https://mise.run | sh` (or see [docs](https://mise.jdx.dev/getting-started.html))
2. **Install dependencies**: `mise install`
3. **Activate hooks**: `mise run prepare` (runs automatically on `npm install`)

Hooks will now run automatically on `git commit`. To run manually:
```bash
mise run pre-commit
```

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

**STT Backends**: ColdVox supports multiple STT engines.
- **Parakeet**: NVIDIA-only, high performance (default if compiled).
- **Moonshine**: CPU-optimized, fast fallback.

See [Feature Flags & Hardware Config](docs/reference/feature_flags.md) for detailed hardware recommendations.

**Note on Defaults**: The `default` feature set enables VAD. You **must** enable an STT backend feature (`parakeet` or `moonshine`) to get speech recognition.
```bash
# Universal build (recommended)
cargo run --features "parakeet,moonshine"
```

### Configuration (Canonical Path)
- Canonical STT selection config lives at `config/plugins.json`.
- Any legacy duplicates like `./plugins.json` or `crates/app/plugins.json` are deprecated and ignored at runtime. A warning is logged on startup if they exist. Please migrate changes into `config/plugins.json` only.
- Some defaults can also be set in `config/default.toml`, but `config/plugins.json` is the source of truth for STT plugin selection.

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

- Review the [North Star](docs/northstar.md) and [current status](docs/plans/current-status.md).
- Follow the repository [Documentation Standards](docs/standards.md).
- Coordinate work through the [Documentation Todo Backlog](docs/todo.md).
- Assistants should read the [Assistant Interaction Index](docs/agents.md).
