# Option C: Unified ColdVox Architecture

> **Status**: PROPOSAL (2026-03-24)
> **Goal**: Merge the best of ColdVox (Rust) and ColdVox_Mini (Python/Tauri) into a single, high-performance Windows dictation app optimized for NVIDIA 5090 / CUDA.

## Context

ColdVox and ColdVox_Mini each solve half the problem well:

| Component | ColdVox (Rust) | ColdVox_Mini (Python) | Winner |
|---|---|---|---|
| Audio capture | `coldvox-audio` (cpal) | Python sounddevice | **ColdVox** |
| VAD | `coldvox-vad-silero` (pure Rust ONNX) | Silero via Python ONNX | **ColdVox** |
| STT engine | `parakeet-rs` (CUDA/TensorRT via ort-rs) | `whisper.dll` (Vulkan ctypes) | **ColdVox** (5090) |
| Text injection | ydotool/AT-SPI (Linux) | Hybrid keyboard sim + clipboard (Windows) | **Mini** |
| GUI | Qt/QML TUI (incomplete) | Tauri v2 + React "Dynamic Island" (polished) | **Mini** |
| Voice commands | None | "new line", "scratch that", punctuation | **Mini** |
| Platform | Linux (Wayland/X11) | Windows 10/11 | **Mini** |

## Architecture

```
┌──────────────────────────────────────────────┐
│            Tauri v2 Shell (from Mini)         │
│  ┌────────────────────────────────────────┐   │
│  │  React Frontend (Dynamic Island UI)    │   │
│  │  - Glassmorphism floating pill         │   │
│  │  - State: Idle → Listening → Done      │   │
│  │  - Voice command feedback              │   │
│  └──────────────┬─────────────────────────┘   │
│                 │ tauri::command invoke()      │
│  ┌──────────────▼─────────────────────────┐   │
│  │  Rust Backend (from ColdVox crates)    │   │
│  │                                        │   │
│  │  coldvox-audio ──► coldvox-vad-silero  │   │
│  │       │                    │            │   │
│  │       ▼                    ▼            │   │
│  │  coldvox-stt (parakeet-cuda)           │   │
│  │       │                                │   │
│  │       ▼                                │   │
│  │  text-injection (Windows native)       │   │
│  └────────────────────────────────────────┘   │
└──────────────────────────────────────────────┘
```

## What We Take From Each

### From ColdVox (Rust crates)
- `coldvox-audio` — microphone capture via cpal (already cross-platform)
- `coldvox-vad-silero` — pure Rust Silero VAD
- `coldvox-stt` with `parakeet-cuda` feature — NVIDIA Parakeet 1.1B via ort-rs
  - ✅ Verified: `cargo check -p coldvox-stt --features parakeet,parakeet-cuda` passes cleanly
- `coldvox-foundation` — error types, shared primitives
- `coldvox-telemetry` — tracing/metrics infrastructure

### From ColdVox_Mini (Python/Tauri)
- `gui/` — Tauri v2 + React + TypeScript + Framer Motion frontend
- Voice command processor logic (port from Python to Rust)
- Windows text injection strategy (hybrid keyboard sim + clipboard)
- PTT hotkey handling via `GetAsyncKeyState` (port to Rust `windows` crate)
- `config.yaml` schema and user-facing configuration

## What We Drop

- **Moonshine STT** — replaced by Parakeet (no more PyO3 bridge fragility)
- **whisper.dll / Vulkan path** — replaced by parakeet-rs CUDA
- **Linux-specific injection** (ydotool, AT-SPI) — Windows-only target for now
- **Qt/QML GUI** — replaced by Tauri/React from Mini
- **All Python runtime** — fully Rust backend, zero Python dependency

## Implementation Phases

### Phase 1: Validate Parakeet Runtime (GATE)
Before committing to the merge, verify `parakeet-rs` actually transcribes on the 5090:
1. Build `coldvox-stt` with `parakeet-cuda`
2. Run the `verify_moonshine` example adapted for parakeet
3. Confirm model download, CUDA session creation, and transcription output

### Phase 2: Scaffold Tauri App
1. Replace `crates/coldvox-gui/` with a Tauri v2 app
2. Copy Mini's React frontend into `crates/coldvox-gui/src/`
3. Wire `tauri::command` handlers to call `coldvox-audio`, `coldvox-vad-silero`, `coldvox-stt`

### Phase 3: Windows Text Injection
1. Port Mini's hybrid injection to Rust using `windows` crate or `enigo`
2. Integrate with the existing `coldvox-text-injection` crate interface

### Phase 4: Voice Commands & Polish
1. Port Mini's command processor from Python to Rust
2. End-to-end testing: mic → VAD → STT → injection on Windows

## Risks

| Risk | Mitigation |
|---|---|
| `parakeet-rs` runtime failure on 5090 | Phase 1 is a hard gate; fall back to whisper.cpp CUDA if needed |
| Tauri v2 + existing crates integration friction | Start with a minimal "hello transcription" before full UI |
| Windows audio capture differences | `cpal` is cross-platform; test early |
| Voice command accuracy | Can iterate post-launch |

## Decision Required

> Where do we build this?
> 1. **Inside ColdVox repo** — replace `coldvox-gui` crate, keep workspace
> 2. **Inside ColdVox_Mini repo** — rip out Python, add Rust crates
> 3. **New repo** — clean start, cherry-pick what we need
