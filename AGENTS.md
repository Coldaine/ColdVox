# AGENTS.md

Canonical AI agent instructions for ColdVox. This file is the source of truth for all agent context.

## Anchor

- Read these first before using any other repository docs.

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
  - **Moonshine:** The current working backend, but considered a fragile dependency due to PyO3.
  - **Parakeet:** The designated successor for a pure-Rust/Windows-native STT pipeline (CUDA/DirectML). It *does* compile; focus on runtime validation.
  - **Vaporware:** The `whisper`, `coqui`, `leopard`, and `silero-stt` feature flags are dead stubs. Do not attempt to use them.

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

## Feature Flags

- Default features: `silero`, `text-injection`.
- `moonshine`: Current working Python-based STT.
- `parakeet`: Planned backend work (requires runtime testing).
