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
| **Audio Capture** | ⚠️ COMPILES | `coldvox-audio` unit tests pass. Live capture unverified due to environment security policy. |
| **VAD** | ⚠️ COMPILES | Silero VAD unit tests pass. End-to-end speech detection in a live stream unverified. |
| **STT (Moonshine)** | ⚠️ COMPILES | Python backend initialized. Transcription from a live buffer unverified. |
| **Text Injection** | ✅ STABLE | Enigo Windows injection verified via unit tests (non-hardware dependent). |
| **Workspace** | ✅ CLEAN | All crates compile on Windows. Legacy flags removed. |

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
