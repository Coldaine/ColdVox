---
doc_type: plan
subsystem: general
status: active
freshness: current
summary: Windows-focused multi-agent execution plan for ColdVox recovery and STT modernization.
---

# Windows Multi-Agent Recovery & Modernization Plan

> **Status**: ACTIVE
> **Target OS**: Windows 11
> **Hardware Priority**: Native CUDA / DirectML acceleration

This document is the definitive action plan to recover the ColdVox codebase from compilation blockers, sanitize the environment, stabilize the STT pipeline, and pave the way for a pure-Rust, Windows-native STT backend.

This plan is explicitly designed to be executed by **independent subagents**, with an absolute requirement for **live testing over mock testing** to validate audio and STT.

---

## 🤖 Subagent 1: Windows Build & Environment Stabilization

**Objective:** Get the project compiling on Windows and establish a single source of truth for the Python environment (`uv`).

**Action Items:**
1. **Rubato 1.0.1 Migration:**
    - Rewrite `StreamResampler` in `crates/coldvox-audio/src/resampler.rs` to conform to the `rubato = "1.0"` breaking changes.
    - Implement the `audioadapter` wrapping logic required by the new `SincFixedIn` API.
    - Validate with `cargo check -p coldvox-audio`.
2. **Environment Sanitization:**
    - Delete `requirements.txt` (it conflicts with `pyproject.toml`).
    - Remove `python = "3.13"` from `mise.toml` to prevent PyO3 compilation errors.
    - Enforce `.python-version` (`3.12`) and `uv sync` as the exclusive Python environment management path.
3. **Live Testing:**
    - Build a minimal capture test: `cargo run -p coldvox-audio --example live_capture` (create if missing) to prove microphone access and resampling on Windows without crashing.

---

## 🤖 Subagent 2: STT Lifecycle & Codebase Pruning

**Objective:** Stop aggressive STT garbage collection and delete "vaporware" features that clutter the project.

**Action Items:**
1. **Port PR #366 (STT GC Fix):**
    - Modify `SttPluginManager` in `crates/app/src/stt/plugin_manager.rs` to absolutely prevent garbage collection of the *currently active* STT plugin.
2. **Nuclear Pruning:**
    - Open `crates/coldvox-stt/Cargo.toml`.
    - Delete the `whisper`, `coqui`, `leopard`, and `silero-stt` feature flags.
    - Delete the corresponding unused plugin `.rs` files in `crates/coldvox-stt/src/plugins/`.
3. **Live Testing:**
    - Run `cargo run --features moonshine,text-injection`.
    - Perform a continuous 5-minute live dictation session into Notepad to prove the STT model does not unload unprompted.

---

## 🤖 Subagent 3: Windows-Native STT Path (Parakeet)

**Objective:** Transition away from fragile Python/PyO3 dependencies to a pure-Rust or ONNX-based Windows STT backend (Parakeet).

**Action Items:**
1. **Parakeet Compilation Verification:**
    - Run `cargo check -p coldvox-app --features parakeet` on Windows.
2. **CUDA / DirectML Probing:**
    - Ensure `ort` (ONNX Runtime) or the underlying ML framework for Parakeet is configured to leverage the Windows GPU (CUDA or DirectML).
3. **Live Validation Harness:**
    - Create a command-line harness to feed a known `.wav` file into the Parakeet backend directly: `cargo run -p coldvox-app --bin test_parakeet -- test.wav`.
    - Once file transcription is validated, pipe live Windows microphone input directly to Parakeet.
4. **Transition Plan:**
    - Once Parakeet achieves parity with Moonshine in accuracy and latency, document the deprecation of Moonshine and the removal of the PyO3 dependency.

---

## 🤖 Subagent 4: Documentation & Alignment

**Objective:** Ensure all documentation accurately reflects working reality and guides future development.

**Action Items:**
1. **Documentation Updates:**
    - Update `README.md`, `CLAUDE.md`, and project docs to remove any claims that `whisper` is available.
    - Update the Quick Start guide to firmly mandate `uv sync` on Windows before building Moonshine.
2. **Anchor Updates:**
    - Ensure `AGENTS.md` and `GEMINI.md` reflect `docs/plans/windows-multi-agent-recovery.md` as the core reality tracker.

---

## Execution Rules for Agents

- **No Mocking:** If testing VAD or STT, use the live Windows microphone or a local `.wav` file. Mocking audio buffers hides API mismatches.
- **Fail Fast:** If an agent encounters a broken build or deep API mismatch (like the `rubato` buffer issue), it must stop and prioritize fixing the compilation blocker over logical features.
- **Workspace Priority:** Use `cargo {cmd} -p {crate}` for speed, but always finish with `cargo check --workspace --all-targets` to ensure global safety.

## 🤖 Completed Remediation Tasks (2026-03-25)

**Dependency Audit & CI Fixes:**
- Added `choco install vcredist140` to `.github/workflows/ci.yml` to resolve native Windows issues.
- Upgraded Rust crates (`tar`, `tokio`, `serde`, `clap`, `tracing`, `serde_json`, `thiserror`, `log`) to their latest stable/compatible versions.
- Upgraded Python packages using `uv` while respecting the Python `<3.13` constraint.
- Refactored `coldvox-audio-quality` to use `rustfft` instead of `spectrum-analyzer` to completely remove the unmaintained `paste` crate.
- Removed dead `whisper` feature flags and references from `Cargo.toml`.
- Re-built PyO3 bindings for `coldvox-stt` using `uv sync` and `cargo build -p coldvox-stt --features moonshine`.
- Created `docs/system/Windows-dll-requirements.md` to explicitly document DLL dependencies (VCRUNTIME140.dll, PyO3 DLLs, ONNX/CUDA DLLs).
- Ran a full dependency audit (`cargo audit`, `cargo outdated`, `uv pip list`) and stored the results in `docs/research/dependency-audit-report-2026-03-25.md`.
