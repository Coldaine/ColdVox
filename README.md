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

Minimal root README. Full developer & architecture guide: see [docs/architecture.md](docs/architecture.md) and [.github/copilot-instructions.md](.github/copilot-instructions.md).

**Version History**: See [CHANGELOG.md](CHANGELOG.md) for all notable changes and release notes.

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

More detail: See [docs/architecture.md](docs/architecture.md) for full developer guide.

## Known Issues

### ⚠️ TEMPORARY: Hardcoded Microphone Device
**Status**: Emergency temporary fix in place (will be reverted)

**What**: The VAD microphone probe (`crates/app/src/probes/vad_mic.rs`) currently has the microphone device hardcoded to `"HyperX QuadCast"` to bypass broken device detection logic.

**Why**: Device detection/enumeration is currently unstable and causing test failures. This hardcoded workaround allows testing to continue while the underlying device management issues are being diagnosed and fixed.

**Location**: `crates/app/src/probes/vad_mic.rs:24-26`

**Impact**: The `mic_probe` test binary will only work with a HyperX QuadCast microphone. Other device selection is temporarily bypassed.

**Next Steps**: 
- Diagnose root cause of device detection instability (likely CPAL/ALSA enumeration issues)
- Implement proper device caching and debouncing (partially done in device monitor)
- Remove hardcoded device name once detection is stable
- Add comprehensive device detection tests

**Workaround for other devices**: Manually edit the device name in the probe source code if you need to test with a different microphone during this transition period.

### Documentation Review (Pending)
- [ ] Recent text injection changes consolidated paste behavior. A docs/diagram sweep is pending to reflect:
	- Clipboard-only injector is internal-only.
	- Single paste path (Clipboard+Paste with AT‑SPI→ydotool fallback) is last in order.
	- Updated diagrams exported in `diagrams/`.

## Slow / Environment-Sensitive Tests
Some end‑to‑end tests exercise real injection & STT. Gate them locally by setting an env variable (planned):
```bash
export COLDVOX_SLOW_TESTS=1
cargo test -- --ignored
```
Headless behavior: Text injection works in headless environments via clipboard strategies. See `docs/deployment.md` for configuration and `crates/coldvox-text-injection/README.md` for backend details.

## License
Dual-licensed under MIT or Apache-2.0. See `LICENSE-MIT` and `LICENSE-APACHE` if present, else crate-level manifests.
