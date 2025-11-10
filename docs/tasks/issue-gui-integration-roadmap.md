# Issue: GUI Integration Roadmap

**Status:** New (Consolidated)
**Original Issues:** #58, #59, #60, #61, #62, #63
**Epic:** GUI
**Tags:** `roadmap`, `gui`, `consolidation`

## Summary

This issue consolidates several legacy GUI integration tasks into a single, actionable roadmap. The goal is to provide a clear path for integrating the `coldvox-gui` crate with the core application logic, covering window activation, focus handling, event routing, and platform-specific adapters.

## Milestones

### M1: Initial Window & State Management
- [ ] **Task:** Establish a basic window provider that can launch and manage the main application window.
- [ ] **Task:** Implement state synchronization between the core app state and the GUI.
- [ ] **Acceptance:** The GUI window launches and reflects the application's "listening" or "idle" state.

### M2: Focus & Activation Handling
- [ ] **Task:** Implement robust focus handling to detect the active application for text injection.
- [ ] **Task:** Develop platform-specific activation logic (e.g., using `xdotool` or `at-spi`) to bring the target window to the foreground.
- [ ] **Acceptance:** The system can reliably identify and focus the target application before injecting text.

### M3: Event Routing & Cross-platform Adapters
- [ ] **Task:** Create a unified event bus for communication between the GUI and core services (VAD, STT, etc.).
- [ ] **Task:** Develop platform-specific adapters for Wayland and X11 to handle desktop environment differences.
- [ ] **Acceptance:** GUI-initiated actions (e.g., "start listening") are correctly routed, and the system functions across different desktop environments.

### M4: Full Integration & Testing
- [ ] **Task:** Integrate all components into a seamless user experience.
- [ ] **Task:** Develop end-to-end integration tests for the complete GUI workflow.
- [ ] **Acceptance:** All GUI features are fully functional and tested.
