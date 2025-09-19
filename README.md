# ColdVox

> ⚠️ **Internal Alpha** - This project is in early development and not ready for production use.

Minimal root README. Full developer & architecture guide: see [`CLAUDE.md`](CLAUDE.md).

## Overview
ColdVox is a modular Rust workspace providing real‑time audio capture, VAD, STT (Vosk), and cross‑platform text injection.

## Quick Start

### Developer Setup

```bash
# One-time environment bootstrap
just setup

# (Optional) enable auto-setup on `cd` using direnv
direnv allow   # reruns setup automatically and loads .env
```

The setup command installs pinned tool versions (Rust 1.75, just, pre-commit, cargo-nextest),
configures git hooks, and pre-fetches dependencies. When direnv is enabled, the workspace runs
`just setup-auto` the first time you enter the directory and reloads environment variables from
`.env` on subsequent visits.

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

**Note on Defaults**: Vosk STT is now the default feature (enabled automatically), ensuring real speech recognition in the app and tests. This prevents fallback to the mock plugin, which skips transcription. Override with `--stt-preferred mock` or env `COLDVOX_STT_PREFERRED=mock` if needed for testing. For other STT backends (e.g., Whisper), enable their features and set preferred accordingly.

### Vosk Model Setup
- **Small Model** (~40MB, included): Located at `models/vosk-model-small-en-us-0.15/`
- **Auto-Discovery**: Model automatically found when running from project root
- **Manual Path**: Set `VOSK_MODEL_PATH` for custom locations if needed
- **Verification**: `sha256sum -c models/vosk-model-small-en-us-0.15/SHA256SUMS`

## How It Works
1. **Audio Capture** → **VAD** → **STT** → **Text Injection**
2. **Push-to-Talk**: Hold `Super+Ctrl`, speak, release (hotkey mode)
3. **Voice Activation**: Automatically detects speech and transcribes (VAD mode)

More detail: See [`CLAUDE.md`](CLAUDE.md) for full developer guide.

## Testing

For development, we recommend using `cargo nextest` as the preferred test runner for faster execution and better output:

```bash
# Install nextest
cargo install cargo-nextest --locked

# Run all tests with nextest (faster and more reliable)
cargo nextest run --workspace --locked

# For comprehensive testing guide, see docs/TESTING.md
```

## Slow / Environment-Sensitive Tests
Some end‑to‑end tests exercise real injection & STT. Gate them locally by setting an env variable (planned):
```bash
export COLDVOX_SLOW_TESTS=1
cargo test -- --ignored
```
Headless behavior notes: see [`docs/text_injection_headless.md`](docs/text_injection_headless.md).

## License
Dual-licensed under MIT or Apache-2.0. See `LICENSE-MIT` and `LICENSE-APACHE` if present, else crate-level manifests.
