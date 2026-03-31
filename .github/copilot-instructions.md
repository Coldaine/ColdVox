# AGENTS.md

Canonical AI agent instructions for ColdVox. This file is the source of truth for all agent tools.

## Anchor

- Product and technical anchor: `docs/northstar.md`
- Current execution plan: `docs/plans/windows-multi-agent-recovery.md`
- Architecture direction: `docs/architecture.md`
- CI source of truth: `docs/dev/CI/architecture.md`

If guidance conflicts, use this precedence:
1. `docs/northstar.md`
2. `docs/plans/windows-multi-agent-recovery.md`
3. `docs/dev/CI/architecture.md`

## Current Product Direction & Reality

- **Target OS:** Windows 11 priority.
- **Python Environment:** Exclusively managed by `uv`. Do NOT use `mise` or raw `pip` for Python packages. Ensure `.python-version` is respected.
- **STT Backend:**
  - **Parakeet (CUDA/DirectML/CPU):** The primary STT path. Pure Rust via ONNX Runtime. GPU primary, CPU fallback. Upgrade from v0.2 to v0.3.4 needed.
  - **Moonshine:** Working Python-based fallback via PyO3. Fragile on Windows.
  - **HTTP Remote:** Emergency cloud/remote STT fallback (stub).
  - **Dead stubs:** `whisper`, `coqui`, `leopard`, `silero-stt` feature flags are dead. Do not use.

## Project Overview

ColdVox is a Rust voice pipeline: audio capture -> VAD -> STT -> text injection.
Multi-crate Cargo workspace under `crates/`.

Key crates to know:
- `coldvox-app` (Main execution and binaries)
- `coldvox-audio` (Capture and resampling via rubato)
- `coldvox-stt` (STT Plugin logic)
- `coldvox-text-injection` (Output injection logic)

## Working Rules

**DO:**
- Use `cargo {cmd} -p {crate}` for iteration speed, but finish with `cargo check --workspace --all-targets`.
- Only use live testing (real microphone/`.wav` files) to test VAD and STT. Do not mock audio buffers.
- Treat `docs/plans/windows-multi-agent-recovery.md` as the absolute truth for what is currently broken or needing work.

**DO NOT:**
- Claim Whisper or Parakeet are currently production-ready.
- Modify Python dependencies without using `uv`.
- Auto-run commands that destroy data or commit unverified changes.

## Commands

File-scoped (preferred):
```bash
cargo check -p coldvox-stt
cargo clippy -p coldvox-audio
cargo test -p coldvox-text-injection
cargo fmt --all -- --check
```

Workspace (when needed):
```bash
./scripts/local_ci.sh
cargo clippy --workspace --all-targets --locked
cargo test --workspace --locked
cargo build --workspace --locked
```

Run:
```bash
cargo run -p coldvox-app --bin coldvox
cargo run -p coldvox-app --bin tui_dashboard
cargo run --features text-injection,parakeet
cargo run --features text-injection,parakeet-cuda  # With CUDA GPU
cargo run --features text-injection,moonshine       # Fallback with Python
```

## Feature Flags

- `silero`: Silero VAD
- `text-injection`: text injection backends
- `moonshine`: Current working STT backend (Python-based via PyO3, CPU/GPU) — fragile
- `parakeet`: Parakeet STT via ONNX Runtime (parakeet-rs) — primary path forward
- `parakeet-cuda`: Parakeet with CUDA GPU acceleration
- `parakeet-tensorrt`: Parakeet with TensorRT optimization
- `http-remote`: HTTP remote STT endpoint
- `examples`: example binaries
- `live-hardware-tests`: hardware test suites

## CI Environment

Canonical CI policy is `docs/dev/CI/architecture.md`.

Principle:
- GitHub-hosted runners handle fast general CI work.
- Self-hosted Fedora/Nobara runner handles hardware-dependent tests.

Do not use:
- Xvfb on self-hosted runner
- `apt-get` on Fedora runner
- `DISPLAY=:99` in self-hosted jobs

## Key Files

- Main entry: `crates/app/src/main.rs`
- Audio capture: `crates/coldvox-audio/src/capture.rs`
- VAD engine: `crates/coldvox-vad-silero/src/silero_wrapper.rs`
- STT plugins: `crates/coldvox-stt/src/plugins/` (parakeet.rs, moonshine.rs, http_remote.rs, mock.rs, noop.rs)
- Text injection manager: `crates/coldvox-text-injection/src/manager.rs`
- Build detection: `crates/app/build.rs`

## PR Checklist

- `./scripts/local_ci.sh` passes (or equivalent crate-scoped checks)
- Docs updated for behavior/direction changes
- `CHANGELOG.md` updated for user-visible changes
- No secrets committed
