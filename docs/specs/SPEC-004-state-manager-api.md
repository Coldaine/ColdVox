---
id: SPEC-004
title: State Manager API Specification
level: specification
status: drafting
owners:
  - CDIS
criticality: 5
parent: SYS-004
pillar_trace:
  - PIL-004
  - DOM-004
  - SUB-004
  - SYS-004
  - SPEC-004
implements:
  - "IMP-004"
verified_by:
  - "TST-004"
---

# State Manager API Specification [SPEC-004]

## 1. Overview

This specification defines the public API for the State Manager Service. The contract ensures that application state is managed in a centralized, predictable, and safe manner.

## 2. Core Enum: `AppState`

This enum defines the complete set of possible application states.

**Variants:**
```rust
// Simplified for specification from coldvox-foundation/src/state.rs
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AppState {
    Initializing,
    Running,
    Paused,
    ShuttingDown,
    Stopped,
}
```

## 3. Core Struct: `StateManager`

This struct provides the primary interface for interacting with the application state.

**Public API:**
```rust
// Simplified for specification
pub struct StateManager;

impl StateManager {
    /// Creates a new `StateManager`, initialized to the `Initializing` state.
    pub fn new() -> Self;

    /// Returns the current application state.
    pub fn get_state(&self) -> AppState;

    /// Attempts to transition the application to a new state.
    /// Returns an error if the transition is not valid.
    pub fn set_state(&mut self, new_state: AppState) -> Result<(), StateError>;
}
```

## 4. State Transition Rules

The `set_state` method MUST enforce the following state transition graph. Any other attempted transition MUST fail.

-   `Initializing` -> `Running`
-   `Running` -> `Paused`
-   `Running` -> `ShuttingDown`
-   `Paused` -> `Running`
-   `Paused` -> `ShuttingDown`
-   `ShuttingDown` -> `Stopped`

The `Stopped` state is terminal and cannot be transitioned out of.
