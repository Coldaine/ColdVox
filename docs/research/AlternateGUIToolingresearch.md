---
doc_type: research
subsystem: gui
status: active
freshness: current
summary: Research on alternate GUI tooling approaches for Phase 3A-3C, including Tauri alternatives.
---

# Research: Alternate GUI Tooling & Spike Plan

## Strategic Timing
**Current Status:** Phase 3A (Tauri v2 + React Shell).
**Go/No-Go Criteria:**
The main Tauri-based approach should be pushed until **Phase 3C (Live Runtime Binding)** is complete. If, after binding real-time transcription and audio RMS data to the React frontend, the IPC latency or rendering performance cannot sustain the "Aurora Oracle" visual requirements (buttery-smooth 60fps animations and non-rectangular click-through transparency), then the following spike approaches become the primary path forward.

---

## 5-Approach Technical Spike Plan

If the Tauri runtime proves too heavy or introduces too much IPC latency for high-frequency audio-reactive visualizers, we will execute the following 5 spikes to determine the most viable pure-Rust or high-performance alternative.

### Spike 1: The "Slint Native" Approach
*   **Research Question:** Can Slint's properties be driven directly from the audio thread without a serialized bridge, testing raw data throughput?
*   **Methodology (PoC):** Build a 50-bar visualizer model updated at 60Hz.
*   **Success Criteria:** Measure CPU and memory overhead against the Tauri baseline. Determine if memory-sharing is natively supported without locking the UI thread.

### Spike 2: The "Xilem + Vello" Compute Approach
*   **Research Question:** Can Vello's compute-centric architecture handle complex generative shaders (aurora waves) directly fed by raw audio buffers with <1ms frame render time?
*   **Methodology (PoC):** Implement a generative Aurora shader using WGPU compute shaders.
*   **Success Criteria:** A compiled `wgsl` shader file and a benchmark test demonstrating <1ms frame render time when fed raw audio buffer arrays.

### Spike 3: The "GPUI / Zed" Performance Approach
*   **Research Question:** Does GPUI's custom GPU renderer maintain its sub-millisecond input-to-render claim for a transparent, floating desktop overlay?
*   **Methodology (Benchmarking):** Implement a high-frequency transcript stream simulating 100 words per second injected into the UI.
*   **Success Criteria:** Profiling shows frame-time jitter remains strictly < 16ms under sustained high-frequency load.

### Spike 4: The "Egui + WGPU" Custom Shader Approach
*   **Research Question:** Can we wrap a raw WGSL shader inside an immediate-mode egui window to achieve advanced visual effects without the overhead of a retained scene graph?
*   **Methodology (PoC):** Render a custom circular mask over a test pattern inside a transparent egui window.
*   **Success Criteria:** Visual confirmation of the shader, plus OS-level confirmation that click-through works outside the drawn mask.

### Spike 5: The "Winit + Raw Shaders" Minimalist Approach
*   **Research Question:** Is it viable to build a "Desktop Creature" (no rectangular border) using only windowing primitives (`winit`) and pixel-pushing (`tiny-skia` or `pixels`), bypassing UI frameworks entirely?
*   **Methodology (PoC):** Use `winit` to draw a 300x300 circle and dynamically manipulate OS-level hit-testing.
*   **Success Criteria:** Assert that dynamically calling `window.set_cursor_hittest(false)` based on alpha-channel boundaries successfully passes mouse events to the OS on Windows 11.

---

## Primary Candidate for Deep Investigation: Xilem + Vello

**Timebox:** 2 Days

### Executive Summary
In 2026, **Vello** represents the cutting edge for high-performance 2D vector graphics in Rust. Unlike Qt (scene graph) or Tauri (DOM), Vello uses **GPU compute shaders** for almost all of its rendering. This makes it uniquely suited for the "Aurora Oracle" vision.

### Research Questions
1.  **Hitbox Masking:** How stable is `winit`'s `set_cursor_hittest` on Windows 11 for creating non-rectangular "click-through" shapes?
2.  **Shared Memory:** Can we pass lock-free ring buffers directly from `coldvox-audio` to the Vello renderer, completely bypassing the JSON/IPC bottleneck?
3.  **Generative Visuals:** Can Vello's compute shaders handle multi-layered Perlin-noise (required for aurora waves) without dropping frames?

### Proposed Methodology
1.  **Windowing Test:** Write a standalone `winit` script that toggles `set_cursor_hittest` dynamically based on mouse position relative to a 150px radius.
2.  **Shader Prototype:** Write a basic WGSL shader for the aurora effect and feed it mock RMS data from a separate Rust thread.
3.  **Profiling:** Measure VRAM and CPU footprint.

### Definition of Done
*   [ ] A minimal, executable Rust prototype demonstrating a borderless, circular window.
*   [ ] The window successfully ignores clicks outside the visible circle.
*   [ ] A benchmark report comparing the prototype's resource usage against the current Tauri + WebView2 implementation.
*   [ ] A final Go/No-Go recommendation for adopting the Linebender stack.
