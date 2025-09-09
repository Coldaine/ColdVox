# Text Injection in Headless / Minimal Desktop Environments

This note explains the repeated `UnknownMethod` errors you may see in test logs for the
text injection subsystem (AT-SPI path) and why they are expected in certain environments.

## Summary
- Errors originate from the D-Bus call: `org.a11y.atspi.Collection.GetMatches`.
- In lean/headless CI or lightweight WM sessions (no full GNOME / KDE shell), that
  higher‑level AT-SPI Collection interface is absent → D-Bus returns
  `org.freedesktop.DBus.Error.UnknownMethod`.
- The injection stack is designed to degrade gracefully: focus probing downgrades to
  `FocusStatus::Unknown`, AT-SPI insertion attempts fail fast, the adaptive strategy
  applies cooldowns, and fallback methods (clipboard / noop) remain testable.

These logs therefore validate resilience rather than indicating a regression.

## Where in the Code
| Component | Responsibility | Behavior When Method Missing |
| --------- | ------------- | ----------------------------- |
| `focus.rs` (`FocusTracker`) | Attempts rich focus classification via AT-SPI | Logs the UnknownMethod error, returns `FocusStatus::Unknown` |
| Strategy Manager | Chooses injection backend order | Continues; may mark AT-SPI method as failed and decay priority |
| Session / Processor | Buffers and dispatches text | Still runs; may inject via clipboard or produce a controlled failure |

## Why This Is Acceptable
1. **Graceful Degradation** – Production systems without full accessibility stacks still function using fallback injection.
2. **Test Coverage** – Exercising cooldown, failure accounting, and fallback ordering logic under adverse conditions.
3. **No False Pass** – We do not silently ignore errors; we record them and adapt.

## Making the Errors Disappear (Optional)
To see “clean” runs (successful AT-SPI focus + insert), ensure a full desktop accessibility stack:
1. Launch inside a real user desktop session (GNOME / KDE) instead of bare Xvfb.
2. Ensure these processes exist: `at-spi2-registryd`, a per-session `dbus-daemon`.
3. Environment variables exported: `DISPLAY`, `DBUS_SESSION_BUS_ADDRESS`, (X11) `XAUTHORITY`.
4. On Wayland: run a compositor providing accessibility (e.g. full GNOME Wayland session).
5. Confirm a simple AT-SPI query works (e.g. `python -c 'import pyatspi; print(pyatspi.Registry.getDesktopCount())'`).

## Troubleshooting Matrix
| Symptom | Likely Cause | Action |
| ------- | ------------ | ------ |
| `UnknownMethod` on `GetMatches` | Minimal AT-SPI implementation | Accept in CI / switch to full desktop |
| All AT-SPI inject methods fail quickly | No accessible focus tree | Rely on clipboard path; verify a11y daemons |
| Clipboard injection also fails | Missing clipboard utilities (Wayland/X11) | Install `wl-clipboard` / `xclip` |
| Excessive log noise | Repeated backend selection / failures | (Future) throttle logging or demote to `trace` |

## Planned / Optional Improvements
- Add a one-time capability probe to disable AT-SPI attempts after N consecutive `UnknownMethod` failures in a session.
- Suppress repeat logs (first at `WARN`, subsequent at `TRACE`).
- Add a metric: `a11y_capability_missing_total` for observability dashboards.

## When to Worry
You should investigate only if **all** of these become true simultaneously:
- Running inside a full desktop with accessibility daemons present.
- Other AT-SPI tools (e.g. `accerciser`, `pyatspi`) can enumerate apps.
- Clipboard fallback also fails unexpectedly.

Otherwise: the errors are noise from an intentionally reduced environment, and the fallback path is operating as designed.

## Quick Environment Check Script (Optional)
```bash
#!/usr/bin/env bash
set -euo pipefail
echo "DISPLAY=${DISPLAY:-}"
pgrep -fl at-spi2-registryd || echo "(at-spi2-registryd not running)"
dbus-send --session --dest=org.a11y.atspi.Registry \
  /org/a11y/atspi/registry org.freedesktop.DBus.Introspectable.Introspect >/dev/null 2>&1 \
  && echo "AT-SPI registry reachable" || echo "AT-SPI registry NOT reachable"
```

## FAQ
**Does this hide real regressions?** No—the fallback still exercises internal accounting paths; panic conditions are unaffected.

**Should we silence the log?** Not yet; it provides evidence the degraded path was covered. We may demote after adding explicit capability metrics.

**Can we forcibly skip AT-SPI in CI?** Yes: add a feature flag or env gate (e.g. `COLDVOX_DISABLE_ATSPI=1`) and short-circuit backend ordering—future enhancement.

---
Document version: 2025‑09‑09
Maintainer note: Update this if AT-SPI probing strategy or fallback ordering changes.
