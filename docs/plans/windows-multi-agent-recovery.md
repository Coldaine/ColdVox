---
doc_type: plan
subsystem: general
status: active
freshness: 2026-03-26
summary: Canonical ColdVox recovery plan. Single source of truth for Windows modernization, STT pipeline, GUI migration, and feature reality.
---

# ColdVox Recovery & Modernization Plan

> **Status**: ACTIVE (Updated 2026-03-26)
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
| Tauri overlay shell | ✅ | `crates/coldvox-gui` now hosts a demo-only Phase 3A seam |

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

**Wave-1 canonical backend profile for this workstream:** `http-remote` means **Parakeet CPU on `http://localhost:5092`**. Treat Moonshine, Granite, Qwen3-ASR, and Voxtral as deferred comparison profiles rather than equal first-wave implementation targets.

**Benchmarked Options (all tested, RTX 5090):**

| Service | Port | Latency (4s clip) | VRAM | Status |
|---------|------|-------------------|------|--------|
| Moonshine base | 5096 | 309ms | 0 GB | ✅ Working |
| Moonshine tiny | 5096 | 158ms | 0 GB | ✅ Fastest |
| Parakeet-TDT-0.6B | 5092 | 86ms | ~4 GB | ✅ Local OpenAI-compatible server |
| IBM Granite 4.0 1B | 5093 | 780ms | 4.3 GB | ✅ Best accuracy |
| Qwen3-ASR-1.7B | 5094 | 1.07s | 14-24 GB | ✅ 52 languages |
| Voxtral-Mini-4B | 5095 | ~4.9s | 8.25 GB | ⚠️ Slow on Windows |

**Implementation:** See archived `stt-http-remote-plugin.md` for detailed code.

**Advantages:**
- Zero PyO3/Python dependencies in the main app
- Model runs in isolated container/process
- Easy to swap models without recompiling
- Works today

**Wave-1 ColdVox client upload contract:** ColdVox sends a mono 16 kHz 16-bit WAV upload to `POST /v1/audio/transcriptions`, probes readiness with `GET /health`, and expects JSON responses containing `text`. This freezes the ColdVox client payload shape for wave 1; it does **not** claim the backend only accepts that audio shape.

**Local endpoint reality on this machine:**
- Canonical wave-1 backend — Parakeet CPU: `http://localhost:5092/v1/audio/transcriptions`
- Optional GPU comparison profile — Parakeet GPU: `http://localhost:8200/audio/transcriptions`
- Deferred comparison profile — Moonshine: `http://localhost:5096/v1/audio/transcriptions`
- Deferred comparison profile — Granite: `http://localhost:5093/v1/audio/transcriptions`
- Deferred comparison profile — Qwen3-ASR: `http://localhost:5094/v1/audio/transcriptions`
- Deferred comparison profile — Voxtral: `http://localhost:5095/v1/audio/transcriptions`

**Future-facing transport rule:**
- Ship batch HTTP first for finalized utterances.
- Preserve a clean upgrade path to true streaming partials later, most likely via WebSocket on localhost.
- Product target remains: words materialize live while speaking, but text injection commits at utterance end.

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

**Current status:** Phase 3A is now in place as a thin transparent overlay shell with typed commands/events and a demo driver. It proves the host-shell contract without claiming real backend integration.

**Product goal:** Merge the best of ColdVox and ColdVox_Mini into a single Windows-first dictation app.

**What we keep from each codebase:**

| Source | Keep |
|--------|------|
| ColdVox | `coldvox-audio`, `coldvox-vad-silero`, `coldvox-stt`, `coldvox-foundation`, `coldvox-telemetry` |
| ColdVox_Mini | Tauri v2 + React shell concepts, floating overlay behavior ideas, Windows-oriented injection flow ideas, voice command ideas, user-facing config patterns |

**Architecture:**
```
┌──────────────────────────────────────────────┐
│            Tauri v2 Shell                     │
│  ┌────────────────────────────────────────┐   │
│  │  React Frontend (Phase 3A host shell)  │   │
│  │  - restrained overlay layout           │   │
│  │  - states: idle / listening /          │   │
│  │    processing / ready / error          │   │
│  │  - partial vs final transcript lanes   │   │
│  │  - nearby control bar + shell seam     │   │
│  └──────────────┬─────────────────────────┘   │
│                 │ tauri::command invoke()      │
│  ┌──────────────▼─────────────────────────┐   │
│  │  Rust Host Shell + later backend bind  │   │
│  │                                        │   │
│  │  Phase 3A: bootstrap + demo events     │   │
│  │  Phase 3B+: bind coldvox-audio / STT   │   │
│  │  and later injection workflows         │   │
│  └────────────────────────────────────────┘   │
└──────────────────────────────────────────────┘
```

**Action Items:**
- [x] Remove the active Qt/QML stub path from `crates/coldvox-gui/`
- [x] Scaffold a Tauri v2 + React shell at `crates/coldvox-gui/`
- [x] Add a typed command/event seam with a demo driver for overlay states and transcript updates
- [ ] Replace the demo seam with real runtime bindings from existing Rust crates
- [ ] Implement live partial transcript updates from the actual audio/STT path
- [ ] Keep partials visible in UI while buffering injection until final text
- [ ] Port ColdVox_Mini voice command behaviors into Rust
- [ ] Port Windows-first text injection behavior into `coldvox-text-injection`
- [ ] Global hotkey handling (PTT) via `rdev` or `windows` crate
- [ ] Reconcile Mini-style user config with ColdVox runtime/config files

**What We Drop:**
- Qt/QML GUI (incomplete)
- Moonshine PyO3 dependency (if using HTTP path)
- `whisper.dll` / Vulkan path
- Linux-specific injection (for now)

**Implementation phases inside GUI modernization:**
- Phase 3A: Replace the Qt/QML shell with Tauri v2 and deliver a floating overlay host shell with a typed demo seam. ✅
- Phase 3B: Replace the demo seam with real runtime commands/events and deepen the React shell where needed.
- Phase 3C: Surface live partial transcripts and rich state transitions without injecting partial text.
- Phase 3D: Port Windows-first injection ergonomics and voice command behaviors.
- Phase 3E: Package the app for Windows with sane first-run model/runtime setup.

**Non-negotiable UX rules:**
- Show words live while speaking in both PTT and VAD modes.
- Do not inject partial text by default; inject committed final text at utterance end.
- Provide visible feedback for capture, inference, command handling, and injection success/failure.

**Critical implementation details:**
- Global hotkeys:
  Use a real background listener on Windows. The GUI cannot depend on focus to start/stop dictation.
- Voice commands:
  Port Mini's command logic to Rust rather than burying it in the frontend. Command handling belongs near STT/post-processing.
- Packaging and model/runtime distribution:
  Treat large models and runtime DLLs as first-run assets or managed sidecars, not as assumptions on a clean Windows machine.
- Streaming roadmap:
  Batch HTTP is acceptable for Phase 2, but the GUI/event model must stay compatible with future true streaming partials over a long-lived transport.

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
| `option-c-unified-architecture.md` | Phase 3: GUI modernization, Mini migration scope, Windows injection/hotkey/voice-command requirements |
| `critical-action-plan-REVIEW.md` | "What Actually Works" table |
| `stt-http-remote-plugin.md` | Phase 2: Path A (HTTP Remote STT), endpoint strategy, future streaming direction |
| `test-os-scoping.md` | Phase 1: Subagent 3 |

**Action:** Archive the above documents to `docs/archive/plans/`.

---

*This is the single source of truth. All other plans are historical.*
