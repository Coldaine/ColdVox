# ColdVox — Monorepo Onboarding & Integration Notes

This document is a concise overview of the ColdVox codebase and practical steps to prepare it for inclusion in a monorepo where parts may be reused or refactored across multiple apps.

## Quick facts

- Main inspect location: `crates/` (primary app crate: `crates/app`).
- Number of crates: 11 (under `crates/`).

## Crates and one-line purpose

- `crates/app` — Main application: CLI/TUI binaries and glue for audio → VAD → STT → injection; re-exports common types.
- `crates/coldvox-audio` — Audio capture, normalization, resampling, SPSC ring buffer, and 512-sample framing.
- `crates/coldvox-foundation` — Core scaffolding and shared types: `AppState`, shutdown/health helpers, common errors/configs.
- `crates/coldvox-gui` — GUI components (QML/desktop) — currently incomplete/stubbed; target for standardization.
- `crates/coldvox-stt` — STT core abstractions, events, processing, and plugin architecture.
- `crates/coldvox-stt-vosk` — Vosk offline STT plugin (feature-gated; native `libvosk` dependency).
- `crates/coldvox-telemetry` — Pipeline metrics, FPS tracking, telemetry helpers.
- `crates/coldvox-text-injection` — Platform-aware text injection backends and StrategyManager (AT‑SPI, clipboard, ydotool, enigo, etc.).
- `crates/coldvox-vad` — VAD core traits, configuration, constants (frame size, sample rate, etc.).
- `crates/coldvox-vad-silero` — Silero ONNX-based VAD implementation (default; feature `silero`).
- `crates/voice-activity-detector` — Supporting crate for ONNX inference and VAD model logic (modified third-party code).

## Key architectural decisions and runtime contracts

- Strong multi-crate separation of concerns (audio, VAD, STT, injection, telemetry, UI). This enables reuse when the API surface is stable.
- Audio pipeline contract: CPAL callback → i16 samples → `AudioRingBuffer` (SPSC) → `FrameReader` → `AudioChunker` → broadcast to subscribers. Chunker emits 512-sample frames at 16 kHz (32 ms).
- Standardized windowing/resampling: many components assume 16 kHz and 512-sample frames (constants live in `coldvox-vad`).
- VAD: default ML-based Silero ONNX engine (`silero` feature). Legacy energy-based VAD exists behind a feature but is disabled by default.
- STT is optional and feature-gated (`vosk`) and built around a plugin architecture with lifecycle/telemetry.
- Text injection is platform-aware with a StrategyManager that composes backends and includes clipboard restore logic and per-method cooldown caching.
- Real-time and robustness concerns: dedicated capture thread, lock-free SPSC ring buffer, watchdog logic (5s no-data restart), and attention to backpressure (drops with logged warnings).
- Configuration layering: `config/default.toml` with environment overrides `COLDVOX_*` and minimal CLI flags.

## Notable implementation patterns

- `coldvox-foundation` centralizes shared types and lifecycle utilities — a natural stable surface to expose across a monorepo.
- Platform and heavy dependencies are feature-gated (`vosk`, `silero`, `ydotool`, etc.) making builds more flexible.
- The app crate re-exports useful types to reduce consumer coupling to many crates.
- End-to-end tests use deterministic WAV-driven inputs where possible; there are mocks for injection and STT paths to support testability.

## Risks and friction points when importing to a monorepo

- Native/system dependencies: `libvosk`, ONNX runtimes, `ydotool`, AT‑SPI and other GUI stack tooling add cross-platform build and CI complexity.
- Platform-specific code paths (Wayland vs X11, KDE integration) require special handling or isolated adapters for reuse.
- GUI (`crates/coldvox-gui`) is incomplete and uses QML; it will need to be aligned to the suite's UI standards.
- Branch-level or in-progress regressions (e.g., AT‑SPI focus detection) exist and should be fixed before broad reuse.
- Tests that need GUI/audio hardware may be flaky in headless CI without mock adapters.

## Practical checklist to prepare for monorepo inclusion

1. Export and stabilize public APIs
   - Identify public surfaces in `coldvox-foundation`, `coldvox-audio`, `coldvox-vad`, `coldvox-stt`, and `coldvox-text-injection`.
   - Add crate-level READMEs and curated re-exports for stable contracts.

2. Stabilize shared core
   - Keep platform-agnostic logic (resampler, chunker, ring buffer, constants) as the core reusable pieces.

3. Isolate system dependencies behind adapters
   - Move heavy platform bindings and native integrations behind small adapter crates and feature flags.
   - Provide `noop` or `mock` implementations for CI and other apps.

4. Standardize the GUI
   - Convert `crates/coldvox-gui` into a standalone component library with a documented UI contract for wiring app logic.
   - Align technology and style with your suite (or provide a thin adapter).

5. CI and tests
   - Add workspace-level CI jobs that build a minimal feature set and separate jobs for feature-heavy builds (e.g., `--features vosk`, `--features silero`).
   - Add contract tests that verify the 512 @ 16k audio frame contract using mocked inputs.

6. Docs & onboarding
   - Per-crate short READMEs (purpose, public API, feature flags, external deps).
   - Top-level integration guide (this file) documenting build flags and system reqs.

## Low-risk refactors to accelerate reuse

- Promote `coldvox-foundation` as the single stable dependency for common runtime types (errors, state, shutdown, config types).
- Create small adapter crates:
  - `coldvox-stt-api` exposing only the STT plugin trait & event types.
  - `coldvox-injection-api` exposing injection strategy interfaces and a mock injector.
- Add `mock-injector` and `mock-stt` crates for headless CI and unit tests.
- Ensure feature flags are consistent and additive across crates (`vosk`, `silero`, `text-injection`).

## Integration notes for monorepo owners

- Build: default `cargo build` from `crates/app` compiles the app; include `--features vosk` to enable Vosk STT.
- Config: runtime configuration lives in `config/default.toml` and supports environment overrides.
- Runtime system requirements: full functionality requires native libs and system tools (libvosk, ONNX runtime, Wayland/X11, uinput tools, etc.).
- Tests: use the WAV-driven E2E tests as deterministic integration checks; prefer mock adapters for CI.

## Suggested immediate follow-ups I can implement

- Produce a per-crate public API inventory (exported types and entry points) to plan refactors.
- Add small adapter/mocks for STT and injection so the core crates compile and test in headless CI.
- Draft `CONTRIBUTING.md` and a more detailed `MONOREPO_ONBOARDING.md` (this file) with build matrix suggestions.

---

If you want, I can now (choose one):
- generate the per-crate API inventory by scanning the workspace, or
- scaffold `mock-stt` and `mock-injector` crates and a minimal CI-friendly example.

Tell me which and I’ll proceed.
