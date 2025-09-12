---
id: IMP-004
title: State Manager Implementation
level: implementation
status: drafting
owners:
  - CDIS
criticality: 4
parent: SYS-004
pillar_trace:
  - PIL-004
  - DOM-004
  - SUB-004
  - SYS-004
implements:
  - "SPEC-004"
---

# State Manager Implementation [IMP-004]

## 1. Overview

This document describes the implementation of the State Manager Service, as defined in [SPEC-004](SPEC-004-state-manager-api.md). The core logic is located in the `coldvox-foundation` crate.

## 2. Code-level Traceability

This implementation directly maps to the following source code files and symbols:

-   **Primary State Logic**: `CODE:repo://crates/coldvox-foundation/src/state.rs#symbol=StateManager`
-   **State Enum Definition**: `CODE:repo://crates/coldvox-foundation/src/state.rs#symbol=AppState`

## 3. Key Components

### `StateManager`

This struct holds the current `AppState` within a `Mutex` to ensure thread-safe access.

**Key functions:**
- `set_state()`: This method contains the core state transition logic. It checks the current state against the requested new state and only permits valid transitions, returning an error for invalid ones.

```rust
// From: crates/coldvox-foundation/src/state.rs
pub struct StateManager {
    current_state: Mutex<AppState>,
}

impl StateManager {
    pub fn set_state(&self, new_state: AppState) -> Result<(), StateError> {
        let mut current_state = self.current_state.lock().unwrap();
        // ... validation logic based on current_state and new_state ...
        *current_state = new_state;
        Ok(())
    }
}
```

## 4. Dependencies

-   `parking_lot`: Provides an efficient `Mutex` for wrapping the `AppState`.
-   `thiserror`: Used to define the `StateError` type for invalid transitions.
-   `log`: For logging state transitions.
