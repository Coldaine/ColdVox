---
id: SPEC-005
title: TUI Controls Specification
level: specification
status: drafting
owners:
  - CDIS
criticality: 3
parent: SYS-005
pillar_trace:
  - PIL-005
  - DOM-005
  - SUB-005
  - SYS-005
  - SPEC-005
implements:
  - "IMP-005"
verified_by:
  - "TST-005"
---

# TUI Controls Specification [SPEC-005]

## 1. Overview

This specification defines the layout, controls, and displayed information for the TUI (Text-based User Interface) Dashboard.

## 2. Layout

The TUI screen is divided into several panels:

-   **VAD Status Panel**: Displays the current state of the Voice Activity Detector (e.g., `Listening`, `Speech`, `Silence`).
-   **STT Events Panel**: Shows a log of the latest transcription events (e.g., `Partial result`, `Final result`).
-   **System Metrics Panel**: Displays real-time performance metrics, such as CPU usage and FPS of the TUI itself.
-   **Log Panel**: A scrolling view of the application's detailed log output.
-   **Help/Controls Footer**: A static footer displaying the available keyboard controls.

## 3. Keyboard Controls

The following keyboard shortcuts MUST be implemented:

| Key | Action                          |
|-----|---------------------------------|
| `S` | Start the audio pipeline        |
| `A` | Toggle between VAD and PTT mode |
| `R` | Reset the pipeline              |
| `Q` | Quit the application            |

## 4. Data Display

-   **VAD State**: Must update in real-time as the VAD engine emits events.
-   **STT Text**: Final transcriptions should be clearly distinguished from partial results.
-   **Logs**: Log messages should be timestamped and color-coded based on their severity level (e.g., INFO, WARN, ERROR).
