# ColdVox GUI Master Plan

> The single source of truth for the ColdVox overlay shell. It synthesizes prior vision documents into a buildable target without prescribing every implementation step.
>
> For visual and interaction principles, see [`docs/domains/gui/gui-design-overview.md`](../../domains/gui/gui-design-overview.md). For archived historical vision, see [`docs/archive/plans/gui/`](../../archive/plans/gui/).

---

## 1. Philosophy

ColdVox is a **floating dictation overlay** for Windows 11. It stays on top of other windows, listens to the microphone, shows recognized words as they are spoken, and injects finalized text into the active application.

Core principles (from [`docs/domains/gui/gui-design-overview.md`](../../domains/gui/gui-design-overview.md) and [`docs/northstar.md`](../../northstar.md)):
- **Immediate feedback while speaking** — the user must know the mic is hot.
- **Clear system state** — idle, listening, processing, ready, and error must be legible at a glance.
- **Low visual weight when idle** — the collapsed state should be unobtrusive.
- **Simple interruption controls** — stop, pause, and clear must be one click away.
- **Confidence that only finalized text gets injected** — partial text is visible in the UI but never committed prematurely.
- **Show words live** in both push-to-talk (PTT) and voice-activity-detection (VAD) modes.

---

## 2. Visual Direction

The archived **Aurora Oracle** plans ([`docs/archive/plans/gui/aspirational-gui-plan.md`](../../archive/plans/gui/aspirational-gui-plan.md)) established the visual language for ColdVox. The Qt/QML implementation stack is dead, but **the aesthetic is being ported to the Tauri/WebGL stack**:

- **Circular lens presence:** A strongly rounded, bubble-like overlay. The OS window remains rectangular (a Windows limitation in any stack), but the *content* is masked to a circle or superellipse.
- **Audio-reactive aurora:** A generative glow shader behind the text that pulses with voice volume.
- **Curved text rendering:** Finalized words arc along the bottom curve of the lens (implemented via SVG `textPath`, Canvas, or WebGL text rendering).
- **Finalization cue:** A brief highlight, ripple, or solar flare when an utterance is committed and injected.
- **Live word appearance:** Words materialize as they are recognized.

**Explicitly rejected from Aurora Oracle:** Qt/QML toolchain, hyper-realistic material shaders (clear coat, ice crystals, micro-scratches), and Linux-first windowing assumptions.

For detailed design qualities, see [`docs/domains/gui/gui-design-overview.md`](../../domains/gui/gui-design-overview.md).

---

## 3. Feature Inventory

The following features are derived from existing documentation. Each maps back to its source.

### 3.1 Overlay States
- **Collapsed state** — minimal pill showing presence and status ([`gui-design-overview.md`](../../domains/gui/gui-design-overview.md)).
- **Expanded state** — larger card with transcript, status, and controls ([`gui-design-overview.md`](../../domains/gui/gui-design-overview.md)).
- **Collapse/expand toggle** — one action to switch states ([`aspirational-gui-plan.md`](../../archive/plans/gui/aspirational-gui-plan.md)).
- **Always-on-top, frameless, transparent window** — implemented in current Tauri shell.

### 3.2 Transcription Display
- **Live provisional text** — dimmed, mutable text while speech is ongoing ([`northstar.md`](../../northstar.md)).
- **Finalized transcript** — solid, committed text that scrolls ([`gui-design-overview.md`](../../domains/gui/gui-design-overview.md)).
- **Auto-scroll to latest text** ([`gui-design-overview.md`](../../domains/gui/gui-design-overview.md)).
- **Clear distinction between partial and final** ([`gui-design-overview.md`](../../domains/gui/gui-design-overview.md)).
- **Do not inject partial text by default** (this document).

### 3.3 State Feedback & Visuals
- **Color-coded state indication** (idle, listening, processing, ready, error) ([`gui-design-overview.md`](../../domains/gui/gui-design-overview.md)).
- **Audio level visualization** ([`gui-design-overview.md`](../../domains/gui/gui-design-overview.md)).
- **Audio-reactive aurora** (ported from Aurora Oracle; multi-layered noise shader simplified to a performant WebGL/Canvas implementation).
- **Visible failure or retry messaging** ([`gui-design-overview.md`](../../domains/gui/gui-design-overview.md)).
- **Injection confirmation / finalization flash** — a brief glow pulse, ripple, or solar flare when finalized text is committed and injected (from Aurora Oracle §5.3).

### 3.4 Controls
- **Stop**, **Pause / Resume**, **Clear**, **Settings access** ([`gui-design-overview.md`](../../domains/gui/gui-design-overview.md)).

### 3.5 Activation Modes
- **VAD mode** — voice activity detection triggers transcription ([`docs/domains/audio/aud-user-config-design.md`](../../domains/audio/aud-user-config-design.md)).
- **Hotkey / PTT mode** — manual push-to-talk activation (this document; [`aud-user-config-design.md`](../../domains/audio/aud-user-config-design.md)).
- **Global hotkey works without window focus** (this document).

### 3.6 Settings & Persistence
- **Window position persistence** ([`gui-design-overview.md`](../../domains/gui/gui-design-overview.md)).
- **Transparency / opacity preference** ([`gui-design-overview.md`](../../domains/gui/gui-design-overview.md)).
- **Audio input device selection** ([`aud-user-config-design.md`](../../domains/audio/aud-user-config-design.md)).
- **Hotkey binding configuration** ([`gui-design-overview.md`](../../domains/gui/gui-design-overview.md)).
- **STT backend preference** (this document; [`parakeet-http-remote-integration-spec.md`](./parakeet-http-remote-integration-spec.md)).

### 3.7 Voice Commands (Deferred)
- **"Delete that"**, **"Clear all"**, **"Stop listening"** — intercepted in Rust before injection (this document).

### 3.8 Text Injection Behavior
- **Injection only at utterance end** (this document).
- **Injection confirmation via backend events** ([`docs/domains/text-injection/ti-overview.md`](../../domains/text-injection/ti-overview.md)).
- **Session state machine** (idle → buffering → waiting → inject) ([`ti-overview.md`](../../domains/text-injection/ti-overview.md)).
- **Retry once, then notify in overlay on failure** ([`northstar.md`](../../northstar.md)).

---

## 4. Implementation Stack

- **Frontend:** Tauri v2 + React
- **Windowing:** Transparent, frameless, always-on-top WebView2 window
- **Graphics:** WebGL/Canvas for audio-reactive visuals
- **Backend integration:** Rust commands/events bridging the existing `coldvox-audio`, `coldvox-stt`, and `coldvox-text-injection` pipeline

**Rejected alternatives** ([`docs/research/AlternateGUIToolingresearch.md`](../../research/AlternateGUIToolingresearch.md)):
- **Xilem + Vello** — rejected due to `wgpu` transparency issues on Windows.
- **egui + WGPU** — deferred fallback only.
- **Qt/QML** — archived.

---

## 5. Current Implementation Reality

The existing code is a **demo seam** ([`crates/coldvox-gui/README.md`](../../../crates/coldvox-gui/README.md)):

- The React frontend renders the collapsed pill, expanded card, transcript lanes, and control buttons.
- The Tauri backend only implements a `demo_driver` that emits fake events.
- **No real audio/STT/injection pipeline** is wired into the GUI crate yet.

The crate index in [`docs/reference/crates/coldvox-gui.md`](../../reference/crates/coldvox-gui.md) lists the entry points.

---

## 6. Remaining Work

### 6.1 Foundation
- [ ] Scope tests by OS (`#[cfg(unix)]` for Linux-only tests).
- [ ] Enable `cargo test --workspace` to pass on Windows.
- [ ] Document the rule: no mocking for audio/STT tests—use live microphone or `.wav` files.

### 6.2 STT Validation
- [ ] Validate in-process Parakeet on RTX 5090.
- **Decision gate:** If validation fails, drop in-process Parakeet and default to HTTP Remote Parakeet.

### 6.3 Phase 1: Core Dictation Loop

**Definition of Done:**
> When the global hotkey is pressed, the overlay enters a "listening" state. The aurora shader pulses with voice volume. Partial transcripts appear dimmed along the arc. Finalized transcripts solidify and arc along the lens, trigger text injection into the focused application, and produce a brief finalization flash or ripple at the moment of commitment.

**Architectural Boundary:**
> The backend in `coldvox-gui` owns the pipeline lifecycle. The frontend owns rendering and user input. Do not leak STT backend details into the React layer.

**Confirmed Integration Path:**
> The Tauri backend should import `coldvox-app` and spawn an `AppHandle`-like manager (from `coldvox_app::runtime`) in a background tokio task. This task drains the STT event channel, maps `TranscriptionEvent` to the existing `OverlaySnapshot` contract, and updates the shared state model. The frontend consumes `OverlaySnapshot` via the existing Tauri event bridge without contract changes.

**Verification — the loop is done when:**
1. Press the global hotkey → the overlay shows "Listening" and the aurora shader pulses with your voice volume.
2. Speak words → they appear dimmed and curved along the bottom arc of the lens as you speak.
3. Stop speaking → the provisional text solidifies into the curved final transcript lane, a brief finalization flash or ripple fires, and the text is injected into the active window.
4. Press Stop → the pipeline halts cleanly and the overlay returns to idle.

**Do not split this into smaller PRs that leave the loop half-wired.**

### 6.4 Phase 2: Voice Commands, Settings & Packaging

**Goal:** Polish and ship.

**Done when:**
- (a) Voice commands ("Delete that", "Clear all", "Stop listening") are handled in Rust.
- (b) Window position and opacity persist across restarts.
- (c) STT backend can be switched in settings.
- (d) Windows installer packages the app with required runtimes.

**Note:** The exact implementation of settings persistence and packaging will depend on how Phase 1 settles the backend/frontend boundary.

---

## 7. Long-Term Vision (Not Near-Term)

The **always-on intelligent listening** architecture is described in [`docs/architecture.md`](../../architecture.md) and staged on the roadmap in [`docs/architecture/roadmap.md`](../../architecture/roadmap.md) (v1.0+). It is explicitly out of scope until the core dictation loop is solid.

---

*This document is the single source of truth for the ColdVox GUI. If it conflicts with an archived plan, this document wins. If it conflicts with the code, the code wins for implemented features and this document wins for planned features.*
