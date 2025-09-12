---
id: SYS-003
title: Injection Manager System
level: system
status: drafting
owners:
  - CDIS
criticality: 4
parent: SUB-003
pillar_trace:
  - PIL-003
  - DOM-003
  - SUB-003
  - SYS-003
---

# Injection Manager System [SYS-003]

The Injection Manager System is the technical component responsible for selecting and executing text injection strategies. It acts as the central brain for the Output Integration pillar, orchestrating focus detection, backend selection, and fallback logic.

This system is primarily implemented by the `StrategyManager` and its associated components within the `coldvox-text-injection` crate.

Key components:
- **`StrategyManager`**: The core component that receives a transcription and decides which injection backend to use based on the currently focused application and the available backends.
- **`FocusProvider`**: A dependency-injected component that provides information about the active window, allowing for deterministic testing.
- **Backend Implementations**: A collection of structs that implement a common `Injector` trait, one for each method (e.g., `ClipboardInjector`, `AtspiInjector`).
- **Configuration**: A set of CLI options and timing controls that allow the user to customize injection behavior, such as enabling specific backends or setting timeouts.
