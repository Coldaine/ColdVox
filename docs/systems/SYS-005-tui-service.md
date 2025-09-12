---
id: SYS-005
title: TUI Service
level: system
status: drafting
owners:
  - CDIS
criticality: 3
parent: SUB-005
pillar_trace:
  - PIL-005
  - DOM-005
  - SUB-005
  - SYS-005
---

# TUI Service [SYS-005]

The TUI Service is the technical component responsible for rendering and managing the interactive terminal dashboard. It is a separate binary within the `coldvox-app` crate that shares the core runtime with the main application.

This system is primarily implemented using the `ratatui` crate and its ecosystem.

Key components:
- **Main TUI Loop**: An event loop that handles user input (keyboard events) and redraws the terminal UI on each "tick".
- **UI Layout Manager**: Code that defines the layout of the dashboard, dividing the screen into different panels and sections.
- **Stateful Widgets**: Widgets that hold their own state, such as tables, lists, and charts, to display application data.
- **Data Receivers**: The TUI runs in its own thread and receives application state and events from the core services via channels.
