# EPIC-2: Platform-Specific Injectors

## Description

This epic focuses on implementing the platform-specific text injection methods for each target environment: KDE Plasma, Hyprland, and Windows. Each implementation must adhere to the core design principles of the injection system, including fast-fail stages, event-based success confirmation, and structured logging.

The detailed implementation plans and code snippets for these injectors are specified in `docs/plans/MasterPlan.md` and `docs/plans/OpusCodeInject.md`.

## Acceptance Criteria

- A reliable and performant injection method is implemented for each supported platform.
- Each injector integrates seamlessly with the core `TextInjector` and its fallback logic.
- All platform-specific code is well-documented and covered by integration tests.
- Each implementation follows the specified method ranking for its environment (e.g., AT-SPI first on Linux).
- All necessary D-Bus, Wayland, or Windows API interactions are handled correctly and efficiently.

## Sub-Tasks

### KDE Plasma (KWin, Wayland)
- [ ] **FEAT-201:** Implement AT-SPI `insert` and `paste` methods.
  - *Labels:* `feature`, `platform:kde`, `at-spi`
- [ ] **FEAT-202:** Implement KWin Fake Input injector as a privileged, feature-flagged option.
  - *Labels:* `feature`, `platform:kde`, `kwin`
- [ ] **TEST-203:** Write integration tests for the KWin Fake Input injector.
  - *Labels:* `testing`, `platform:kde`

### Hyprland (wlroots)
- [ ] **FEAT-204:** Implement the `wlr_virtual_keyboard_v1` injector for Hyprland.
  - *Labels:* `feature`, `platform:hyprland`, `wayland`
- [ ] **TEST-205:** Write integration tests for the Hyprland virtual keyboard injector.
  - *Labels:* `testing`, `platform:hyprland`

### Cross-Platform (Wayland)
- [ ] **FEAT-206:** Implement the Portal/EIS injector for secure, authorized input.
  - *Labels:* `feature`, `platform:kde`, `platform:hyprland`, `portal`
- [ ] **TEST-207:** Write contract tests for Portal/EIS protocol compliance.
  - *Labels:* `testing`, `portal`

### Windows
- [ ] **FEAT-208:** Implement the UI Automation (UIA) injector for Windows.
  - *Labels:* `feature`, `platform:windows`, `uia`
- [ ] **FEAT-209:** Implement the `SendInput` (typing and Ctrl+V) injector as a fallback for Windows.
  - *Labels:* `feature`, `platform:windows`
- [ ] **TEST-210:** Write integration tests for both Windows injection methods.
  - *Labels:* `testing`, `platform:windows`

### General
- [ ] **DOCS-211:** Document the setup and permissions required for each injection method on each platform.
  - *Labels:* `documentation`