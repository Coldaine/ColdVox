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

# ColdVox Text Injection Architecture

## Overview

The ColdVox text injection system uses a **fast-fail orchestrator** design with environment-specific injection pipelines, AT-SPI event-based confirmation, and pre-warming for sub-200ms latency.

## Core Principles

1. **Pre-warm on Buffering** - Don't wait until injection time
2. **Fast-fail stages** - ≤50ms per method, ≤200ms total
3. **Event-based confirmation** - No sleeps/polls, only AT-SPI `text-changed` events
4. **Fixed per-env pipelines** - No dynamic adaptation, just known-good paths
5. **No NoOp fallback** - All failures return structured diagnostics

## Architecture Components

### 1. Pre-Warm System (`prewarm.rs`)

Triggered when session enters `Buffering` state:

```rust
async fn prewarm(ctx: &mut Context) {
    // All operations run in parallel with tiny timeouts

    // 1) AT-SPI bus & focus snapshot (20ms timeout)
    ctx.a11y_ok = atspi::ping(10_ms).await;
    if ctx.a11y_ok {
        ctx.focus = atspi::snapshot_focus(20_ms).await;
        atspi::subscribe_text_changed(&ctx.focus);
    }

    // 2) Clipboard backup (15ms timeout)
    if ctx.cfg.allow_clipboard {
        ctx.clip_backup = clip::snapshot_current(15_ms).await;
    }

    // 3) Portal session (40ms timeout)
    if ctx.cfg.allow_portal && !ctx.portal.ready {
        ctx.portal.ready = portal::ensure_session_keyboard(40_ms).await;
    }

    // 4) Compositor-specific (25ms timeout)
    match ctx.env {
        Env::Hyprland if ctx.cfg.allow_virtual_kbd => {
            ctx.vkbd = vkbd::connect(25_ms).await.ok();
        }
        _ => {}
    }

    // 5) Load compat hints
    ctx.compat = compat::lookup(ctx.focus.app_id(), ctx.focus.toolkit());
}
```

**Pre-warm results cached for ~3s** with TTL refresh on new buffer activity.

### 2. Orchestrator (`orchestrator.rs`)

Environment detection → fixed pipeline → fast-fail loop:

```rust
pub struct Orchestrator {
    env: Environment,
    config: InjectionConfig,
    context: Context,
}

impl Orchestrator {
    pub async fn inject(&mut self, text: &str) -> Result<(), InjectErr> {
        let pipeline = match self.env {
            Environment::KDEPlasma => vec![
                Method::AtspiInsert,
                Method::AtspiPaste,
                Method::PortalEis,
                Method::KdeFakeInput,
            ],
            Environment::Hyprland => vec![
                Method::AtspiInsert,
                Method::AtspiPaste,
                Method::VirtualKeyboard,
                Method::PortalEis,
            ],
            Environment::Windows => vec![
                Method::UiaValue,
                Method::ClipboardPaste,
                Method::SendInput,
            ],
        };

        for method in pipeline {
            if method.timeout_elapsed(50_ms) { continue; }

            match method.execute(text, &mut self.context).await {
                Ok(()) if self.confirm(text).await => return Ok(()),
                _ => continue, // Fast-fail to next
            }
        }

        Err(InjectErr::AllFailed(self.diagnostic()))
    }
}
```

### 3. Confirmation System (`confirm.rs`)

AT-SPI event-based success detection:

```rust
pub async fn text_changed(
    target: &AtspiNode,
    want_prefix: &str,
    window: Duration
) -> bool {
    // Listen for object:text-changed:inserted on target
    // or focused descendant with acceptable roles
    atspi::wait_text_inserted(target, want_prefix, window).await
}
```

- **Prefix-only matching** - First 3-6 visible chars to avoid IME/grapheme issues
- **Tight windows** - ≤75ms timeout, fail immediately if no event
- **No polling** - Event subscription only

### 4. Clipboard Hygiene (`injectors/clipboard.rs`)

Wrapper for seed/restore with optional Klipper cleanup:

```rust
pub async fn with_seed_restore<F, R>(
    payload: &str,
    backup: ClipBackup,
    f: F
) -> Result<R>
where F: Future<Output = Result<R>> {
    clip::set_exact(payload).await?;
    let res = f.await;
    clip::restore(backup).await?;

    #[cfg(feature = "kde-klipper-clean")]
    clip::kde_clear_history().await.ok();

    res
}
```

### 5. Compat Memory (`compat.rs`)

Simple JSON per-app hints:

```json
{
  "org.kde.kate": {
    "focus_debounce_ms": 150,
    "preferred_order": ["AtspiPaste", "AtspiInsert"],
    "notes": "Focus events lag on startup"
  },
  "org.gnome.Terminal": {
    "focus_debounce_ms": 120,
    "disable_methods": ["AtspiPaste"],
    "notes": "Paste triggers confirmation dialog"
  }
}
```

No ML, no statistics - just manual overrides for known quirks.

## KWin Window Watcher (Optional)

### Installation

1. **Create script file**: `~/.local/share/kwin/scripts/coldvox-watcher/contents/code/main.js`

```javascript
// ColdVox Window Watcher for KDE Plasma
// Streams window activation events to ColdVox for enhanced app detection

const SOCKET_PATH = "/tmp/coldvox-window-events.sock";

// Helper to determine window type
function getWindowType(w) {
    if (w.normalWindow) return "normal";
    if (w.dialog) return "dialog";
    if (w.splash) return "splash";
    if (w.menu) return "menu";
    if (w.toolbar) return "toolbar";
    if (w.dock) return "dock";
    return "other";
}

// Build comprehensive window info
function buildWindowInfo(w) {
    if (!w) return null;

    return {
        // Identity
        appId: w.resourceName || "",
        class: w.resourceClass || "",
        role: w.windowRole || "",

        // Metadata
        title: w.caption || "",
        pid: w.pid || 0,

        // Type classification
        type: getWindowType(w),

        // State flags (injection relevance)
        minimized: w.minimized || false,
        fullscreen: w.fullScreen || false,
        active: w.active || false,
        modal: w.modal || false,

        // Context (optional, for advanced compat)
        desktop: (w.desktops && w.desktops[0]) ? w.desktops[0].id : null,
        output: w.output ? w.output.name : null,

        // Timestamp
        timestamp: Date.now()
    };
}

// Main activation handler
workspace.windowActivated.connect(function(window) {
    const info = buildWindowInfo(window);
    if (!info) {
        console.log("[ColdVoxWatcher] windowActivated: null window");
        return;
    }

    // Log for debugging (visible in KWin debug console)
    console.log("[ColdVoxWatcher] activated:", JSON.stringify(info));

    // TODO: Send to Unix socket or DBus
    // For now, logs are sufficient for monitoring
    // Future: Implement actual IPC to ColdVox daemon
});

// Optional: Track window additions for desktop changes
workspace.windowAdded.connect(function(window) {
    if (!window) return;

    console.log("[ColdVoxWatcher] windowAdded:",
        window.resourceName,
        "pid:", window.pid);
});

console.log("[ColdVoxWatcher] Initialized - monitoring window events");
```

2. **Create metadata**: `~/.local/share/kwin/scripts/coldvox-watcher/metadata.json`

```json
{
    "KPlugin": {
        "Id": "coldvox-watcher",
        "Name": "ColdVox Window Watcher",
        "Description": "Streams window activation events for ColdVox text injection",
        "Authors": [
            {
                "Name": "ColdVox Project"
            }
        ],
        "Category": "Accessibility",
        "Version": "1.0",
        "Website": "https://github.com/yourusername/coldvox"
    }
}
```

3. **Enable the script**:
```bash
# List installed scripts
qdbus org.kde.KWin /Scripting org.kde.kwin.Scripting.scripts

# Load the script
kwriteconfig5 --file kwinrc --group Plugins --key coldvox-watcherEnabled true
qdbus org.kde.KWin /KWin reconfigure
```

4. **View logs** (for debugging):
```bash
# KWin debug console
qdbus org.kde.KWin /Scripting org.kde.kwin.Scripting.loadScript \
  ~/.local/share/kwin/scripts/coldvox-watcher/contents/code/main.js

# Or check system logs
journalctl --user -f | grep ColdVoxWatcher
```

### Data Flow

```
Window Activation
       ↓
KWin Script (main.js)
       ↓
buildWindowInfo()
       ↓
{appId, class, role, title, pid, type, state...}
       ↓
[Future: Unix Socket → ColdVox]
       ↓
compat::update_hints()
       ↓
Context enrichment for next injection
```

### Why This Matters

**Without KWin hook**: Fall back to `window_manager.rs` (xprop/qdbus/swaymsg)
- Synchronous blocking calls
- ~20-50ms latency per query
- No proactive updates

**With KWin hook**: Real-time stream
- Zero latency (already cached)
- Window state awareness (minimized, fullscreen)
- Type-based strategy hints (dialog vs normal)

### Property Usage

| Property | Use Case | Example |
|----------|----------|---------|
| `appId` + `class` | Primary app matching | `"kate"` + `"org.kde.kate"` → load Kate hints |
| `role` | Distinguish windows | Main editor vs settings dialog |
| `type` | Strategy selection | Dialogs → prefer paste over insert |
| `fullscreen` | Behavior override | Games/media → queue injection until exit |
| `minimized` | Skip injection | Background windows → defer until focus |
| `modal` | Focus validation | Modal dialogs might steal focus mid-injection |
| `desktop`/`output` | Multi-workspace hints | Only inject on specific virtual desktop |

## Environment-Specific Pipelines

### KDE Plasma (KWin, Wayland)

```rust
async fn inject_kde(text: &str, ctx: &mut Ctx) -> Result<()> {
    // Stage 1: AT-SPI Insert (≤50ms)
    if ctx.a11y_ok {
        if let Some(ed) = atspi::focused_editable(15_ms).await {
            if atspi::insert(&ed, text, 20_ms).await &&
               confirm::text_changed(&ed, &prefix(text), 75_ms).await {
                return Ok(());
            }
        }
    }

    // Stage 2: AT-SPI Paste (≤50ms)
    if ctx.a11y_ok && ctx.cfg.allow_clipboard {
        if let Some(ed) = atspi::focused_editable(10_ms).await {
            return clipboard::with_seed_restore(text, async {
                atspi::paste_at_caret(&ed, 10_ms).await &&
                confirm::text_changed(&ed, &prefix(text), 75_ms).await
            }, ctx.clip_backup).await;
        }
    }

    // Stage 3: Portal/EIS (≤50ms)
    if ctx.cfg.allow_portal && ctx.portal.ready {
        if portal::eis_type(text, 40_ms).await &&
           confirm::focus_stream(&prefix(text), 75_ms).await {
            return Ok(());
        }
    }

    // Stage 4: KDE fake-input (privileged, feature-flagged)
    if ctx.cfg.allow_kde_fake_input {
        if kde_fake_input::type_text(text, 40_ms).await &&
           confirm::focus_stream(&prefix(text), 75_ms).await {
            return Ok(());
        }
    }

    Err(InjectErr::AllFailed)
}
```

### Hyprland (wlroots)

```rust
async fn inject_hypr(text: &str, ctx: &mut Ctx) -> Result<()> {
    // 1) AT-SPI Insert
    if ctx.a11y_ok {
        if let Some(ed) = atspi::focused_editable(15_ms).await {
            if atspi::insert(&ed, text, 20_ms).await &&
               confirm::text_changed(&ed, &prefix(text), 75_ms).await {
                return Ok(());
            }
        }
    }

    // 2) AT-SPI Paste
    if ctx.a11y_ok && ctx.cfg.allow_clipboard {
        if let Some(ed) = atspi::focused_editable(10_ms).await {
            return clipboard::with_seed_restore(text, async {
                atspi::paste_at_caret(&ed, 10_ms).await &&
                confirm::text_changed(&ed, &prefix(text), 75_ms).await
            }, ctx.clip_backup).await;
        }
    }

    // 3) wlr Virtual Keyboard (≤50ms)
    if let Some(vkbd) = ctx.vkbd.as_ref() {
        if vkbd::type_text(vkbd, text, 40_ms).await &&
           confirm::focus_stream(&prefix(text), 75_ms).await {
            return Ok(());
        }
    }

    // 4) Portal/EIS
    if ctx.cfg.allow_portal && ctx.portal.ready {
        if portal::eis_type(text, 40_ms).await &&
           confirm::focus_stream(&prefix(text), 75_ms).await {
            return Ok(());
        }
    }

    Err(InjectErr::AllFailed)
}
```

### Windows

```rust
fn inject_windows(text: &str, ctx: &mut WinCtx) -> Result<()> {
    // 1) UIA direct (ValuePattern/TextPattern)
    if let Some(el) = uia::focused_editable(25_ms)? {
        if uia::set_value(&el, text, 25_ms)? ||
           uia::insert_range(&el, text, 25_ms)? {
            return Ok(());
        }
    }

    // 2) Clipboard + Ctrl+V
    let backup = winclip::snapshot()?;
    winclip::set_unicode(text)?;
    sendinput::ctrl_v()?;

    if uia::confirm_changed(75_ms)? {
        winclip::restore(backup)?;
        return Ok(());
    }
    winclip::restore(backup)?;

    // 3) SendInput typing
    sendinput::type_text(text)?;
    if uia::confirm_changed(75_ms)? {
        Ok(())
    } else {
        Err(InjectErr::AllFailed)
    }
}
```

## Logging Schema

Single event per injection attempt:

```json
{
  "ts": "2024-10-09T12:34:56.789Z",
  "env": "KDE",
  "utterance_id": "uuid",
  "app_id": "org.kde.kate",
  "class": "kate",
  "role": "MainWindow",
  "title": "document.txt - Kate",
  "type": "normal",
  "fullscreen": false,
  "minimized": false,
  "prewarm": {
    "a11y_ok": true,
    "portal": true,
    "vkbd": false,
    "clip_backup": true
  },
  "method": "atspi_insert",
  "stage_ms": 37,
  "confirm": {
    "text_changed": true,
    "caret_moved": true
  },
  "clipboard": {
    "seeded": false,
    "restored": false,
    "manager_cleared": false
  },
  "result": "ok",
  "total_ms": 128,
  "char_count": 42
}
```

Levels:
- `TRACE`: Raw AT-SPI event names (no payload text)
- `DEBUG`: Decision points, timings, method selection
- `INFO`: Success summary with structured JSON
- `WARN/ERROR`: Failure diagnostics with fix hints

## Integration with Main Pipeline

```
STT Final Event
      ↓
InjectionSession (buffering)
      ↓
[Trigger pre-warm when entering Buffering]
      ↓
Silence detected → ReadyToInject
      ↓
Orchestrator::inject()
      ↓
Environment pipeline → fast-fail loop
      ↓
Success (with confirmation) or Diagnostic
```

## Configuration

```rust
pub struct InjectionConfig {
    // Method toggles
    pub allow_clipboard: bool,
    pub allow_portal: bool,
    pub allow_kde_fake_input: bool,
    pub allow_virtual_kbd: bool,

    // Timing budgets
    pub confirm_window_ms: u64,      // 75
    pub stage_budget_ms: u64,        // 50
    pub total_budget_ms: u64,        // 200
    pub focus_debounce_ms: u64,      // 100 (overridden by compat)

    // Clipboard
    pub clipboard_restore_delay_ms: u64,  // 500
}
```

## Testing Strategy

Live integration tests on Nobara runner:

1. **AT-SPI Insert**: Type `"hello✓世界"` into Kate → assert event within 75ms
2. **AT-SPI Paste**: Seed clipboard → paste → confirm + restore + Klipper clear
3. **Hyprland VKeyboard**: Type 200 chars → no stuck keys → confirm events
4. **Portal/EIS**: Disable a11y → authorize portal → inject → confirm
5. **Windows UIA**: SetValue → read Value back

Non-blocking mode: Immediate status + scheduled 100ms confirm check.

## What NOT to Do

| Method | Why Excluded |
|--------|--------------|
| ydotool/raw uinput | Root daemon, flaky Unicode, security surface |
| X11-only tricks | Wayland-first architecture |
| Blind Ctrl+V synthesis | Portal/fake-input required |
| Deep AT-SPI tree walks | Bounded BFS only when focus fails (max 3000 nodes, 10 depth, 12ms) |
| NoOp fallback | Return structured diagnostic instead |

## Future Enhancements

1. **KWin Socket Bridge**: Unix socket instead of console logs
2. **Portal EIS**: Full libei integration when compositor support stabilizes
3. **IBus commit-text**: Direct IME path for CJK languages
4. **Per-toolkit hints**: GTK vs Qt vs Electron strategies
5. **Wayland vkeyboard protocol v2**: When spec finalizes

## Migration from Current Architecture

Current (StrategyManager) → Target (Orchestrator):

1. **Delete**: `manager.rs` (1390 lines), `session.rs`, `processor.rs`, `noop_injector.rs`
2. **Create**: `orchestrator.rs`, `prewarm.rs`, `confirm.rs`, `logging.rs`
3. **Reorganize**: Move injectors to `injectors/` subdirectory
4. **Simplify**: Remove adaptive selection, cooldowns, success tracking
5. **Add**: Event-based confirmation, pre-warming, KWin hook

Estimated LOC reduction: ~5400 → ~1500-2000 lines.
