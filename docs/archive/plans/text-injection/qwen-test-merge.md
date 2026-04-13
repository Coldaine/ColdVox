---
doc_type: plan
subsystem: text-injection
status: draft
freshness: stale
preservation: preserve
last_reviewed: 2025-10-19
owners: Documentation Working Group
version: 1.0.0
---

Absolutely. Below is a **comprehensive, production-grade test plan** for the **ColdVox vNext text injection stack**, specifically targeting the three core injection backends you’ve implemented:

1. **Wayland Virtual Keyboard (Hyprland/wlroots)**
2. **Portal/EIS (xdg-desktop-portal + libei)**
3. **KWin Fake Input (KDE Plasma)**

The plan covers **unit**, **integration**, **end-to-end**, and **failure-mode** testing, with clear success criteria, environment setup notes, and telemetry hooks.

---

## 🧪 **Test Strategy Overview**

| Test Type | Purpose | Scope | Execution Frequency |
|----------|--------|-------|---------------------|
| **Unit** | Validate individual functions (keymap, keycode resolution, D-Bus stubs) | Isolated Rust modules | On every commit |
| **Integration** | Verify protocol handshakes, event flows, and component wiring | Full injector + mock compositor/portal | Nightly + PR |
| **E2E (Live)** | Confirm real-world injection in target apps (Kate, VS Code, etc.) | Full ColdVox stack on real Wayland session | Weekly + release |
| **Failure/Edge** | Validate graceful degradation, timeouts, error recovery | All methods under stress/failure | Nightly |
| **Permissions/Setup** | Ensure correct system setup (uinput, portal auth, AT‑SPI) | Pre-flight checks | On install/startup |

---

## 🔧 **Test Environment Requirements**

| Component | Required Setup |
|----------|----------------|
| **OS** | Nobara Linux (KDE Plasma 6 + Hyprland dual-boot or VM) |
| **AT‑SPI** | `at-spi2-core`, `QT_LINUX_ACCESSIBILITY_ALWAYS_ON=1` |
| **Portals** | `xdg-desktop-portal-kde` (KDE), `xdg-desktop-portal-hyprland` (Hyprland) |
| **Input** | User in `input` group (for uinput fallbacks, though not primary) |
| **Apps** | Kate, Konsole, Firefox, VS Code (Electron), Gedit, Alacritty |

---

## ✅ **1. Unit Tests**

### **1.1 Virtual Keyboard**
- **Keymap creation**: Verify US keymap loads, anonymous file created
- **Keysym → keycode**: Test ASCII, accented chars, symbols, Unicode fallback
- **Shift logic**: Validate level detection for `'A'` vs `'a'`
- **Chunking**: Confirm 10-char chunks with micro-delays

```rust
#[test]
fn test_keysym_to_keycode_ascii() {
    let vkbd = VirtualKeyboard::test_keymap();
    let (keycode, shift) = vkbd.resolve_keysym('A' as u32);
    assert!(shift);
    assert!(keycode > 0);
}
```

### **1.2 Portal/EIS**
- **D-Bus stubs**: Mock `RemoteDesktopProxy` responses
- **Restore token**: Save/load roundtrip
- **EIS handshake**: Simulate socket handshake + device discovery
- **Timeouts**: Ensure 50ms device discovery loop exits cleanly

### **1.3 KWin Fake Input**
- **Authentication mock**: Simulate KWin `authenticate()` true/false
- **Keycode cache**: Verify ASCII + common control chars cached
- **Unmappable char**: Ensure warning logged, no panic

---

## 🔗 **2. Integration Tests**

### **2.1 Virtual Keyboard (wlroots)**
- **Mock compositor**: Use `wayland-rs` test harness to simulate `zwp_virtual_keyboard_v1`
- **Keymap upload**: Verify fd sent, size matches
- **Event sequence**: Press → release → shift press/release → verify order

**Success criteria**: No protocol errors, all key events received by mock

### **2.2 Portal/EIS**
- **Mock portal**: Use `ashpd` test utils or `zbus` mock server
- **Session flow**: `create_session` → `select_devices` → `start` → `ConnectToEIS`
- **EIS device**: Simulate keyboard device advertisement
- **Text injection**: Send “Hello” → verify key events on mock EI socket

**Success criteria**: Full handshake completes in <300ms, text events emitted

### **2.3 KWin Fake Input**
- **Mock KWin D-Bus**: Use `zbus` test server to simulate `org.kde.kwin.FakeInput`
- **Auth flow**: Return `true` → proceed; `false` → return auth error
- **Key sequence**: Inject “Test” → verify 8 D-Bus calls (4 press + 4 release)

**Success criteria**: All D-Bus calls match expected sequence, no auth bypass

---

## 🌐 **3. End-to-End (Live) Tests**

Run on **real KDE Plasma + Hyprland sessions**.

### **3.1 Test Matrix**

| Method | KDE Plasma | Hyprland | Expected Success |
|--------|------------|----------|------------------|
| **AT‑SPI Insert** | ✅ Kate, Firefox | ✅ Firefox, Alacritty | High |
| **Virtual Keyboard** | ❌ (no protocol) | ✅ All apps | Medium-High |
| **Portal/EIS** | ✅ (with consent) | ✅ (if portal supports) | Medium |
| **KWin Fake Input** | ✅ (if authorized) | ❌ | Medium |

### **3.2 Test Cases**

#### **TC1: AT‑SPI + Virtual Keyboard Fallback (Hyprland)**
1. Launch Alacritty (no AT‑SPI)
2. Inject “ColdVox test”
3. **Verify**: Text appears, no errors

#### **TC2: Portal/EIS with Consent (KDE)**
1. Ensure portal not pre-authorized
2. Trigger injection → user sees consent dialog
3. Approve → inject “Portal test”
4. **Verify**: Text appears, restore token saved

#### **TC3: KWin Fake Input (KDE)**
1. Enable “Virtual Input” in System Settings
2. Inject into Konsole
3. **Verify**: Text appears, no AT‑SPI used

#### **TC4: Electron App (VS Code)**
1. Launch VS Code with `--enable-features=UseOzonePlatform --ozone-platform=wayland`
2. Focus editor
3. Inject long text (500 chars)
4. **Verify**: Full text inserted, no truncation

#### **TC5: Password Field Safety**
1. Focus password field (e.g., KWallet dialog)
2. Attempt injection
3. **Verify**: Skipped (no AT‑SPI), fallback used if allowed, no text leaked to logs

---

## ⚠️ **4. Failure & Edge Case Tests**

### **4.1 Timeout Handling**
- **Virtual Keyboard**: Kill compositor mid-injection → verify 50ms timeout
- **Portal**: Block D-Bus response → verify 100ms session timeout
- **KWin**: Return D-Bus error → verify graceful fallback

### **4.2 Resource Exhaustion**
- **Buffer size**: Inject 10k chars → verify chunking, no OOM
- **Keymap cache**: Inject 1k unique Unicode chars → verify cache growth bounded

### **4.3 Permission Denied**
- **KWin**: Disable “Virtual Input” → verify clear error: “Enable in System Settings”
- **Portal**: Revoke permission → verify new consent prompt

### **4.4 Focus Race**
1. Start injection
2. Switch window mid-process
3. **Verify**: Injection aborted, no text in wrong app

---

## 📊 **5. Observability & Telemetry Validation**

### **Metrics to Assert**
- `injection_attempt_total{method="atspi_insert", app="kate"} 1`
- `injection_success_total{method="vkbd", app="alacritty"} 1`
- `injection_duration_ms_bucket{method="portal_eis", le="100"} 1`
- `fallback_triggered{from="atspi_insert", to="vkbd"} 1`

### **Log Validation**
- **Success**: “Injected 12 chars via vkbd into Alacritty (role: terminal)”
- **Failure**: “KWin fake input not authorized. Enable in System Settings > Input Devices”
- **Privacy**: No raw text in logs (redacted as “[TEXT]”)

---

## 🛠️ **6. Setup & Pre-Flight Checks**

### **6.1 System Readiness Script**
```bash
#!/bin/bash
# check_vnext_ready.sh
echo "🔍 Checking AT-SPI..."
xprop -root | grep -q AT_SPI_BUS && echo "✅ AT-SPI bus active" || echo "❌ Missing AT-SPI"

echo "🔍 Checking input group..."
groups | grep -q input && echo "✅ User in input group" || echo "❌ Add user to input group"

echo "🔍 Checking portals..."
systemctl --user is-active xdg-desktop-portal && echo "✅ Portal active" || echo "❌ Start xdg-desktop-portal"
```

### **6.2 App Accessibility Validation**
- Use `accerciser` to confirm:
  - Kate exposes `EditableText`
  - VS Code editor has `STATE_EDITABLE` + `EditableText`
  - Konsole does **not** expose `EditableText` (expected)

---

## 📅 **7. Test Execution Plan**

| Phase | Tests | Owner | Tooling |
|------|-------|-------|---------|
| **CI (PR)** | Unit + Integration (mock) | Developer | `cargo test`, `zbus-test` |
| **Nightly** | Failure modes + Integration | QA Bot | `pytest`, custom Wayland VM |
| **Weekly** | E2E Live (KDE + Hyprland) | QA Engineer | Nobara test rig |
| **Release** | Full matrix + Permissions | Release Manager | Manual + automation |

---

## 🎯 **Success Criteria for vNext Release**

- ✅ **95%+ success rate** on AT‑SPI apps (Kate, Firefox)
- ✅ **80%+ success rate** on non-AT‑SPI apps (Alacritty, Konsole) via fallbacks
- ✅ **<200ms p95 latency** per injection
- ✅ **Zero crashes** on focus race or permission denial
- ✅ **Clear user guidance** for all failure modes

---

This test plan ensures your **vNext injection stack is robust, observable, and user-friendly** across the full spectrum of KDE Plasma and Hyprland environments. Ready for implementation in your CI/CD pipeline.