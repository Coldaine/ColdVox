---
id: TST-005
title: TUI Tests
level: test
status: drafting
owners:
  - CDIS
criticality: 2
parent: null
pillar_trace:
  - PIL-005
  - DOM-005
  - SUB-005
  - SYS-005
verifies:
  - "SPEC-005"
---

# TUI Tests [TST-005]

## 1. Overview

This document describes the testing strategy for the TUI Dashboard, which aims to verify the functionality defined in [SPEC-005](SPEC-005-tui-controls.md).

## 2. Code-level Traceability

*(Conceptual: There are currently no automated tests for the TUI Dashboard binary.)*

## 3. Test Cases

Automated testing of terminal user interfaces is challenging and often has a low return on investment. As such, there are no automated test cases for the TUI at this time.

Verification of the TUI is performed through manual testing. Key manual test scenarios include:

-   **Layout Verification**: Launching the TUI and visually inspecting that all panels are rendered correctly and in the correct positions.
-   **Control Verification**: Pressing each of the documented keyboard controls (`S`, `A`, `R`, `Q`) and verifying that the application responds as expected.
-   **Data Display Verification**: Running the application with a live microphone and observing that the VAD Status, STT Events, and Log panels update correctly in real-time.
-   **Resize Handling**: Resizing the terminal window and verifying that the TUI layout adapts gracefully without crashing or corrupting the display.

## 4. Test Strategy

-   **Manual Testing**: The primary testing strategy for the TUI is manual, exploratory testing. Developers and QA testers run the TUI dashboard and interact with it to identify bugs or visual glitches.
-   **Future Improvements (Conceptual)**: It may be possible to implement snapshot testing for the TUI. This would involve capturing the terminal's buffer state as a string and comparing it against a known-good "snapshot". This would catch regressions in the layout but would not test interactivity.
