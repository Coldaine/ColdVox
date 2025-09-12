---
id: SPEC-003
title: Injection API Specification
level: specification
status: drafting
owners:
  - CDIS
criticality: 4
parent: SYS-003
pillar_trace:
  - PIL-003
  - DOM-003
  - SUB-003
  - SYS-003
  - SPEC-003
implements:
  - "IMP-003"
verified_by:
  - "TST-003"
---

# Injection API Specification [SPEC-003]

## 1. Overview

This specification defines the public API for all text injection backends in ColdVox. The contract is primarily defined by the `Injector` trait, which ensures that all injection methods can be managed and executed by the `StrategyManager`.

## 2. Core Trait: `Injector`

Any component that provides text injection services MUST implement this trait.

**Public API:**
```rust
// Simplified for specification
pub trait Injector {
    /// Returns the name of the injector.
    fn name(&self) -> &str;

    /// Injects the given text.
    fn inject(&self, text: &str) -> Result<(), InjectionError>;
}
```

## 3. Core Component: `StrategyManager`

The `StrategyManager` is the primary entry point for the text injection system.

**Public API:**
```rust
// Simplified for specification
pub struct StrategyManager;

impl StrategyManager {
    /// Creates a new manager with a set of available injectors.
    pub fn new(injectors: Vec<Box<dyn Injector>>, focus_provider: Box<dyn FocusProvider>) -> Self;

    /// Processes a final transcription, selecting and executing the best injection strategy.
    pub fn process_final_transcription(&self, event: &FinalTranscription) -> Result<(), InjectionError>;
}
```

## 4. Configuration

The behavior of the injection system is controlled by a configuration struct populated from CLI arguments.

**Key Fields:**
- `allow_kdotool`: `bool`
- `allow_enigo`: `bool`
- `restore_clipboard`: `bool`
- `max_total_latency_ms`: `u64`

## 5. Data Flow

1.  The `StrategyManager` is initialized with a collection of available `Injector` backends.
2.  When a final transcription is received, the manager calls the `FocusProvider` to identify the active application.
3.  Based on application-specific rules and available backends, the manager selects a prioritized list of injectors to try.
4.  It attempts to call `inject()` on the first injector in the list.
5.  If the injection fails or times out, it proceeds to the next injector in the fallback chain until one succeeds or all have failed.
