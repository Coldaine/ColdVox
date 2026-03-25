---
doc_type: plan
subsystem: general
status: active
freshness: 2026-03-25
summary: Canonical ColdVox recovery plan. Single source of truth for Windows modernization, STT pipeline, and feature reality.
---

# ColdVox Recovery & Modernization Plan

> **Status**: ACTIVE (Updated 2026-03-25)
> **Target OS**: Windows 11 (primary), Linux secondary
> **Hardware Priority**: RTX 5090 / CUDA acceleration
> **This document supersedes:** `option-c-unified-architecture.md`, `critical-action-plan-REVIEW.md`, `stt-http-remote-plugin.md`

---

## What Actually Works (Verified)

| Feature | Status | Notes |
|---------|--------|-------|
| Default build | ✅ | `cargo build -p coldvox-app` |
| Moonshine STT | ✅ | Requires `uv sync` first |
| Text injection | ✅ | Default feature |
| Silero VAD | ✅ | Default feature |
| Tests | ✅ | `cargo test -p coldvox-app` |
| Rubato 1.0.1 | ✅ | Migration complete |

## What Is Broken / Removed

| Feature | Status | Notes |
|---------|--------|-------|
| Whisper | ❌ REMOVED | Nuclear pruning complete |
| Coqui | ❌ REMOVED | Nuclear pruning complete |
| Leopard | ❌ REMOVED | Nuclear pruning complete |
| Silero-STT | ❌ REMOVED | Nuclear pruning complete |
| Parakeet (in-process) | ⚠️ PLANNED | Compiles but needs runtime validation |

---

## Phase 1: Foundation (COMPLETE ✓)

**Subagent 1: Windows Build & Environment Stabilization** ✅ DONE
- [x] Rubato 1.0.1 Migration
- [x] Environment Sanitization (requirements.txt deleted, mise.toml fixed)
- [x] live_capture example created

**Subagent 2: STT Lifecycle & Codebase Pruning** ✅ DONE
- [x] STT GC Fix (active plugin protected from garbage collection)
- [x] Nuclear Pruning (whisper, coqui, leopard, silero-stt removed)
- [x] Plugin configs cleaned

**Subagent 3: Test Infrastructure** ⏳ NEXT
- [ ] Scope tests by OS (`#[cfg(unix)]` for Linux-only tests)
- [ ] Enable `cargo test --workspace` to pass on Windows
- [ ] Document: No mocking for audio/STT tests—use live hardware

---

## Phase 2: STT Modernization (IN PROGRESS)

We have **TWO viable paths** for modern STT. Choose based on validation results:

### Path A: HTTP Remote STT (RECOMMENDED FIRST)

Use local HTTP servers for STT to avoid PyO3 fragility entirely.

**Benchmarked Options (all tested, RTX 5090):**

| Service | Port | Latency (4s clip) | VRAM | Status |
|---------|------|-------------------|------|--------|
| Moonshine base | 5096 | 309ms | 0 GB | ✅ Working |
| Moonshine tiny | 5096 | 158ms | 0 GB | ✅ Fastest |
| Parakeet-TDT-0.6B | 8200 | 86ms | ~4 GB | ✅ GPU Docker |
| IBM Granite 4.0 1B | 5093 | 780ms | 4.3 GB | ✅ Best accuracy |
| Qwen3-ASR-1.7B | 5094 | 1.07s | 14-24 GB | ✅ 52 languages |

**Implementation:** See archived `stt-http-remote-plugin.md` for detailed code.

**Advantages:**
- Zero PyO3/Python dependencies in the main app
- Model runs in isolated container/process
- Easy to swap models without recompiling
- Works today

### Path B: In-Process Parakeet (VALIDATION REQUIRED)

Direct ONNX Runtime integration via `parakeet-rs` crate.

**Status:** Compiles with `cargo check -p coldvox-stt --features parakeet,parakeet-cuda`

**Gate:** Must validate actual transcription on RTX 5090 before committing.

**Validation Checklist:**
- [ ] `cargo run -p coldvox-stt --example verify_parakeet --features parakeet,parakeet-cuda -- test.wav` produces correct text
- [ ] Live microphone → Parakeet → transcription works
- [ ] GPU utilization confirmed (not falling back to CPU)

**If validation passes:** Proceed with Tauri GUI integration.
**If validation fails:** Default to Path A (HTTP Remote).

---

## Phase 3: GUI Modernization (OPTION C)

**Decision:** Replace Qt/QML with Tauri v2 + React.

**Architecture:**
```
┌──────────────────────────────────────────────┐
│            Tauri v2 Shell                     │
│  ┌────────────────────────────────────────┐   │
│  │  React Frontend (Dynamic Island UI)    │   │
│  │  - Glassmorphism floating pill         │   │
│  │  - States: Idle → Listening → Done     │   │
│  └──────────────┬─────────────────────────┘   │
│                 │ tauri::command invoke()      │
│  ┌──────────────▼─────────────────────────┐   │
│  │  Rust Backend (existing crates)        │   │
│  │                                        │   │
│  │  coldvox-audio ──► coldvox-vad-silero  │   │
│  │       │                    │            │   │
│  │       ▼                    ▼            │   │
│  │  coldvox-stt (HTTP or Parakeet)        │   │
│  │       │                                │   │
│  │       ▼                                │   │
│  │  Windows text injection (SendInput)    │   │
│  └────────────────────────────────────────┘   │
└──────────────────────────────────────────────┘
```

**Action Items:**
- [ ] Remove `crates/coldvox-gui/` (Qt/QML stub)
- [ ] Scaffold Tauri v2 app at `crates/coldvox-gui/`
- [ ] Port ColdVox_Mini React components
- [ ] Wire tauri::command handlers to Rust backend
- [ ] Global hotkey handling (PTT) via `rdev` or `windows` crate

**What We Drop:**
- Qt/QML GUI (incomplete)
- Moonshine PyO3 dependency (if using HTTP path)
- whisper.dll / Vulkan path
- Linux-specific injection (for now)

---

## Execution Rules

1. **No Mocking:** Audio and STT tests use live microphone or `.wav` files
2. **Fail Fast:** Build blockers take priority over features
3. **Windows First:** Linux support is secondary until Windows is solid
4. **Live Testing Over Unit Tests:** For audio pipeline, integration tests with real hardware matter more than mocks

---

## Superseded Documents

The following plans have been consolidated into this document:

| Old Plan | Content Merged Here |
|----------|---------------------|
| `option-c-unified-architecture.md` | Phase 3: GUI Modernization section |
| `critical-action-plan-REVIEW.md` | "What Actually Works" table |
| `stt-http-remote-plugin.md` | Phase 2: Path A (HTTP Remote STT) |
| `test-os-scoping.md` | Phase 1: Subagent 3 |

**Action:** Archive the above documents to `docs/archive/plans/`.

---

*This is the single source of truth. All other plans are historical.*
