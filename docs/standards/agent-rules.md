# Working Rules

## DO

- Use `cargo {cmd} -p {crate}` for iteration speed, but finish with `cargo check --workspace --all-targets`.
- Only use live testing (real microphone/`.wav` files) to test VAD and STT. Do not mock audio buffers.
- Check `docs/plans/current-status.md` for what currently works and what's broken.

## DO NOT

- Claim Whisper or Parakeet are currently production-ready.
- Modify Python dependencies without using `uv`.
- Auto-run commands that destroy data or commit unverified changes.
