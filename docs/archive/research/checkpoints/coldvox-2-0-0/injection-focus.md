---
doc_type: research
subsystem: text-injection
status: draft
freshness: historical
preservation: delete
last_reviewed: 2025-10-19
owners: Documentation Working Group
version: 1.0.0
---

# Focus Detection Pipeline

ColdVox relies on a dedicated focus tracker to gate injection attempts. The tracker now ships with a pluggable backend that
mirrors production behaviour while remaining testable in headless CI runs.

## System flow

```mermaid
flowchart TD
    A[StrategyManager] --> B[FocusTracker]
    B --> C{Cache fresh?}
    C -- Yes --> D[Use cached status]
    C -- No --> E[Focus backend]
    E -->|Success| F[EditableText/NonEditable]
    E -->|Error| G[Unknown]
    F --> H[Injection decision]
    G --> H
```

* **Caching** – the tracker stores the last decision for the configured window (default `200 ms`) to avoid hammering
  the accessibility bus. When the cache expires, the backend is queried again.
* **Backends** – the default backend gracefully falls back to `Unknown` when the bus is unavailable or times out. Tests can inject custom backends for deterministic scenarios.
* **Async-safe** – all operations are wrapped in timeout guards to prevent hangs.

## Focus Status Values

- `EditableText`: Focus is in a text input field that supports editing
- `NonEditable`: Focus is in a non-editable element (buttons, labels, etc.)
- `Unknown`: Focus status cannot be determined (fallback behavior)

## Configuration

Focus detection behavior is controlled by these config options:

```toml
[injection]
inject_on_unknown_focus = true   # Allow injection when focus is unknown
require_focus = false           # Require editable focus for injection
focus_cache_duration_ms = 200   # Cache duration for focus status (ms)
```

## Backend Architecture

The `FocusBackend` trait allows different implementations:

```rust
#[async_trait]
pub trait FocusBackend: Send + Sync {
    async fn query_focus(&self) -> Result<FocusStatus, InjectionError>;
}
```

This enables:
- Alternative focus detection methods
- Mock backends for testing
- Platform-specific implementations</content>

## Aspirational interaction modes

Long-term we envision an "always listening" option as an aspirational mode where the system monitors audio continuously but does not perform injection until explicitly triggered. The recommended and default interaction model remains Push-to-Talk (manual hotkey activation) which minimizes accidental injection and preserves user privacy. A lightweight "Speak-to-Talk" toggle (single-button in the UI) can be offered as an opt-in mode: when enabled it behaves like a momentary push-to-talk but presented as a persistent toggle for convenience.

- Default: Push-to-Talk (hotkey) — recommended for privacy and predictability.
- Aspirational: Always listening with explicit trigger to begin injection workflow.
- Optional UI: Single-button "Speak-to-Talk" toggle to opt into a momentary activation mode without requiring a hotkey.

These modes should be documented in architecture and UX design materials and only enabled after clear user consent and UI affordances.
<parameter name="filePath">/home/coldaine/Desktop/ColdVoxRefactorTwo/ColdVox/docs/architecture/injection-focus.md