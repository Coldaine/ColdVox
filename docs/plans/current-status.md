---
doc_type: status
subsystem: general
status: active
---

# Current Product Direction & Reality

*Audited April 2026. This is the source of truth for repository structure and branch strategy.*

## Branch Strategy & Mainlines

- **`main`**: The legacy Qt-based path. Currently stable but deprecated.
- **`tauri-base`**: The **integration branch and future mainline**. This branch accumulates the Tauri v2 GUI migration, dead code removal, HTTP-remote STT plugin, and lint gates. Agent PRs and feature branches should target `tauri-base` moving forward until it is promoted to `main`.

## Target Environment

- **OS:** Windows 11 priority.
- **Python Environment:** Exclusively managed by `uv`. Do NOT use `mise` or raw `pip` for Python packages. Ensure `.python-version` is respected.

## STT Backend Reality

- **Current Working (Legacy):** **Moonshine** is the current working backend but is a fragile dependency due to PyO3/Python 3.13 instabilities. It is being phased out.
- **Forward Path (Tauri-base):** **HTTP-Remote Parakeet**. A pure-Rust HTTP plugin (`HttpRemotePlugin`) is code-complete and designed to speak to a local, containerized Parakeet STT service optimized for NVIDIA CUDA/DirectML.
- **Vaporware:** The `whisper`, `coqui`, `leopard`, and `silero-stt` feature flags are dead stubs. They have been purged in `tauri-base` / PR #384.

## GUI Reality

- **Tauri v2 Shell:** ~80% complete on a UI contract level (React + Rust state machine). It is currently a demo/mock driver.
- **Missing Integration:** The last mile—wiring the real audio/STT pipeline (HTTP-remote Parakeet) and Text Injection into the Tauri shell—is at 0% and is the highest priority next step to make `tauri-base` functional.

## Known Blockers & Bugs

- **Memory Leaks:** PyO3/Moonshine unloading does not fully release memory on Windows 11.
- **Integration Gaps:** Text-injection can report false-positive "green" status when backends are skipped. True containerized validation for Parakeet has not been conducted.
