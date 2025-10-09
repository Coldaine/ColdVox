# Master Plan: High-Performance Text Injection

This document synthesizes the architecture, implementation, and testing strategy for the ColdVox text injection system. It serves as the single source of truth, combining insights from `InjectionMaster.md`, `OpusCodeInject.md`, `InjectionTest1008.md`, and `OpusTestInject2.md`.

---

## 1. Core Design Principles

*   **Pre-warm:** As soon as the buffer state leaves `Idle`, pre-warm connections (AT-SPI, Portals) to minimize injection latency.
*   **Fast-Fail Stages:** Each injection stage must complete or fail in **≤ 50 ms**. Total end-to-end injection target is **≤ 200 ms**.
*   **Event-Based Success:** Confirm successful injection via text-change events (AT-SPI, UIA), not `sleep()` calls. If no confirmation, immediately try the next method.
*   **Clipboard Hygiene:** Always restore the clipboard to its original state after a paste operation. Optionally, clear the injected text from clipboard manager history (e.g., Klipper).
*   **Structured Diagnostics:** If all methods fail, return a structured error with clear, actionable advice for the user or system administrator.
*   **Ranked Methods per Environment:** Maintain a ranked list of injection methods for each supported environment and a compatibility memory for per-application overrides.

---

## 2. Injection Method Architecture

### 2.1. Method Rankings by Environment

**KDE Plasma (KWin, Wayland)**
1.  **AT-SPI Insert** (`EditableText.insert`)
2.  **AT-SPI Paste** (`EditableText.paste`) after seeding clipboard.
3.  **Portal/EIS "type"** (via `xdg-desktop-portal`)
4.  **KDE Fake-Input Helper** (privileged, feature-flagged)

**Hyprland (wlroots)**
1.  **AT-SPI Insert**
2.  **AT-SPI Paste**
3.  **wlr Virtual Keyboard** (`zwp_virtual_keyboard_v1`)
4.  **Portal/EIS "type"**

**Windows**
1.  **UI Automation (UIA)** (`ValuePattern`/`TextPattern`)
2.  **Clipboard + `SendInput` Ctrl+V** (with clipboard restoration)
3.  **`SendInput` Typing** (last resort)

### 2.2. Pre-Warm Strategy

Triggered when the session enters a `Buffering` state. The goal is to know which injection paths are viable *before* injection is requested.

```rust
// Pre-warm conceptual implementation
async fn prewarm(ctx: &mut Ctx) {
    // 1. Ping AT-SPI bus and snapshot focus
    ctx.a11y_ok = atspi::ping(10_ms).await;
    if ctx.a11y_ok {
        ctx.focus = atspi::snapshot_focus(20_ms).await;
        atspi::subscribe_text_changed(&ctx.focus);
    }

    // 2. Backup clipboard content
    if ctx.cfg.allow_clipboard {
        ctx.clip_backup = clip::snapshot_current(15_ms).await;
    }

    // 3. Ensure Portal session is ready
    if ctx.cfg.allow_portal && !ctx.portal.ready {
        ctx.portal.ready = portal::ensure_session_keyboard(40_ms).await;
    }

    // 4. Connect to compositor-specific virtual keyboard
    match ctx.env {
        Env::Hyprland if ctx.cfg.allow_virtual_kbd && ctx.vkbd.is_none() => {
            ctx.vkbd = vkbd::connect(25_ms).await.ok();
        }
        _ => {}
    }
}
```

---

## 3. Detailed Implementation Snippets

### 3.1. Hyprland/wlroots Virtual Keyboard

Handles connection, keymap management, and typing.

```rust
// From OpusCodeInject.md - Wayland Virtual Keyboard
use wayland_client::{Connection, protocol::wl_seat};
use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::{ZwpVirtualKeyboardManagerV1, ZwpVirtualKeyboardV1};
use xkbcommon::xkb;

pub struct VirtualKeyboard {
    conn: Connection,
    vkbd: ZwpVirtualKeyboardV1,
    keymap: xkb::Keymap,
    // ... more fields
}

impl VirtualKeyboard {
    pub async fn connect(timeout: Duration) -> Result<Arc<Mutex<Self>>> { /* ... */ }

    pub async fn type_text(&mut self, text: &str, chunk_size: usize) -> Result<()> {
        for chunk in text.chars().collect::<Vec<_>>().chunks(chunk_size) {
            for ch in chunk {
                self.type_char(*ch).await?;
            }
            tokio::time::sleep(Duration::from_micros(500)).await;
        }
        Ok(())
    }

    async fn type_char(&mut self, ch: char) -> Result<()> {
        let keysym = xkb::keysym_from_char(ch);
        // ... find keycode, handle shift modifier, send key press/release
        Ok(())
    }
}
```

### 3.2. Portal/EIS (Event-based Input Synthesis)

Handles session creation, device selection, and typing via the secure portal mechanism.

```rust
// From OpusCodeInject.md - Portal/EIS Implementation
use zbus::{Connection, proxy};
use reis::{ei, event::DeviceEvent};

#[proxy(interface = "org.freedesktop.portal.RemoteDesktop", ...)]
trait RemoteDesktop { /* ... */ }

pub struct PortalEIS {
    conn: Connection,
    session: Option<ObjectPath>,
    eis_context: Option<ei::Context>,
    // ... more fields
}

impl PortalEIS {
    pub async fn setup(timeout: Duration) -> Result<Arc<Mutex<Self>>> { /* ... */ }

    pub async fn ensure_eis_connection(&mut self) -> Result<()> { /* ... */ }

    pub async fn type_text_via_eis(&mut self, text: &str, timeout: Duration) -> Result<()> {
        // ... ensure connection, convert chars to keycodes, send via EIS
    }
}
```

### 3.3. KDE Fake Input

Uses KWin's privileged D-Bus interface for direct input synthesis.

```rust
// From OpusCodeInject.md - KWin Fake Input
use zbus::{Connection, proxy};

#[proxy(interface = "org.kde.kwin.FakeInput", ...)]
trait FakeInput {
    async fn authenticate(&self, app_id: &str, reason: &str) -> zbus::Result<bool>;
    async fn keyboard_key_press(&self, keycode: u32) -> zbus::Result<()>;
    async fn keyboard_key_release(&self, keycode: u32) -> zbus::Result<()>;
}

pub struct KWinFakeInput { /* ... */ }

impl KWinFakeInput {
    pub async fn new(timeout: Duration) -> Result<Arc<Mutex<Self>>> {
        // ... connect and authenticate
    }

    pub async fn type_text(&mut self, text: &str, chunk_size: usize) -> Result<()> {
        // ... type text in chunks via D-Bus calls
    }
}
```

---

## 4. Multi-Layered Testing Strategy

### 4.1. Test Distribution

-   **Service Integration (70%):** Complete injection flows with real dependencies (AT-SPI, clipboard).
-   **Trace-Based (15%):** Multi-app injection verification.
-   **Contract (10%):** Portal/EIS protocol compliance.
-   **Pure Logic (5%):** Keymap conversion, text chunking.

### 4.2. Pre-Commit Hooks (Fast Feedback)

A pre-commit hook runs a suite of fast, deterministic tests that must pass before any code is committed. These tests use behavioral fakes and target a completion time of **< 3 seconds**.

```bash
#!/bin/bash
# .git/hooks/pre-commit
set -e
echo "🚀 Running fast injection tests..."
pytest tests/injection/fast/ -m "not hardware" --fail-fast --timeout=0.5
echo "✅ Fast tests passed."
```

### 4.3. Hardware & End-to-End (E2E) Tests

These tests run in CI and validate the system against real hardware and applications. They are non-blocking for pushes but are required for releases.

**Hardware Test Matrix:**
-   **Continuous:** Mic capture, GPU availability, AT-SPI health.
-   **Nightly:** Real injection into KWin/Hyprland, physical audio tests.
-   **Required for Release:** Full WAV-to-injection pipeline validation.

**E2E Test Example (WAV → Injection):**

```python
# tests/e2e/test_complete_voice_pipeline.py
@pytest.mark.e2e
@pytest.mark.hardware
def test_wav_to_kate_injection(self, hardware_env):
    """
    Story: User speaks, text appears in Kate.
    """
    # 1. Setup: Start Kate, ensure focus, prepare audio
    hardware_env.start_app("kate")
    wav_file = "test_data/audio/hello_world.wav"

    # 2. Execute: Play audio through a virtual cable
    hardware_env.play_audio(wav_file)

    # 3. Verify: Check that the transcribed text appears in Kate
    wait_for(
        lambda: "Hello, world!" in hardware_env.get_kate_content(),
        timeout=5.0,
        error="Text never appeared in Kate"
    )

    # 4. Assert Telemetry: Check that all pipeline stages were recorded
    spans = hardware_env.get_trace()
    assert spans.has("audio.capture")
    assert spans.has("whisper.transcribe")
    assert spans.has("injection.attempt")
    assert spans.has("atspi.text_changed")
```

### 4.4. Failure & Resilience Testing

A `FailureMatrix` defines all conceivable failure scenarios (e.g., AT-SPI bus down, clipboard locked, portal denied) and the expected graceful degradation or recovery behavior. Each scenario is covered by a dedicated test.

---

## 5. Logging and Telemetry

Structured logging (e.g., JSON) is critical for diagnostics. Every injection attempt should generate a detailed trace.

**Example Log/Trace Event:**
```json
{
  "ts": "...",
  "env": "KDE",
  "utterance_id": "...",
  "app_id": "org.kde.kate",
  "prewarm": {"a11y_ok": true, "portal": true},
  "method": "atspi_insert",
  "stage_ms": 37,
  "confirm": {"text_changed": true},
  "clipboard": {"seeded": false, "restored": true},
  "result": "ok",
  "total_ms": 128
}
```

---

## 6. Setup and Permissions

| Capability         | KDE (KWin)                                 | Hyprland                                    | Windows           |
| ------------------ | ------------------------------------------ | ------------------------------------------- | ----------------- |
| **AT-SPI**         | Install/enable `at-spi2-core`              | Install/enable `at-spi2-core`               | n/a               |
| **Portal Keyboard**| `xdg-desktop-portal-kde`                   | `xdg-desktop-portal-hyprland` (or similar)  | n/a               |
| **Virtual Kbd**    | Privileged KWin Fake Input                 | `zwp_virtual_keyboard_v1` (wlr)             | n/a               |
| **Clipboard**      | `wl-clipboard`                             | `wl-clipboard`                              | Win32 Clipboard   |

---

## 7. Methods We Won't Try

-   **ydotool / raw uinput:** Requires root, flaky Unicode support.
-   **X11-only tricks:** The primary targets are Wayland-based.
-   **Blind `Ctrl+V` synthesis:** Disallowed by the Wayland security model.
-   **Deep, full-tree AT-SPI walks:** Too slow and fragile.