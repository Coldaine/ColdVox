---
id: IMP-005
title: TUI Dashboard Implementation
level: implementation
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
implements:
  - "SPEC-005"
---

# TUI Dashboard Implementation [IMP-005]

## 1. Overview

This document describes the implementation of the TUI Dashboard, as defined in [SPEC-005](SPEC-005-tui-controls.md). The core logic is a separate binary located in the `coldvox-app` crate.

## 2. Code-level Traceability

This implementation directly maps to the following source code file:

-   **Primary TUI Logic**: `CODE:repo://crates/app/src/bin/tui_dashboard.rs`

## 3. Key Components

### `main()` function

The entry point for the TUI binary. It is responsible for:
1.  Setting up the terminal for TUI rendering (`crossterm`).
2.  Initializing the core application runtime and services.
3.  Spawning the TUI event loop in a separate thread.
4.  Waiting for the TUI to exit and then cleaning up resources.

### `run_tui()` function

This function contains the main TUI loop.
- It handles drawing the UI widgets on each tick using the `ratatui` library.
- It polls for keyboard input events from the user.
- It receives application state updates from the core services via channels.

### UI Rendering

The UI is drawn using a series of functions that take a `Frame` and a state object, and render widgets like `Paragraph`, `Block`, and `Table` to build the dashboard layout.

## 4. Dependencies

-   `ratatui`: The core library for rendering the text-based user interface.
-   `crossterm`: Provides the low-level terminal manipulation capabilities (e.g., raw mode, clearing screen).
-   `tokio`: The TUI runs within the shared Tokio async runtime.
-   `log`: The TUI consumes log events to display in the log panel.
