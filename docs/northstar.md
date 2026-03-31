---
doc_type: architecture
subsystem: general
status: draft
freshness: stale
preservation: preserve
last_reviewed: 2026-02-09
last_reviewer: Patrick MacLyman
owners: Patrick MacLyman
review_due: 2026-05-10
version: 1.0.0
---

# ColdVox North Star

This document is the product and technical anchor for documentation decisions.
When other docs conflict with this, they should be updated, archived, or removed.

## Core Goals

1. **Windows 11 Primary Target** — ColdVox is a Windows-first voice pipeline.
2. **GPU Parakeet (CUDA) Primary STT** — Using parakeet-rs (latest v0.3.4) with NVIDIA CUDA for best performance on RTX-class GPUs.
3. **CPU Parakeet Fallback** — parakeet-rs auto-falls back to CPU; no GPU still works.
4. **DirectML Support** — parakeet-rs `features=["directml"]` for Windows non-NVIDIA GPU acceleration.
5. **Moonshine (PyO3) Tertiary Fallback** — Only if Python is available; fragile on Windows.
6. **HTTP Remote Emergency Fallback** — Cloud/remote STT as last resort.
7. **Tauri GUI Overlay** — Windows-native transparent overlay showing live transcription.
8. **Streaming STT** — parakeet-rs v0.3 supports EOU (End-of-Utterance) and Nemotron streaming models.

## STT Fallback Chain

The STT backend selection follows a strict priority order:

1. GPU Parakeet (CUDA)
2. GPU Parakeet (DirectML)
3. CPU Parakeet
4. Moonshine (PyO3, Python required)
5. HTTP Remote (cloud/remote, emergency only)

Dead stubs (Whisper, Candle, Coqui, Leopard, Silero-STT) must not be referenced as viable backends. Remove on sight.

## Execution Priority (Current)

- Primary: Complete end-to-end flow on Windows 11: mic → VAD → Parakeet STT → text injection.
- Upgrade parakeet-rs from v0.2 to v0.3.4 — required for CUDA, DirectML, CPU auto-fallback, and streaming support.
- Fix `ParakeetPlugin`: currently coded as GPU-only; must add CPU fallback path.
- GUI: Tauri overlay visible during capture, showing streaming partial transcription.
- No "no-STT" product mode for normal operation.
- Injection failure behavior target: retry once, then notify in overlay.
- Linux: Supported but secondary; no AT-SPI/Wayland testing priority.

## Key Technical Facts

- parakeet-rs v0.3.4 supports: CPU, CUDA, TensorRT, DirectML, WebGPU with auto-fallback.
- Current ColdVox uses parakeet-rs v0.2 — upgrade needed.
- `ParakeetPlugin` incorrectly coded as GPU-only — needs CPU fallback added.
- Moonshine is a fragile dependency due to PyO3 and Python environment requirements on Windows.
- Whisper, Candle, Coqui, Leopard, and Silero-STT feature flags are dead stubs; do not use.

## Documentation Policy Alignment

- Docs can be aspirational, research-oriented, or implementation-tracking, but must state which.
- Docs claiming shipped behavior must include verifiable references to code/config/tests.
- Outdated but valuable research should be archived, not discarded.
- Active docs should map clearly to one or more North Star goals.
