# Text Injection System Review – Oct 13, 2025

Owners requested: @Jules @codex

Scope: Most recent round of edits to the text injection system (orchestrator, clipboard paste, ydotool, focus tracker, manager).

Summary verdict: Looks correct overall with improved timeouts, redaction, throttled logging, and large-span tests. One concrete fix applied in this PR for X11 clipboard writing via xclip stdin. See details and checkboxes below.

## Changes reviewed
- ClipboardPasteInjector: timeout-wrapped clipboard read; AT-SPI-first paste with ydotool fallback; unconditional clipboard restore via delayed task; clear logging at debug/trace.
- YdotoolInjector: capability probing, socket autodetect, per-method and paste timeouts; permission checks with helpful warnings.
- Manager/Strategy: privacy-first redaction (hash/len), log throttle, success tracking + cooldown, metrics recording, focus gating with configurable Unknown handling.
- Focus: backend abstraction with caching; current SystemFocusAdapter returns Unknown (documented TODO).
- Prewarm/Orchestrator: small-timeout prewarming and staged fast-fails with explicit timeout mapping to InjectionError::Timeout.

## Logging review
- Levels: info for success milestones; debug/trace for flow; warn for recoverable failures; error when a method fails in the chain.
- Redaction: InjectionConfig.redact_logs=true by default; Manager logs only length/hash unless disabled, trace logs full text only when redaction disabled.
- Throttling: LogThrottle used in backend selection to reduce noise.
- Events: logging module includes structured helpers and event types; usage in manager aligns with our style.

Action taken: fixed ClipboardInjector X11 path to actually write to xclip via stdin and await exit; include stderr on failure.

## Testing review (large-span preferred)
- Real injection tests present in `src/tests/real_injection.rs` covering:
  - AT-SPI insert (simple, unicode, long, special chars)
  - Ydotool paste path with clipboard seeding
  - Clipboard+paste combined
  - Enigo typing
  - A readiness poll for GTK test app
- Tests check environment and skip gracefully when not applicable; use actual GTK app and clipboard; align with our large-span philosophy.
- Additional unit tests exist in manager for ordering, cooldowns, budgets, basic success/failure flows.

Gaps noted (non-blocking for this PR):
- Focus backend still a stub; add an integration that exercises focused editable vs non-editable paths via AT-SPI when feasible.
- Add a smoke test for X11 clipboard write/read roundtrip (behind feature flag, headful only), now that xclip stdin is fixed.
- Consider a short confirmation heuristic for paste success (existing confirm.rs + prewarm event listener paths are present but not wired in ClipboardPasteInjector).

## Requested feedback
- Confirm ydotool socket discovery covers your setups (HOME/.ydotool/socket, XDG_RUNTIME_DIR, /run/user/UID). Any additional common paths we should probe?
- Are default timeouts acceptable? per_method=250ms, paste_action=200ms. Too aggressive on slow machines?
- Manager: method order now prefers AT-SPI, then opt-ins, then ClipboardPasteFallback. Any objections?
- Logging: default redaction on. OK to keep it on by default for privacy?

## Sign-off checklist
- [x] Timeouts in all external calls (AT-SPI prewarm, paste, ydotool, wl-clipboard read)
- [x] Privacy redaction and log throttling
- [x] Method failure propagates to strategy with metrics and cooldown
- [x] Large-span tests present and runnable in headful envs
- [x] Clipboard restore performed asynchronously with small delay
- [x] X11 clipboard write fixed (this PR)

## CI/Docs
- TESTING.md describes large-span policy; aligns with our philosophy.
- If desired, we can add an optional X11 smoke test behind a feature (future PR).

Thanks!
