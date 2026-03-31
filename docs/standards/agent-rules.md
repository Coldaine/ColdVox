# Agent Working Rules

## DO

- Use `cargo {cmd} -p {crate}` for iteration speed, but finish with `cargo check --workspace --all-targets`.
- Only use live testing (real microphone/`.wav` files) to test VAD and STT. Do not mock audio buffers.
- Treat `docs/plans/windows-multi-agent-recovery.md` as the absolute truth for what is currently broken or needing work.

## DO NOT

- Claim Whisper or Parakeet are currently production-ready.
- Modify Python dependencies without using `uv`.
- Auto-run commands that destroy data or commit unverified changes.
