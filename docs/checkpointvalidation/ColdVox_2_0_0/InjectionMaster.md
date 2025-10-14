got it—let’s ship something you can actually build against. below is a practical, **fast-fail** plan with **pre-warm**, **sub-50 ms stages**, **event-based success detection**, **strict clipboard hygiene**, **deep logging/telemetry**, and **live (no-mock) test hooks**. i split by environment: **KDE Plasma (KWin, Wayland)**, **Hyprland (wlroots)**, and **Windows**. then you’ll get pseudocode + tricky snippets + setup/permissions + a short “don’t-try” table.

---

# 0) design rules (you set these)

* **pre-warm** as soon as your buffer leaves `Idle` (don’t wait for inject time).
* **fast-fail** stages ≤ **50 ms** each; total target ≤ **200 ms** end-to-end.
* **success = events, not sleeps**: confirm via text-change events (AT-SPI or platform equivalent). if we cannot confirm, immediately try next method.
* **no clipboard leftovers**: always restore; optionally drop the item from clipboard managers.
* **no “No-Op”**: if all fail, return a **structured diagnostic** (clear env fix).
* **rank per environment**; keep a tiny **compat memory** for per-app overrides (on/off AT-SPI, preferred path, debounce).

---

# 1) method rankings (expectations)

## KDE Plasma (KWin, Wayland)

1. **AT-SPI Insert** (EditableText.insert) — fastest/safest where exposed.
2. **AT-SPI Paste** (EditableText.paste) after we seed the clipboard — equally robust when caret exists.
3. **Portal/EIS “type”** (authorized input via xdg-desktop-portal + libei) — needs consent; pre-warm to avoid latency.
4. **KDE fake-input** helper (privileged; feature-flagged) — only if you explicitly allow.

## Hyprland (wlroots)

1. **AT-SPI Insert**
2. **AT-SPI Paste**
3. **wlr Virtual Keyboard** (e.g., wtype-style synthesis) — standard wlroots path when a11y fails.
4. **Portal/EIS “type”** (if the portal on the system supports it).

## Windows

1. **UI Automation** (ValuePattern/TextPattern)
2. **Clipboard + SendInput Ctrl+V** (restore)
3. **SendInput typing** (last resort)

---

# 2) pre-warm (triggered when session enters Buffering)

**goal:** by the time you’re ReadyToInject, you already know what will work.

```rust
async fn prewarm(ctx: &mut Ctx) {
    // 1) AT-SPI bus & focus snapshot
    ctx.a11y_ok = atspi::ping(10_ms).await;
    if ctx.a11y_ok {
        ctx.focus = atspi::snapshot_focus(20_ms).await;         // path + role + has EditableText?
        atspi::subscribe_text_changed(&ctx.focus);               // arm confirm channel
    }

    // 2) clipboard backup (only if allowed)
    if ctx.cfg.allow_clipboard {
        ctx.clip_backup = clip::snapshot_current(15_ms).await;  // bytes + mimes
    }

    // 3) portal session (if enabled)
    if ctx.cfg.allow_portal && !ctx.portal.ready {
        ctx.portal.ready = portal::ensure_session_keyboard(40_ms).await;
    }

    // 4) compositor-specific
    match ctx.env {
        Env::Hyprland if ctx.cfg.allow_virtual_kbd && ctx.vkbd.is_none() => {
            ctx.vkbd = vkbd::connect(25_ms).await.ok();
        }
        Env::KDE => { /* optional: check fake-input helper availability */ }
        _ => {}
    }

    // 5) cache app hints (compat memory)
    ctx.compat = compat::lookup(ctx.focus.app_id(), ctx.focus.toolkit());
}
```

* **ttl:** keep pre-warm results “hot” for ~3s since last buffer event (refresh if stale).
* **never block** the main loop for pre-warm; each step has a tiny timeout.

---

# 3) injection micro-pipelines

## 3.1 shared helpers (success confirmation & hygiene)

```rust
// success confirmation via AT-SPI text events (preferred)
async fn confirm_text_changed(target: &AtspiNode, want_prefix: &str, window: Duration) -> bool {
    // listen for object:text-changed:inserted on target (or focused descendant)
    // accept success if first 3–6 chars match; prevents false positive on IME
    atspi::wait_text_inserted(target, want_prefix, window).await
}

// strict clipboard hygiene
async fn with_seeded_clipboard<F, R>(payload: &str, f: F, backup: ClipBackup) -> Result<R, InjectErr>
where F: Future<Output = Result<R, InjectErr>> {
    clip::set_exact(payload).await?;                   // write specific MIME(s)
    let res = f.await;
    clip::restore(backup).await?;                      // always restore
    if cfg!(feature="kde-klipper-clean") {
        clip::kde_clear_history().await.ok();          // best-effort
    }
    res
}
```

---

## 3.2 KDE Plasma (KWin, Wayland)

```rust
async fn inject_kde(text: &str, ctx: &mut Ctx) -> Result<(), InjectErr> {
    // Stage 1: AT-SPI Insert (≤ 50ms + confirm ≤ 75ms)
    if ctx.a11y_ok {
        if let Some(ed) = atspi::focused_editable(15_ms).await {
            if atspi::insert(&ed, text, 20_ms).await &&
               confirm_text_changed(&ed, &text_prefix(text), 75_ms).await {
                return Ok(());
            }
        }
    }

    // Stage 2: AT-SPI Paste with clipboard seed (≤ 50ms total)
    if ctx.a11y_ok && ctx.cfg.allow_clipboard {
        if let Some(ed) = atspi::focused_editable(10_ms).await {
            return with_seeded_clipboard(text, async {
                if atspi::paste_at_caret(&ed, 10_ms).await &&
                   confirm_text_changed(&ed, &text_prefix(text), 75_ms).await {
                    Ok(())
                } else { Err(InjectErr::StageFail("atspi_paste")) }
            }, ctx.clip_backup.take().unwrap_or_default()).await;
        }
    }

    // Stage 3: Portal/EIS typing (authorized; ≤ 50ms)
    if ctx.cfg.allow_portal && ctx.portal.ready {
        if portal::eis_type(text, 40_ms).await &&
           atspi::confirm_focus_stream(&text_prefix(text), 75_ms).await {
            return Ok(());
        }
    }

    // Stage 4: KDE fake-input (privileged; feature-flag)
    if ctx.cfg.allow_kde_fake_input {
        if kde_fake_input::type_text(text, 40_ms).await &&
           atspi::confirm_focus_stream(&text_prefix(text), 75_ms).await {
            return Ok(());
        }
    }

    Err(InjectErr::AllFailed)
}
```

---

## 3.3 Hyprland (wlroots)

```rust
async fn inject_hypr(text: &str, ctx: &mut Ctx) -> Result<(), InjectErr> {
    // 1) AT-SPI Insert
    if ctx.a11y_ok {
        if let Some(ed) = atspi::focused_editable(15_ms).await {
            if atspi::insert(&ed, text, 20_ms).await &&
               confirm_text_changed(&ed, &text_prefix(text), 75_ms).await {
                return Ok(());
            }
        }
    }

    // 2) AT-SPI Paste (clipboard seed + paste_text)
    if ctx.a11y_ok && ctx.cfg.allow_clipboard {
        if let Some(ed) = atspi::focused_editable(10_ms).await {
            return with_seeded_clipboard(text, async {
                if atspi::paste_at_caret(&ed, 10_ms).await &&
                   confirm_text_changed(&ed, &text_prefix(text), 75_ms).await {
                    Ok(())
                } else { Err(InjectErr::StageFail("atspi_paste")) }
            }, ctx.clip_backup.take().unwrap_or_default()).await;
        }
    }

    // 3) wlr virtual keyboard (≤ 50ms)
    if let Some(vkbd) = ctx.vkbd.as_ref() {
        if vkbd::type_text(vkbd, text, 40_ms).await &&
           atspi::confirm_focus_stream(&text_prefix(text), 75_ms).await {
            return Ok(());
        }
    }

    // 4) Portal/EIS typing (authorized)
    if ctx.cfg.allow_portal && ctx.portal.ready {
        if portal::eis_type(text, 40_ms).await &&
           atspi::confirm_focus_stream(&text_prefix(text), 75_ms).await {
            return Ok(());
        }
    }

    Err(InjectErr::AllFailed)
}
```

---

## 3.4 Windows

```rust
fn inject_windows(text: &str, ctx: &mut WinCtx) -> Result<(), InjectErr> {
    // 1) UIA direct
    if let Some(el) = uia::focused_editable(25_ms)? {
        if uia::set_value(&el, text, 25_ms)? || uia::insert_range(&el, text, 25_ms)? {
            return Ok(());
        }
    }

    // 2) Clipboard + Ctrl+V (restore)
    let backup = winclip::snapshot()?;
    winclip::set_unicode(text)?;
    sendinput::ctrl_v()?;
    // confirmation: UIA text-changed / value changed / caret advance
    if uia::confirm_changed(75_ms)? {
        winclip::restore(backup)?;
        return Ok(());
    }
    winclip::restore(backup)?;
    // 3) SendInput typing (last)
    sendinput::type_text(text)?;
    if uia::confirm_changed(75_ms)? { Ok(()) } else { Err(InjectErr::AllFailed) }
}
```

---

# 4) success detection (deep but fast)

* **Preferred:** AT-SPI `object:text-changed:inserted` from the exact `Accessible` we targeted (or its focused descendant with role in `{entry,text,document_*}`).
* **Confirm prefix only** (first 3-6 visible characters) to avoid grapheme/IME mismatches.
* **Timeout windows** small (≤ 75 ms). if nothing arrives: **fail immediately**.

---

# 5) logging & telemetry (structured)

**levels**

* `TRACE`: raw events (AT-SPI event names, payload sizes—not content).
* `DEBUG`: decisions, timings per stage, chosen method, confirm outcomes.
* `INFO`: success summary (chars, method, total_ms).
* `WARN/ERROR`: explicit fix hints (enable a11y bus; allow portal; not supported compositor, etc.).

**fields per attempt**

```json
{
  "ts": "...",
  "env": "KDE|Hyprland|Windows",
  "utterance_id": "...",
  "app_id": "...",
  "title": "...",
  "role": "entry|text|document_*|...",
  "prewarm": {"a11y_ok":true,"portal":true,"vkbd":false},
  "method": "atspi_insert|atspi_paste|vkbd|portal|kde_fake|uia|clipboard_paste|sendinput",
  "stage_ms": 37,
  "confirm": {"text_changed":true,"caret_moved":true},
  "clipboard": {"seeded":true,"restored":true,"manager_cleared":true},
  "result": "ok",
  "total_ms": 128
}
```

**user-facing failure message (example)**

> “No authorized input path was available in this session. Enable the accessibility bus **or** permit the Remote Desktop portal for keyboard input. If this is a headless session, start `at-spi2-registryd` or run with a portal backend.”

---

# 6) end-to-end tests (live, Nobara runner; no mocks)

**apps**: Kate, KWrite, Konsole prompt, Chromium/Firefox input, VS Code.

**scenarios**

1. **AT-SPI Insert**: inject “hello✓世界”; assert `text-changed` within 75 ms.
2. **AT-SPI Paste**: seed + `paste_text`; confirm + clipboard restore; optional Klipper clear.
3. **Portal/EIS**: disable a11y bus, authorize portal, send; confirm via events (if available) or caret/length delta when a11y comes back.
4. **Hyprland VKBD**: type 200–500 chars; assert no key-stick; confirm insert events in focused widget if exposed.
5. **Windows UIA**: SetValue then read Value back.

**non-blocking**: test harness returns immediately with a “pending live check” field; a background confirmer flips the status within ~100 ms and logs the result. (You still don’t block UI.)

---

# 7) KDE window watcher (optional KWin script)

* purpose: stream `{appId, title, pid}` on activation; handy for compat memory and targeted heuristics.

```js
// kwin script (ColdVoxWatcher)
workspace.clientActivated.connect(function (c) {
  print("[ColdVoxWatcher] activated",
        JSON.stringify({ appId: c.resourceName, title: c.caption, pid: c.pid }));
});
```

(you can wrap with a tiny companion service that exposes these via DBus or writes to a unix socket your injector reads.)

---

# 8) setup & permissions (one-time)

| Capability           | KDE (KWin)                                                      | Hyprland                                             | Windows         |
| -------------------- | --------------------------------------------------------------- | ---------------------------------------------------- | --------------- |
| **AT-SPI running**   | install/enable `at-spi2-core` (session spawns registry)         | same                                                 | n/a             |
| **Portal keyboard**  | `xdg-desktop-portal` + KDE backend; allow session               | `xdg-desktop-portal-hyprland` (or compositor portal) | n/a             |
| **Virtual keyboard** | not standard; optional **KDE fake-input** helper (feature-flag) | `zwp_virtual_keyboard_v1` (wlr vkbd)                 | n/a             |
| **Clipboard**        | `wl-clipboard` (seed/restore), optional Klipper DBus clear      | same                                                 | Win32 Clipboard |
| **IBus (optional)**  | run IBus if you want commit-text path for IME                   | same                                                 | n/a             |

---

# 9) methods we **won’t** try (your call enforced)

| Method                      | Why not                                                               |
| --------------------------- | --------------------------------------------------------------------- |
| ydotool / raw uinput        | root/daemon, flaky Unicode, security surface                          |
| X11-only tricks             | you’re on Wayland                                                     |
| blind Ctrl+V synthesis      | disallowed by Wayland model; use portal or fake-input                 |
| full-tree deep AT-SPI walks | slow/fragile; we only BFS when focus isn’t editable, with strict caps |

---

# 10) “tricky bits” snippets

### a) AT-SPI: find focused EditableText fast (bounded)

```rust
async fn focused_editable(timeout: Duration) -> Option<AtspiNode> {
    let f = atspi::focused_node(timeout).await?;      // object with role/state
    if f.has_editable_text { return Some(f); }
    bfs::find_nearby_editable(&f, max_nodes=3000, max_depth=10, 12_ms).await
}
```

### b) wl-clipboard hygiene (seed + restore)

```rust
async fn seed_and_restore(text: &str, backup: ClipBackup, f: impl Future<Output=bool>) -> bool {
    clip::set_exact_utf8(text).await.ok()?;
    let ok = f.await;
    clip::restore(backup).await.ok();
    #[cfg(feature="kde-klipper-clean")]
    { let _ = clip::kde_clear_history().await; }
    ok
}
```

### c) Hyprland wlr virtual keyboard (skeletal)

```rust
// bind zwp_virtual_keyboard_manager_v1, create vkbd; convert text -> keysyms; send key down/up
async fn vkbd_type_text(vkbd: &mut Vkbd, text: &str, budget: Duration) -> bool { /* … */ }
```

### d) Portal/EIS typing (skeletal)

```rust
// CreateSession -> SelectDevices(keyboard) -> Start -> ConnectToEIS -> speak EI to send keys
async fn eis_type(text: &str, budget: Duration) -> bool { /* … */ }
```

### e) Windows UIA quick path

```rust
fn uia_set_or_insert(el: &UIAElement, text: &str, budget: Duration) -> bool {
    uia::set_value(el, text, budget).ok()
      || uia::insert_via_textpattern(el, text, budget).ok()
}
```

---

## final notes

* this is **lean** by design: AT-SPI first, then a compositor-sanctioned path, all with **pre-warm** and **event-based confirmation**.
* you’ll maintain a small **“works with AT-SPI” app list** (compat memory) and skip brainstorming when we already know a path won’t work.
* failures always return a **precise, actionable message** (e.g., “accessibility bus off”, “portal not authorized”, “no virtual-keyboard support on this compositor”).

if you want, I can turn this into **drop-in Rust modules** (`atspi_injector.rs`, `clipboard_injector.rs`, `vkbd_injector.rs`, `portal_injector.rs`, `logging.rs`) wired to your `StrategyManager` trait with the exact timeouts and confirm logic above.
