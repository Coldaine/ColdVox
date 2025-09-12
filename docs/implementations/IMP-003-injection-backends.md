---
id: IMP-003
title: Injection Backends Implementation
level: implementation
status: drafting
owners:
  - CDIS
criticality: 3
parent: SYS-003
pillar_trace:
  - PIL-003
  - DOM-003
  - SUB-003
  - SYS-003
implements:
  - "SPEC-003"
---

# Injection Backends Implementation [IMP-003]

## 1. Overview

This document describes the implementation of the various text injection backends, as defined in [SPEC-003](SPEC-003-injection-api.md). The core logic is located in the `coldvox-text-injection` crate, with each backend in its own module.

## 2. Code-level Traceability

This implementation maps to the following source code files:

-   **Strategy Manager**: `CODE:repo://crates/coldvox-text-injection/src/manager.rs#symbol=StrategyManager`
-   **AT-SPI Backend**: `CODE:repo://crates/coldvox-text-injection/src/atspi_injector.rs`
-   **Clipboard Backend**: `CODE:repo://crates/coldvox-text-injection/src/clipboard_injector.rs`
-   **YDotool Backend**: `CODE:repo://crates/coldvox-text-injection/src/ydotool_injector.rs`
-   **Enigo Backend**: `CODE:repo://crates/coldvox-text-injection/src/enigo_injector.rs`
-   **Combo Backends**: `CODE:repo://crates/coldvox-text-injection/src/combo_clip_ydotool.rs`

## 3. Key Components

### `StrategyManager`

This struct is the central orchestrator. It holds a vector of `Box<dyn Injector>` and uses the `FocusProvider` to make decisions.

### Injector Implementations

Each backend is a struct that implements the `Injector` trait. For example:

```rust
// From: crates/coldvox-text-injection/src/clipboard_injector.rs
pub struct ClipboardInjector;

impl Injector for ClipboardInjector {
    fn inject(&self, text: &str) -> Result<(), InjectionError> {
        // ... implementation to set clipboard and simulate paste ...
    }
}
```

This pattern is repeated for all backends, allowing them to be treated polymorphically by the `StrategyManager`.

## 4. Dependencies

-   `atspi`: For the AT-SPI accessibility backend.
-   `wl-clipboard-rs`: For the Wayland clipboard backend.
-   `ydotool`: For the ydotool keyboard emulation backend.
-   `enigo`: For the cross-platform keyboard emulation backend.
-   `log`: For logging injection attempts and strategy decisions.
