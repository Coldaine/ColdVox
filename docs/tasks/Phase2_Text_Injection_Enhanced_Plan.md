# Phase 2 — Enhanced Text Injection (KDE/Wayland, AT‑SPI2 + Clipboard)

This is a pragmatic Phase 2 plan for ColdVox’s session-based text injection on KDE Plasma/Wayland. Goal: add AT‑SPI2 batch injection with basic focus awareness, plus a Rust-native clipboard fallback. Keep scope light (personal project), with small, verifiable steps.

## Scope (what we’re adding now)
- AT‑SPI2 injection via the Odilia atspi crate (zbus 5 under the hood).
- Event-driven focus tracking (best-effort), used to decide when to inject.
- Clipboard fallback using a Rust crate (no external wl-copy dependency).
- Optional ydotool path kept as a manual, opt-in fallback (unchanged).
- Session-based buffering from Phase 1 remains; we only swap in better injectors.

Out of scope for Phase 2:
- IME integrations (IBus/Fcitx5), per-app profiles, ML timing.

## Repo fit
- Current app crate has no atspi/zbus/clipboard deps; we’ll add them behind a feature flag.
- Keep everything in a small module tree: `crates/app/src/text_injection/`.
- Don’t break existing binaries; injection remains optional.

## Dependencies (Rust-first)
Add to `crates/app/Cargo.toml` (feature-gated):
- atspi = { version = "0.28", features = ["connection", "proxies"], optional = true }
  - Brings zbus 5 transitively; no need to depend on zbus directly unless desired.
- wl-clipboard-rs = { version = "0.9", optional = true }
  - Wayland-native clipboard for headless/CLI apps; good fit for KWin/Plasma.
- anyhow, tracing already present.

Optional (if you prefer the simpler API):
- arboard = { version = "3.6", default-features = false, features = ["wayland-data-control"], optional = true }

Feature flags:
- text-injection (enables Phase 2 injection path)
- text-injection-atspi (enables atspi usage)
- text-injection-clipboard (enables clipboard fallback)

Minimal default: keep features off unless building demos/tests.

## Module layout (new files)
- `src/text_injection/mod.rs`
  - `pub trait TextInjector { fn name(&self) -> &'static str; fn inject(&self, text: &str) -> anyhow::Result<()>; fn is_available(&self) -> bool; fn supports_batch(&self) -> bool { true } }`
  - `InjectionManager` holds an ordered list of injectors and a simple `try_inject(text)`.
- `src/text_injection/session.rs`
  - Reuse/port Phase 1 session logic (buffer, silence timeout, take_buffer()).
- `src/text_injection/focus.rs` (feature = text-injection-atspi)
  - Event-driven focus tracker using atspi; cache last focused ObjectRef and a minimal interface set.
  - Expose `enum FocusStatus { ConfirmedEditable, NonEditable, Unknown }`.
- `src/text_injection/atspi_injector.rs` (feature = text-injection-atspi)
  - Resolve focused object; if it has EditableText → call `set_text_contents` or `insert_text`.
  - Else, if it has Action → find a “paste” action and `do_action(index)`.
  - Guard each D‑Bus call with a small timeout (~300 ms).
- `src/text_injection/clipboard_injector.rs` (feature = text-injection-clipboard)
  - Use `wl-clipboard-rs` to set the clipboard to the full session text.
  - Provide helper to combine with an AT‑SPI paste action when available.
- `src/text_injection/processor.rs`
  - Owns session + manager + optional focus; receives STT strings via an mpsc.
  - On silence timeout, calls `try_inject()` and clears the buffer.

Note: Keep code minimal and defensive; return early on empty/whitespace strings.

## Selection & timeouts (practical defaults)
- Build injector chain in this order:
  1) AT‑SPI2 EditableText/Action (if feature and available)
  2) Clipboard (set) + AT‑SPI “Paste” action (if both features available)
  3) Clipboard only (notify/trace; user pastes manually)
  4) ydotool (opt-in) — unchanged from Phase 1
- Timeouts: wrap D‑Bus calls in 150–300 ms timeouts; overall fallback budget ≤ 800 ms.
- Debounce focus changes by ~75 ms before injection.

## Minimal wiring
- Add a new optional `InjectionConfig { silence_timeout_ms: u64, inject_on_unknown_focus: bool, allow_ydotool: bool, restore_clipboard: bool }`.
- In main pipeline setup, spawn `InjectionProcessor` only when `--enable-text-injection` (or feature) is active.
- Log injector used and success/failure counts via existing tracing.

## Local testing (manual)
- Ensure AT‑SPI is present (KDE installs at-spi2 by default):
  - Wayland session: `$ echo $WAYLAND_DISPLAY`
  - Accessibility must be enabled (org.a11y.Status IsEnabled via DE settings).
  - Basic check: focus a text field in Kate/Firefox and run the demo (below).
- Add a tiny demo binary (optional) to exercise injectors without STT:
  - `cargo run -p coldvox-app --example vad_demo --features text-injection,text-injection-atspi,text-injection-clipboard`
  - Or create a small `examples/atspi_inject_demo.rs` that sends a fixed string through the manager.
- Clipboard path: verify paste works by focusing a text box and triggering clipboard+paste action.
- Keep the clipboard owner alive briefly (don’t drop immediately after setting contents).

Debugging helpers:
- accerciser to inspect the accessibility tree and verify EditableText/Action.
- busctl to introspect the accessibility bus (separate from the session bus).

## Risks & mitigations
- Terminals often lack EditableText → expect clipboard path.
- Focus may land on containers → try Action (Paste) before giving up.
- Localization of action names → normalize by lowercase match for "paste" in name/description.
- Wayland clipboard ownership semantics → keep provider alive until consumer reads.
 - Firefox Wayland may need `MOZ_ENABLE_WAYLAND=1` on some distros.
 - Electron apps often require Wayland flags (`--ozone-platform-hint=auto`, etc.) and remain quirky.

## Acceptance (good enough for Phase 2)
- Batch injection into common apps (Firefox text areas, Kate, LibreOffice) via AT‑SPI or clipboard+paste.
- Fallback to clipboard-only with a trace message when paste isn’t triggerable.
- Bounded latency: attempt primary path first; complete fallback sequence within ~0.8 s.
- No crashes on missing buses or unavailable features; injector list filters by `is_available()`.

Clarifications:
- Clipboard fallback has two modes:
  1) Clipboard + AT‑SPI Paste (when AT‑SPI is available but target lacks EditableText).
  2) Clipboard + user manual paste (when AT‑SPI is unavailable entirely).
- For one-shot probes, a helper using `desktop.get_active_descendant()` can fetch the current focus without subscribing to events.

## Follow-ups (Phase 3 candidates)
- ydotool integration behind explicit flag and consent prompt.
- IME workflows (IBus/Fcitx5) for apps that accept IME text but lack AT‑SPI EditableText.
- Per-app quirks cache and adaptive timings.

---

Appendix A — Cargo changes (sketch)

- Add features to `crates/app/Cargo.toml`:
  - `text-injection = []`
  - `text-injection-atspi = ["text-injection", "atspi"]`
  - `text-injection-clipboard = ["text-injection", "wl-clipboard-rs"]`
- Add deps under `[dependencies]` with `optional = true` as listed above.

Appendix B — Tiny contracts
- Input: UTF‑8 text from STT (already normalized); ignore empty.
- Output: Injected into focused widget, or clipboard set; errors traced.
- Errors: Missing bus, timeouts, non-editable focus → escalate to fallback.
- Success: Any injector returns Ok.
