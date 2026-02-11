# AGENTS.md

Canonical AI agent instructions for ColdVox. This file is the source of truth for all agent tools.

## Anchor

- Product and technical anchor: `docs/northstar.md`
- Documentation triage anchor: `docs/anchor-2026-02-09.md`
- Architecture direction: `docs/architecture.md`
- Current breakage/reality tracker: `docs/plans/critical-action-plan.md`
- CI source of truth: `docs/dev/CI/architecture.md`

If guidance conflicts, use this precedence:
1. `docs/northstar.md`
2. `docs/anchor-2026-02-09.md`
3. `docs/dev/CI/architecture.md`
4. other docs

## Current Product Direction (2026-02-09)

- Reliability first.
- Required end-to-end path: microphone input -> STT -> correct text injection.
- STT now: Moonshine.
- STT later: Parakeet.
- No normal-operation no-STT mode.
- Overlay shows live partial text while actively capturing (PTT and VAD modes).
- Injection failure policy: retry once, then notify in overlay.
- CUDA goal: best CUDA-capable model path, not hardware-specific micro-tuning.

## Project Overview

ColdVox is a Rust voice pipeline: audio capture -> VAD -> STT -> text injection.
Multi-crate Cargo workspace under `crates/`.

Key crates:
- `coldvox-app`
- `coldvox-audio`
- `coldvox-vad`
- `coldvox-vad-silero`
- `coldvox-stt`
- `coldvox-text-injection`
- `coldvox-telemetry`
- `coldvox-foundation`
- `coldvox-gui`

## Working Rules

Do:
- Prefer crate-scoped commands for iteration speed.
- Run `cargo fmt --all` before commit.
- Add tests for new behavior.
- Update docs when behavior or direction changes.
- Keep aspirational docs explicit about status and intent.

Do not:
- Claim Whisper is a working backend.
- Claim Parakeet is currently production-ready.
- Add conflicting CI instructions outside `docs/dev/CI/architecture.md`.
- Create `docs/agents.md`.

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
cargo run --features text-injection,moonshine
```

## Feature Flags

Default features: `silero`, `text-injection`.

- `silero`: Silero VAD
- `text-injection`: text injection backends
- `moonshine`: Current working STT backend (Python-based, CPU/GPU)
- `parakeet`: planned backend work; not current reliable path
- `whisper`: legacy/removed path; do not treat as active
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
- STT plugins: `crates/coldvox-stt/src/plugins/`
- Text injection manager: `crates/coldvox-text-injection/src/manager.rs`
- Build detection: `crates/app/build.rs`

## PR Checklist

- `./scripts/local_ci.sh` passes (or equivalent crate-scoped checks)
- Docs updated for behavior/direction changes
- `CHANGELOG.md` updated for user-visible changes
- No secrets committed
