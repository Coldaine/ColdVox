---
doc_type: status
subsystem: general
status: active
last_reviewed: 2026-03-31
---

# ColdVox Current Status & Direction

## Target Platform
- **Primary OS:** Windows 11
- **GPU Target:** NVIDIA RTX-class (RTX 5090 target), CUDA-first

## STT Backend Status

| Backend | Status | Feature Flag | Notes |
|---------|--------|--------------|-------|
| Parakeet (GPU/CUDA) | **Primary** — compiles, needs runtime validation | `parakeet`, `parakeet-cuda` | parakeet-rs v0.2 → upgrade to v0.3.4 needed |
| Parakeet (DirectML) | **Planned** — Windows non-NVIDIA fallback | `parakeet` + directml | parakeet-rs supports this in v0.3 |
| Parakeet (CPU) | **Planned** — universal fallback | `parakeet` | parakeet-rs auto-falls back to CPU |
| Moonshine (PyO3) | **Working** — fragile on Windows | `moonshine` | Python dependency, PyO3 0.28 |
| HTTP Remote | **Stub** — emergency fallback | `http-remote` | Not validated |
| Whisper/Candle | **Dead** — do not use | — | Remove all references |
| Coqui/Leopard/Silero-STT | **Dead** — do not use | — | Never implemented |

## GUI Status
- **Framework:** Tauri 2.x (native Windows webview)
- **Crate:** `coldvox-gui` (`crates/coldvox-gui/src-tauri`)
- **Status:** Shell exists with overlay model, window management, demo script
- **Next:** Connect to live audio pipeline, wire STT events to overlay

## What Needs Work (Priority Order)
1. Upgrade parakeet-rs to v0.3.4 and add CPU/DirectML fallback support
2. Fix ParakeetPlugin to support CPU execution (currently GPU-only incorrectly)
3. Implement Parakeet Drop trait for GPU resource cleanup (#290)
4. Fix blocking locks in audio callback (#283)
5. Wire Tauri GUI to live audio/STT pipeline
6. Add plugin manager tests (#288)
7. Fix failover cooldown oscillation (#291)
8. Fix concurrency hazards in text injection (#292)
9. Add cancellation tokens for background tasks (#289)
10. Update AGENTS.md/CLAUDE.md with correct feature flags (#286, #287)

## What Is Deprioritized
- Linux AT-SPI focus backend (Windows uses different API)
- Wayland/X11 compositor testing matrix
- ydotool/kdotool injectors (Linux-only)
- VM-based compositor testing
- Whisper/Candle migration (dead path)
