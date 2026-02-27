---
doc_type: reference
subsystem: text-injection
status: draft
freshness: stale
preservation: preserve
description: Overview and usage guide for the ColdVox text-injection system.
domain_code: ti
id: text-injection-overview
last_reviewed: 2025-11-06
owners: Text Injection Maintainers
title: Text Injection Overview
version: 1.0.0
---

# coldvox-text-injection

Automated text injection system for ColdVox transcribed speech.

## What's New (workspace v2.0.1)

- FocusProvider DI: inject focus detection for deterministic and safe tests
- Combo clipboard+paste injector (`combo_clip_ydotool`) with async `ydotool` check
- Real injection testing with lightweight test applications for comprehensive validation
- Full desktop CI support with real audio devices and desktop environments available
- Allow/block list semantics: compiled regex path when `regex` is enabled; substring matching otherwise

## Purpose

This crate provides text injection capabilities that automatically type transcribed speech into applications:

- Multi-Backend Support: Multiple text injection methods for different environments
- Focus Tracking: Automatic detection of active application windows
- Smart Routing: Application-specific injection method selection
- Cross-Platform: Support for X11, Wayland, and other desktop environments

## Key Components

### Text Injection Backends
- AT-SPI: Accessibility API for direct text insertion (preferred on Linux when available)
- Unified Clipboard: Seed clipboard, then paste via Enigo (if enabled) or `ydotool` fallback
- YDotool: uinput-based key events (opt-in, primarily Wayland environments)
- KDotool Assist: KDE/X11 window activation assistance (opt-in)
- Enigo: Cross-platform key simulation used by the Unified Clipboard paste path (opt-in)

### Focus Detection
- Active window detection and application identification
- Application-specific method prioritization
- Unknown application fallback strategies

### Smart Injection Management
- Latency optimization and timeout handling
- Method fallback chains for reliability
- Configurable injection strategies per application

## Features

- `default`: Core text injection functionality with safe defaults
- `atspi`: Linux AT-SPI accessibility backend
- `wl_clipboard`: Clipboard-based injection via wl-clipboard-rs
- `enigo`: Cross-platform input simulation
- `ydotool`: Linux uinput automation
- `kdotool` / `xdg_kdotool`: KDE/X11 window activation assistance (alias supported)
- `regex`: Compiled allow/block list patterns (regex)
- `all-backends`: Enable all available backends
- `linux-desktop`: Enable recommended Linux desktop backends

## Strategy and Selection Order

The orchestrator uses a fast-fail loop with tight budgets to try methods in order. Defaults:

1. AT-SPI Insert (preferred for reliability, accessibility, and content fidelity)
2. Unified Clipboard Paste (clipboard seed + paste via Enigo or `ydotool`)

Notes:
- There is no AT-SPI "paste" fallback path. If AT-SPI direct insert cannot target the widget, the orchestrator falls back to the clipboard-based injector.
- On Windows/macOS, only the Unified Clipboard path is attempted.
- KDotool is an assistance mechanism for focus/activation; it is not an injection method by itself.

## Configuration

### CLI Options

- `--allow-kdotool`: Enable KDE-specific tools
- `--allow-enigo`: Enable Enigo input simulation
- `--restore-clipboard`: Restore clipboard contents after injection
- `--inject-on-unknown-focus`: Inject even when focus detection fails

### Timing Controls
- `--max-total-latency-ms`: Maximum time allowed for injection
- `--per-method-timeout-ms`: Timeout per backend attempt
- `--cooldown-initial-ms`: Delay before first injection attempt

## System Requirements

### Linux
```bash
# For AT-SPI support
sudo apt install libatk-bridge2.0-dev

# For X11 helpers
sudo apt install libxtst-dev wmctrl

# For clipboard functionality
sudo apt install xclip wl-clipboard

# For ydotool-based paste (optional)
sudo apt install ydotool
```

## Error Types

The text injection system uses `InjectionError` (defined in `coldvox-foundation`) with the following variants:

| Variant | Meaning |
|---------|---------|
| `NoEditableFocus` | No editable text field has focus in the target application. |
| `MethodNotAvailable(String)` | A specific injection method is not available in this environment. |
| `Timeout(u64)` | An operation exceeded its timeout budget (value in ms). |
| `AllMethodsFailed(String)` | Every method in the fallback chain failed. |
| `MethodUnavailable(String)` | Requested method is unavailable (similar to MethodNotAvailable, used in different contexts). |
| `MethodFailed(String)` | A specific method attempted injection and failed. |
| `BudgetExhausted` | The total latency budget for the injection attempt was exceeded before all methods could be tried. |
| `Clipboard(String)` | Clipboard read, write, or restore operation failed. |
| `Process(String)` | An external process (e.g., ydotool, xclip) failed. |
| `PermissionDenied(String)` | Insufficient permissions for the requested operation (e.g., /dev/uinput access). |
| `Io(std::io::Error)` | Underlying I/O error. |
| `Other(String)` | Catch-all for errors that don't fit other variants. |

## Allow/Block List Semantics

The `InjectionConfig` includes `allowlist` and `blocklist` vectors of patterns matched against the window class reported by the window manager.

**Evaluation order (from `StrategyManager::is_app_allowed`):**

1. If the **allowlist is non-empty**, only applications matching at least one allowlist pattern are eligible. The blocklist is not consulted.
2. If the **allowlist is empty** and the **blocklist is non-empty**, applications matching any blocklist pattern are rejected.
3. If **both lists are empty**, all applications are eligible.

**Pattern matching:**
- When the `regex` feature is enabled, patterns are compiled as `Regex` objects at construction time.
- Without the `regex` feature, patterns use substring matching (with leading `^` and trailing `$` anchors stripped).
- Invalid regex patterns are logged as warnings and skipped rather than crashing the runtime.

## Security Considerations

Text injection requires various system permissions:
- X11: Access to X server for input simulation
- Wayland: May require special permissions for input
- AT-SPI: Accessibility service access
- Clipboard: Read/write access to system clipboard

## Usage

Enable through the main ColdVox application:

```bash
# Basic text injection
cargo run --features text-injection

# With specific backends
cargo run --features text-injection -- --allow-ydotool --restore-clipboard
```

### Diagnostics

Run the `injection_diagnostics` example to inspect the computed fallback chain and execute a real injection:

```bash
cargo run -p coldvox-text-injection --example injection_diagnostics \
    -- --config ../../config/default.toml --text "diagnostic ping" --no-redact
```

Omit `--no-redact` to keep text content hashed in logs.

## Dependencies

- Backend-specific libraries (optional based on features)
- Platform integration libraries for focus detection
- Async runtime support for timeout handling

## Testing
## Rationale for Approaches

- AT-SPI direct insert: Best fidelity and least disruptive when supported by the focused control; avoids polluting clipboard and is accessible-first.
- Unified Clipboard: Broad compatibility via clipboard seeding plus paste action; uses Enigo or `ydotool` for the paste trigger depending on features and OS.
- No AT-SPI paste: When AT-SPI cannot address the control for direct insertion, invoking an AT-SPI "paste" action does not help and adds overhead. Therefore, the clipboard injector does not attempt AT-SPI operations.

## Session State Machine

The `InjectionSession` manages buffering of transcriptions and determines injection timing through a four-state machine:

- **Idle**: No active session; waiting for the first transcription event.
- **Buffering**: Actively receiving transcriptions. Text is accumulated in a buffer with whitespace normalization.
- **WaitingForSilence**: No new transcriptions have arrived within the buffer pause timeout. The system waits for the silence timeout before injecting.
- **ReadyToInject**: Silence timeout reached (or buffer size limit hit, or sentence-ending punctuation detected). The buffered text is handed to the strategy manager for injection.

Transitions:
- `Idle` -> `Buffering`: First `TranscriptionEvent::Final` received.
- `Buffering` -> `WaitingForSilence`: `buffer_pause_timeout_ms` elapsed since last transcription.
- `WaitingForSilence` -> `Buffering`: New transcription arrives (resets the timer).
- `WaitingForSilence` -> `ReadyToInject`: `silence_timeout_ms` elapsed with no new input.
- `Buffering` -> `ReadyToInject`: Buffer exceeds `max_buffer_size` chars, or sentence-ending punctuation detected.
- `ReadyToInject` -> `Idle`: Buffer consumed by the injector.

Default timing values (`silence_timeout_ms = 0`, `buffer_pause_timeout_ms = 0`) cause immediate injection after each final transcription, which is appropriate when the STT engine handles its own audio buffering.

## Focus Detection

The `FocusTracker` gates injection attempts based on the focused element in the target application.

### Focus Status Values

- **EditableText**: Focus is in a text input field that supports editing. Injection proceeds normally.
- **NonEditable**: Focus is on a non-editable element (button, label, etc.). Injection is skipped unless overridden.
- **Unknown**: Focus status cannot be determined (accessibility bus unavailable or timed out). Behavior depends on `inject_on_unknown_focus` config.

### Caching and Backends

The tracker caches the last focus query result for `focus_cache_duration_ms` (default 200ms) to avoid flooding the accessibility bus. When the cache expires, the backend is queried again.

The `FocusBackend` trait allows pluggable implementations. The default `SystemFocusAdapter` queries the AT-SPI bus and gracefully falls back to `Unknown` when the bus is unavailable. Custom backends can be injected for deterministic testing.

### Relevant Config Options

| Option | Default | Description |
|--------|---------|-------------|
| `inject_on_unknown_focus` | `true` | Allow injection when focus status is Unknown |
| `require_focus` | `false` | Require EditableText focus before injecting |
| `focus_cache_duration_ms` | `200` | How long to cache focus status (ms) |

## Confirmation and Prewarming

### Confirmation

After a successful method returns, the orchestrator runs a quick confirmation check using AT-SPI text-changed events. The confirmation system:

- Subscribes to `object:text-changed:inserted` events on the targeted accessible element.
- Performs **prefix matching** using the first 3-6 Unicode grapheme clusters of the injected text. This avoids false positives from IME composition or grapheme normalization differences.
- Uses a **75ms timeout** with 10ms polling intervals (up to 7 polls).
- Returns `ConfirmationResult::Success`, `Timeout`, `Mismatch`, or `Error`.

Non-success does not immediately fail the injection; the next strategy may be attempted.

### Prewarming

On entering `Buffering` state, the orchestrator triggers targeted prewarming for the first method in the current strategy order to reduce first-use latency (e.g., establishing AT-SPI context). Pre-warmed data is cached with a ~3 second TTL that refreshes on new buffer activity.

All tests use real desktop applications and injection backends with full desktop environments available in all environments:

```bash
# Run crate tests with real injection validation
cargo test -p coldvox-text-injection --locked

# No-default-features with real hardware
cargo test -p coldvox-text-injection --no-default-features --locked

# Regex feature with real injection testing
cargo test -p coldvox-text-injection --no-default-features --features regex --locked
```

See `docs/domains/text-injection/ti-testing.md` for details on live/CI testing and feature matrices.
