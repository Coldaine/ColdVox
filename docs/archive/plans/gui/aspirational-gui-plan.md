---
doc_type: plan
subsystem: gui
status: draft
freshness: dead
preservation: delete
last_reviewed: 2025-10-19
owners: Documentation Working Group
version: 1.0.0
---

# Aspirational GUI Plan: The Aurora Oracle

## 1.0 Vision

This document outlines an ambitious, phased development plan to create the "Aurora Oracle," a sophisticated, audio-reactive visual interface for ColdVox. This GUI will serve as a visually stunning and highly functional overlay on the KDE Plasma desktop.

The plan integrates the advanced visual and interactive goals of the Aurora Oracle with the robust, step-by-step foundational plan for building a Qt/QML application with a Rust backend.

---

## 2.0 Phase 1: Environment Setup & Toolchain Validation

**Goal:** To prepare the development environment and project structure, culminating in a successful compilation of a minimal Rust-to-C++ bridge. This phase is identical to the foundational plan and ensures all dependencies are correctly configured.

*(This section contains the same steps as in the `comprehensive_gui_plan.md`: System Dependencies, Rust Toolchain, Project Scaffolding, CXX-Qt Integration, and Initial Build Validation.)*

---

## 3.0 Phase 2: Minimal Viable GUI (Smoke Test)

**Goal:** To render a simple QML window that demonstrates two-way communication with the Rust backend. This proves the entire Rust -> C++ -> QML pipeline is working before tackling the complex visuals.

*(This section contains the same steps as in the `comprehensive_gui_plan.md`: creating the minimal `GuiBridge`, `main.qml`, and `main.rs` to validate the toolchain.)*

---

## 4.0 Phase 3: Core Backend Integration

**Goal:** To connect the GUI to the live ColdVox audio and transcription pipeline. This phase focuses on data flow and state management, replacing mock data with real-time information.

### 4.1 The `ColdVoxController` QObject

The `GuiBridge` will be renamed and evolved into a `ColdVoxController`. This `QObject` will be the primary interface to the backend.

-   **Threading:** The `ColdVoxController` will be moved to a dedicated worker thread to prevent the UI from freezing. It will manage the `tokio` runtime and the ColdVox `AppHandle`.
-   **State Properties:** It will expose core application state to QML using `#[qproperty]` and `#[qenum]`:
    -   `activationMode: ActivationMode` (using `#[qenum]`)
    -   `pipelineStatus: PipelineStatus` (a new enum: `Running`, `Stopped`, `Error`)
    -   `isSpeaking: bool`
-   **Signals:** It will use `#[qsignal]` to emit events from the backend to the frontend:
    -   `vadEvent(VadStatus)`
    -   `newTranscription(TranscriptWord)`

### 4.2 Audio Processing Module

Instead of using `QtMultimedia`, we will leverage the existing, high-performance audio pipeline in ColdVox.

-   **Action:** The `ColdVoxController` will subscribe to the `AppHandle`'s audio and VAD event streams.
-   **Data Flow:**
    1.  The Rust backend captures audio and calculates RMS/peak values.
    2.  These values will be exposed as `#[qproperty]`s on the `ColdVoxController` (e.g., `audioLevelRms: f32`).
    3.  The `isSpeaking` property will be updated based on `VadEvent::SpeechStart` and `VadEvent::SpeechEnd`.
    4.  These properties will be read by the QML shaders to drive the aurora's intensity and speed.

### 4.3 Transcription Data Model

Implement the `TranscriptModel` and `TranscriptWord` (`#[qgadget]`) as planned, but connect them to the live transcription stream.

-   **Action:** When the `ColdVoxController` receives a `TranscriptionEvent::Final` from the backend, it will update the `TranscriptModel`. The `cxx-qt` library will automatically notify the QML `ListView` of the data change.

---

## 5.0 Phase 4: The Aurora Oracle - Visual Implementation

**Goal:** To build the advanced visual components of the Aurora Oracle within the now-functional GUI framework.

### 5.1 Circular Lens Container & Overlay

-   **Action:** Implement the frameless, transparent, always-on-top window as described in the foundational plan.
-   **QML:** Create a `CircularLens.qml` component that uses a `ShaderEffectSource` with a clipping shader to ensure all nested content is masked to a 300px circle. A subtle inner glow can be achieved with a `Glow` effect or another shader.

### 5.2 Generative Aurora System

-   **Action:** Create a GLSL fragment shader (`aurora.frag.glsl`) and load it into a `ShaderEffect` in QML.
-   **Shader Uniforms:** The shader will take uniforms that are bound to Rust properties:
    -   `uniform float time;` (driven by a QML `NumberAnimation`)
    -   `uniform float audioLevel;` (bound to `controller.audioLevelRms`)
    -   `uniform float rippleOrigin;`
    -   `uniform float rippleIntensity;`
-   **Implementation:** The shader will use multi-layered Perlin/Simplex noise functions to generate the volumetric aurora effect. The `audioLevel` will directly influence the `time` progression, making the aurora waves move faster with louder input.

### 5.3 Audio-Reactive Text Rendering

-   **Action:** This is the most complex visual part. It will be implemented using a `Shape` item in QML with a custom `ShapePath` that follows the circular arc of the lens.
-   **Text Bending:** The text itself will be rendered character by character along this path.
-   **Color Sampling:** The fragment shader for the text will need to sample the output of the aurora shader. This can be done by rendering the aurora to a texture first, then passing that texture to the text shader.
-   **Solar Flare & Ripple:** When a new word is finalized, the `ColdVoxController` will emit a signal. In QML, this signal will trigger:
    1.  A `PropertyAnimation` on the `rippleIntensity` uniform in the aurora shader, creating a ripple effect.
    2.  A brief, bright `Glow` effect on the current word in the `ListView`.

### 5.4 State Management (Collapsed/Expanded)

-   **Action:** Use QML's `State` and `Transition` elements.
-   **`collapsed` State:** The `CircularLens.qml`'s height will be reduced, and its `clip` property will be animated to show only the bottom arc. The aurora shader's intensity will be reduced to a calm, pale green.
-   **`expanded` State:** The default state, showing the full circle.
-   **Trigger:** A `Shortcut` in QML (`Ctrl+Shift+Space`) will toggle between the states with smooth, animated `Transitions`.

---

## 6.0 Phase 5: Polishing and Optimization

**Goal:** To add the final material properties and ensure the application is performant.

### 6.1 Material Properties & Textures

-   **Action:** The hyper-realistic material properties (clear coat, ice crystals, scratches) are advanced features. They will be implemented as additional layers in the primary fragment shader.
-   **Textures:** A texture loading system will be created to load CC-0 normal maps and scratch textures, which will be passed as uniforms to the shader.

### 6.2 Performance Optimization

-   **Action:** The `ColdVoxController` will expose system performance metrics to QML.
-   **LOD (Level of Detail):** The QML components will use these metrics to disable expensive effects (like the micro-scratches or subsurface scattering) if the frame rate drops below a target (e.g., 60 FPS).

---

## 7.0 Development Workflow & Troubleshooting

*(This section is identical to the `comprehensive_gui_plan.md`, containing the Justfile and troubleshooting guide.)*
