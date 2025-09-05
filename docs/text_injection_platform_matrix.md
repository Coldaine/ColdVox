# Text Injection — Platform Test Matrix

This document tracks platform-specific behavior and recommended backends for text injection across common Linux desktop environments and window managers. It complements docs/testing.md and text_injection_failover.md.

Target outcomes:
- Document reproducible setup per environment.
- Capture which backends work reliably and their caveats.
- Provide a simple script to collect env diagnostics.

Environment quick-dump:
- Run `scripts/collect_platform_env.sh` and attach the output to findings.

Backend legend:
- AT-SPI = Direct insert via AT-SPI EditableText
- Clipboard = Copy text, paste via app
- ydotool = Simulated Ctrl+V (Wayland, uinput)
- kdotool = KDE/X11 assistance
- Combo = Clipboard + Paste combo (AT-SPI paste → ydotool fallback)

## Matrix

| Backend | GNOME/Wayland | GNOME/X11 | KDE/Wayland | KDE/X11 | Sway | i3 |
|---------|---------------|-----------|-------------|---------|------|-----|
| AT-SPI | ? | ? | ? | ? | ? | ? |
| Clipboard | ? | ? | ? | ? | ? | ? |
| ydotool | ? | N/A | ? | N/A | ? | N/A |
| kdotool | N/A | ? | N/A | ? | N/A | ? |
| Combo | ? | ? | ? | ? | ? | ? |

Notes:
- Wayland compositors may require portals or explicit permissions.
- ydotool needs uinput; test under a dedicated group or with root when appropriate.
- AT-SPI requires accessibility services; ensure they are enabled.

## Per-Platform Setup

GNOME (Wayland & X11)
- Packages: `at-spi2-core`, `wl-clipboard`.
- Ensure Accessibility: enable from Settings → Accessibility.

KDE Plasma (Wayland & X11)
- Packages: `at-spi2-core`, `wl-clipboard`, `kdotool` (if used).
- KGlobalAccel may influence shortcuts; test with/without.

Sway / Hyprland
- Packages: `at-spi2-core`, `wl-clipboard`, `ydotool`.
- Verify ydotool daemon and uinput permissions.

XFCE / Cinnamon / i3
- X11-centric; ensure `xclip` or `xsel` if clipboard paths are exercised.

## How to Test

1) Basic injection
- Single word, multi-word, special chars, emoji, >1000 chars.

2) Performance
- Measure latency (rough): wall clock for 1k chars via clipboard and AT-SPI.

3) Edge cases
- Rapid injections, focus changes, multiple monitors/virtual desktops.

4) Failure scenarios
- No clipboard manager; accessibility disabled; permission denials.

## Recording Results

Create a short report per environment:
- Output from `scripts/collect_platform_env.sh`.
- Which backends succeeded/failed and error messages.
- Any required config changes or permissions.
- Subjective latency observations.

Once coverage improves, we can consider automating subsets in CI with headless compositors or mocks.
