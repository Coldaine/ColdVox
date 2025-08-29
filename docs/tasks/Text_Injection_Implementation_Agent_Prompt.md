# Agent Prompt: Implement KDE/Wayland Text Injection (Phase 2+)

## Mission
Implement a modular, adaptive text-injection subsystem for KDE Plasma (Wayland) using safe, KDE-friendly methods first, with gated fallbacks. Wire it into `crates/app` under `src/text_injection/` and integrate a Strategy Manager that selects the best working method per target app and caches success.

## Scope (Phase 2+ now, Phase 3 optional)
- Core injectors (enable by default unless disabled via config):
  - AT-SPI2 EditableText insert (context aware when available)
  - Clipboard set via Wayland-native API
  - Clipboard+AT-SPI Paste (set clipboard then Action::Paste on focused control)
- Optional fallbacks (explicitly gated via config flags):
  - ydotool (external uinput binary) for Ctrl+V synthetic paste
  - kdotool (external CLI) for KDE window activation/focus assistance only
  - enigo (library; experimental Wayland/libei paths) for synthetic text/paste
  - mouse-keyboard-input (uinput) for synthetic keys (last-resort)
- Phase 3 (not required now):
  - IME composition (text-input v3 + input-method v2)
  - Portals/libei (ashpd + reis) for user-consented input injection when available

## Deliverables
- New module files in `crates/app/src/text_injection/`:
  - `types.rs` — InjectionConfig, InjectionMethod enum, Result types, metrics spans
  - `focus.rs` — Focus tracker (AT-SPI2); helpers to resolve focused, editable objects
  - `manager.rs` — Strategy Manager (method ordering, per-app success cache, cooldown/backoff, timeouts)
  - `atspi_injector.rs` — Insert text via AT-SPI2 EditableText and/or Paste action
  - `clipboard_injector.rs` — Set Wayland clipboard (wl-clipboard-rs)
  - `combo_clip_atspi.rs` — Set clipboard then AT-SPI Paste
  - `ydotool_injector.rs` — Spawn `ydotool` for keystrokes (opt-in)
  - `kdotool_injector.rs` — Spawn `kdotool` for window activation/focus help (opt-in)
  - `enigo_injector.rs` — Enigo-based text/paste (opt-in; feature flags)
  - `mki_injector.rs` — mouse-keyboard-input uinput path (opt-in; last-resort)
- Wire config:
  - Add `InjectionConfig` with booleans: allow_ydotool, allow_kdotool, allow_enigo, allow_mki, restore_clipboard, inject_on_unknown_focus; and duration fields for per-method and global timeouts/cooldowns.
  - Expose CLI/env toggles to enable/disable optional methods.
- Telemetry & logs:
  - Method attempts, success/failure, elapsed, per-app success cache hits, cooldown/backoff entries.
- Minimal tests/examples:
  - Unit tests for Clipboard and Strategy Manager ordering/cooldown.
  - An example binary under `crates/app/examples/inject_demo.rs` that tries methods and prints outcome.

## Crates to use (vetted)
- AT-SPI2 & D-Bus:
  - atspi = { version = "^0.25", features = ["tokio"] }
  - zbus = { version = "^3", default-features = false, features = ["tokio"] }
- Wayland clipboard (no window required):
  - wl-clipboard-rs = "^0.9"
- Portals & libei (Phase 3; keep optional):
  - ashpd = { version = "^0.9", features = ["tokio", "wayland"], optional = true }
  - reis = { version = "^0.4", features = ["tokio"], optional = true }
- Synthetic input (opt-in fallbacks):
  - enigo = { version = "^0.2", default-features = false, features = ["wayland", "libei_tokio"], optional = true }
  - mouse-keyboard-input = { version = "^0.9", optional = true }
- Wayland client plumbing for future IME (Phase 3; optional):
  - wayland-client, wayland-protocols, wayland-protocols-wlr, smithay-client-toolkit (optional)

Notes:
- `kdotool` is an app crate (CLI), not a library. Call it via `tokio::process::Command` if enabled.
- `ydotool` is an external binary that requires uinput permissions. Treat as off-by-default.

## Implementation outline
1) Types & config
- Define `InjectionMethod` enum with: AtspiInsert, Clipboard, ClipboardAndPaste, YdoToolPaste, KdoToolAssist, EnigoText, UinputKeys.
- `InjectionConfig` with timeouts:
  - per_method_timeout_ms (default 250), paste_action_timeout_ms (200), discovery_timeout_ms (200), global_budget_ms (700)
  - cooldown_ms_per_app_method (default 60000), backoff multiplier 2x on repeated failures

2) Clipboard injector
- Use `wl_clipboard_rs::copy::{Options, Source, MimeType}` to set UTF-8 text.
- If `restore_clipboard`, capture current selection and restore after injection completes.

3) AT-SPI focus & insert
- Maintain an async AT-SPI connection.
- Resolve focused application and accessible object; check for EditableText capability.
- Prefer `EditableText.insert_text(offset=caret_position_or_end)`. Fallback to `Action::Paste` if insert is unsupported.
- Honor per-method and global timeouts.

4) Combo Clipboard+AT-SPI Paste
- Set clipboard, then trigger Paste action via AT-SPI on focused widget.
- Ensure the target supports Action::Paste; otherwise skip with a typed error.

5) Optional fallbacks (guarded)
- ydotool: spawn `ydotool key ctrl+v` with a short timeout; require capability probe at startup.
- kdotool: only use to bring a target window to front when AT-SPI can identify a candidate but activation fails; do not send keys via kdotool (window control only).
- enigo: prefer enigo with `wayland`/`libei_tokio` features for text/paste; probe availability and gracefully skip if backend not working.
- mouse-keyboard-input: create a virtual keyboard device and emit key events for Ctrl+V; only if user enabled and permissions exist; short-circuit if not in input group.

6) Strategy Manager
- Order (KDE-first, conservative): AtspiInsert → ClipboardAndPaste → Clipboard-only → (if enabled) KdoToolAssist → EnigoText → UinputKeys → YdoToolPaste.
- Cache the last successful method per app-id; try that first on subsequent injections; record failures and apply cooldown/backoff.
- Enforce global budget; stop as soon as one method succeeds.

7) Tests & example
- Unit test: Clipboard injector set/restore roundtrip (skip if compositor lacks data-control; mark ignored on CI if needed).
- Unit test: Strategy ordering + per-app cache + cooldown behavior.
- Example: `inject_demo.rs` printing success/failure per enabled method for a sample string.

## Acceptance criteria
- Build compiles with default features on Linux (Wayland session).
- Clipboard, AT-SPI insert, and Combo injectors pass basic tests locally.
- Strategy Manager enforces ordering, caches per-app successes, and applies cooldowns.
- Optional fallbacks are gated, probed at startup, and skipped cleanly when unavailable.
- Logs and metrics show attempt counts, success/failure, and durations.

## References (crates)
- atspi — Pure-Rust AT-SPI2 (zbus-based)
- zbus — D-Bus
- wl-clipboard-rs — Wayland clipboard without a window
- ashpd — XDG Portals (optional)
- reis — libei/eis protocol (optional)
- enigo — cross-platform input simulation (Wayland/libei features optional)
- mouse-keyboard-input — uinput wrapper
- kdotool — KDE Wayland xdotool-like (CLI; use via Command)
