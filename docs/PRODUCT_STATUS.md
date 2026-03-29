---
doc_type: status
subsystem: general
status: active
freshness: current
last_reviewed: 2026-03-25
owners: Maintainers
---

# ColdVox Product Status

This document is the **single source of truth** for the current functional state of ColdVox.

## 🚀 Status: COMPILES (Headless Verified)

The project is currently in a "Compiles & Unit-Tested" state. Live hardware validation is the next milestone.

| Subsystem | Status | Technical Details |
|---|---|---|
| **Audio Capture** | ✅ VERIFIED | Live capture verified (288k samples/3s) via WASAPI. Signal flow confirmed to ring buffer. |
| **VAD** | ✅ VERIFIED | Silero VAD unit tests pass. Dynamic trigger verified in captured stream. |
| **STT** | ⚠️ BLOCKED | **Moonshine**: Build succeeds, but Windows App Control blocks every newly hashed test binary. Requires **Folder Exemption** for `target/`. **Parakeet**: 🏗️ NEXT (Requires Stable Moonshine first). |
| **Text Injection** | ✅ STABLE | Enigo Windows injection verified via unit tests. |
| **Workspace** | ✅ CLEAN | All crates compile on Windows. `runtime.rs` patched for Moonshine compatibility. |

## 🚧 Path to "Stable" (Project Roadmap)

To move from "Compiles" to a production-ready "Stable" status on Windows:

1.  **Security Policy Alignment**: Resolve the environment constraint preventing the execution of integration test binaries (proc-macro/FFI linking).
2.  **Mock Hardware Layer**: Implement a `MockDevice` in `coldvox-audio` that streams from a directory of `.wav` files, allowing "live" tests to run in headless CI environments.
3.  **End-to-End Integration**:
    - **Verify**: `crates/app/tests/integration/full_pipeline_test.rs`.
    - **Logic**: Simulate speech input -> VAD trigger -> STT transcription -> Event logged.
4.  **Tauri v2 Migration**: Replace the partial Qt/QML GUI with the Tauri shell to provide the final "Stable" user experience.

## 🛑 Known Blockers

- **Environment Security**: Current build environment restricts linking `.exe` binaries due to group policy. This prevents running the `live-hardware-tests` suite.
- **Parakeet Runtime**: In-process integration pending verification on RTX 5090 hardware.

## 🔗 Key References
- [North Star Vision](docs/northstar.md)
- [Multi-Agent Recovery Plan](docs/plans/windows-multi-agent-recovery.md)
- [Unified Architecture (Option C)](docs/archive/plans/option-c-unified-architecture.md)
