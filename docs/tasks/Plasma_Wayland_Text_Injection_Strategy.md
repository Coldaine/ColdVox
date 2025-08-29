# Text Injection Strategy — KDE Plasma (Wayland/KWin)

This document defines a comprehensive session-based text injection strategy for KDE Plasma on Wayland (KWin). It implements a buffered approach where transcriptions are accumulated during active dictation and injected as a batch after a configurable silence period, addressing the unique challenges of Wayland's security model while maintaining reliability and user experience.

## Goals

- Inject accumulated transcriptions into the currently focused application after silence detection.
- Buffer multiple transcriptions during active speech for natural dictation flow.
- Attempt focus detection to verify text field presence before injection.
- Prefer user‑space, permission‑light methods; avoid root where feasible.
- Provide robust fallbacks when the preferred path is unavailable.
- Maintain session coherence with configurable silence timeouts.
- Instrument choices and timings to refine ordering per environment.

## Session-Based Injection Architecture

### Core Concept

Unlike immediate injection per transcription, this strategy implements a **dictation session** model where:
1. Transcriptions are buffered during active speech
2. A silence timer monitors for pauses in dictation
3. After a configurable timeout (default 1500ms), the complete buffer is injected
4. Focus detection attempts to verify a text field is active before injection

### Benefits

- **Natural dictation flow**: Users can speak multiple sentences without interruption
- **Self-correction window**: Pause and resume before text is committed
- **Reduced injection overhead**: Single batch operation vs many small injections
- **Application compatibility**: Many apps handle batch text better than character streams
- **Coherent text blocks**: Related thoughts stay together

### Session State Machine

```
IDLE → BUFFERING → WAITING_FOR_SILENCE → READY_TO_INJECT → IDLE
         ↑              ↓
         └──────────────┘ (new transcription resets timer)
```

## Summary of Viable Methods (KDE/Wayland)

1. IME Injection (IBus or Fcitx5)
   - Commit text via the active input method engine.
   - Compositor‑agnostic and designed for text entry.
   - Requires user to select your IME engine when dictating.
2. AT‑SPI2 Editable Text
   - Insert/set text on focused widgets implementing the `EditableText` interface.
   - Works across many Qt/GTK apps, user‑space only, no root.
   - Not universal; some apps/widgets don’t expose `EditableText`.
3. ydotool (uinput)
   - System‑wide synthetic keystrokes via a background daemon.
   - Broad coverage; requires uinput permissions/capabilities; user‑opt‑in.
4. Clipboard (wl‑clipboard)
   - Set clipboard contents reliably; safe last‑resort.
   - Still needs an input path to trigger paste (e.g., IME/AT‑SPI2 action/ydotool).
5. X11/XWayland niche fallback
   - Useful only for legacy X11 apps under XWayland; not applicable to native Wayland windows.

## Methods Not Applicable as Default on KDE/KWin

- Wayland Virtual Keyboard protocol (`zwp_virtual_keyboard_manager_v1`)
  - KWin does not implement the wlroots virtual keyboard protocol.
- `wtype`
  - Designed for wlroots compositors (Sway/Hyprland/River); not for KWin.

## Recommended Strategy Order (KDE Plasma/KWin - Session-Based)

1. AT‑SPI2 Editable Text (with focus verification)
2. Clipboard + AT‑SPI2 Paste Action
3. ydotool (uinput) — user‑enabled fallback
4. IME (IBus or Fcitx5) — less suitable for batch
5. X11/XWayland path for legacy apps only

Rationale for Session-Based Priority:
- AT‑SPI2 is prioritized first as it can verify focus state and handle batch text well.
- Clipboard with paste action is reliable for batch text and preserves formatting.
- ydotool works well with batch text when enabled, good coverage.
- IMEs moved lower as they're better suited for character-by-character input than batch.
- XWayland remains a niche case for legacy applications.

### Focus Detection Strategy

Due to Wayland's security model, focus detection is challenging but attempted through:
1. **AT-SPI2 accessibility bus**: Can query focused element's EditableText interface
2. **Best-effort approach**: When detection fails, optionally inject anyway (configurable)
3. **User feedback**: Visual/audio cue when focus state is uncertain

---

## Architecture Overview

The session-based injection system consists of three main components: session management for buffering transcriptions, focus detection for target validation, and the injection manager with pluggable backends.

### Dependencies (Modern Rust Stack)

```toml
# Core dependencies
zbus = { version = "4", features = ["tokio"] }  # Type-safe D-Bus for AT-SPI2
tokio = { version = "1", features = ["time", "process"] }  # Async runtime
rtrb = "0.3"  # Lock-free ring buffers for IPC
anyhow = "1"  # Error handling
tracing = "0.1"  # Structured logging

# Optional injector dependencies  
wayland-client = { version = "0.31", optional = true }  # Direct Wayland protocol
x11rb = { version = "0.13", optional = true }  # X11/XWayland support
```

### Core Components

```rust
// crates/app/src/text_injection/mod.rs
pub trait TextInjector {
    fn name(&self) -> &'static str;
    fn inject_text(&self, text: &str) -> anyhow::Result<()>;
    fn is_available(&self) -> bool;
    fn supports_batch(&self) -> bool; // Some injectors work better with batch text
}

// Session management for buffered injection
pub struct InjectionSession {
    buffer: Vec<String>,              // Accumulated transcriptions
    last_transcription: Instant,      // For silence detection
    silence_timeout: Duration,        // Configurable, default 1500ms
    state: SessionState,
    join_separator: String,           // How to join buffered text
}

pub enum SessionState {
    Idle,
    Buffering,                        // Actively receiving transcriptions
    WaitingForSilence,               // Timer running, no new input
    ReadyToInject,                   // Silence period complete
}

// Focus detection for target validation
pub struct FocusDetector {
    atspi_conn: Option<AccessibilityConnection>,
}

pub enum FocusStatus {
    TextFieldConfirmed,              // Definitely a text input
    NonTextElement,                  // Focused but not editable
    Unknown,                         // Can't determine (common on Wayland)
}

// Manager orchestrating the injection pipeline
pub struct InjectionManager {
    injectors: Vec<Box<dyn TextInjector + Send + Sync>>,
    session: Arc<Mutex<InjectionSession>>,
    focus_detector: FocusDetector,
}
```

### Injectors (Phased Implementation)

**Phase 1 (MVP - Week 1-2):**
- `ClipboardInjector` - Simple, reliable batch text via `wl-copy`
- `YdotoolInjector` - Fallback for paste triggering (opt-in)

**Phase 2 (Enhanced - Week 3-4):**
- `AtspiInjector` - Primary method using `zbus` for type-safe D-Bus
- Focus detection via accessibility APIs

**Phase 3 (Advanced - Week 5+):**
- `ImeInjector` - Specialized workflows (lower priority for batch)
- `X11Injector` - XWayland support if needed

All injectors expose `is_available()`, `supports_batch()`, and `estimated_latency()` methods.

### Injection Processor

The injection processor runs in a dedicated thread/task and manages the session lifecycle:

```rust
// crates/app/src/text_injection/processor.rs
pub struct InjectionProcessor {
    session: InjectionSession,
    manager: InjectionManager,
    focus_detector: FocusDetector,
    rx: mpsc::Receiver<String>,      // From STT processor
    config: InjectionConfig,
    check_interval: Duration,         // How often to check silence (100ms default)
}

impl InjectionProcessor {
    pub async fn run(mut self) -> Result<()> {
        let mut interval = time::interval(self.check_interval);
        
        loop {
            tokio::select! {
                // New transcription from STT
                Some(text) = self.rx.recv() => {
                    self.session.add_transcription(text);
                    info!("Buffered transcription, {} items in session", 
                          self.session.buffer.len());
                }
                
                // Periodic silence check
                _ = interval.tick() => {
                    if self.session.should_inject() {
                        self.try_inject().await?;
                    }
                }
            }
        }
    }
    
    async fn try_inject(&mut self) -> Result<()> {
        // Take buffered text
        let text = self.session.take_buffer();
        if text.is_empty() { return Ok(()); }
        
        // Attempt focus detection
        match self.focus_detector.is_text_field_focused() {
            FocusStatus::NonTextElement if !self.config.inject_on_non_text => {
                warn!("Focus not on text field, skipping injection");
                return Ok(());
            }
            FocusStatus::Unknown if !self.config.inject_on_unknown_focus => {
                warn!("Cannot determine focus, skipping injection");
                return Ok(());
            }
            _ => {} // Proceed with injection
        }
        
        // Try injectors in order
        for injector in &self.manager.injectors {
            if !injector.is_available() { continue; }
            
            match injector.inject_text(&text) {
                Ok(()) => {
                    info!("Successfully injected via {}", injector.name());
                    return Ok(());
                }
                Err(e) => {
                    debug!("Injection failed via {}: {}", injector.name(), e);
                }
            }
        }
        
        error!("All injection methods failed");
        Err(anyhow!("No working injection method"))
    }
}
```

### Selection Policy

1. Detect compositor/environment (Wayland vs X11, KWin vs wlroots).
2. Build injector list optimized for batch text on KDE/KWin.
3. Attempt focus detection before injection.
4. Try injectors in order until success.
5. Log failures for diagnostics without blocking the pipeline.

---

## Environment & Dependency Detection

Prefer lightweight checks; avoid spawning processes on every injection.

- Wayland/KDE/KWin
  - `WAYLAND_DISPLAY` present → Wayland session.
  - `XDG_CURRENT_DESKTOP` contains `KDE` or `KDE Plasma`.
  - `KDE_FULL_SESSION=1` or D‑Bus name `org.kde.KWin` present.
- IME detection
  - Env vars: `QT_IM_MODULE`, `GTK_IM_MODULE` (e.g., `ibus`, `fcitx`, `fcitx5`).
  - D‑Bus names: `org.freedesktop.IBus` (session bus), `org.fcitx.Fcitx5`.
  - Optional process presence: `ibus-daemon`, `fcitx5`.
- AT‑SPI2
  - D‑Bus accessibility bus present; test `atspi::AccessibilityConnection::open()`.
- ydotool
  - Binary present in PATH; `ydotool --version` (once, at startup).
  - `ydotoold` running (pid or systemd service active).
- Clipboard
  - `wl-copy`/`wl-paste` available in PATH.
- XWayland niche
  - `XDG_SESSION_TYPE` = `x11` for entire session, or per‑window detection (advanced) for XWayland windows.

---

## Implementation Risks and Mitigation

### High Risk Areas

1. **AT-SPI2 Application Support**
   - **Risk**: Not all applications expose proper EditableText interfaces
   - **Mitigation**: Comprehensive fallback chain, maintain application compatibility matrix
   - **Testing**: Firefox, LibreOffice, Kate, VS Code, Terminal emulators

2. **Wayland Security Model**
   - **Risk**: Compositor may block synthetic input methods
   - **Mitigation**: Multiple injection strategies, user configuration for preferred methods
   - **Fallback**: Always maintain clipboard as last resort

### Medium Risk Areas

1. **Focus Detection Accuracy**
   - **Risk**: Wayland limits focus information access
   - **Mitigation**: Best-effort detection, configurable behavior for unknown focus
   - **User Control**: Visual/audio feedback when focus uncertain

2. **Session Timing Optimization**
   - **Risk**: Optimal silence timeout varies by user speech patterns
   - **Mitigation**: Configurable timeouts (500ms-5000ms), future ML-based adaptation
   - **Default**: Conservative 1500ms works for most users

### Low Risk Areas

1. **Session State Management**: Well-understood state machine pattern
2. **zbus Integration**: Mature library with extensive documentation
3. **Clipboard Operations**: Standard `wl-copy` tool is reliable

## Success Metrics

### Phase 1 (MVP) Success Criteria
- ✅ Natural dictation flow without interruption between sentences
- ✅ 95%+ injection success rate with clipboard + manual paste
- ✅ Configurable silence timeouts (500ms - 5000ms)
- ✅ Session state visible in UI/telemetry
- ✅ Buffer management prevents memory issues

### Phase 2 (Enhanced) Success Criteria
- ✅ AT-SPI2 injection working in 80%+ of common applications
- ✅ Focus detection prevents 90%+ of accidental injections
- ✅ Automatic fallback chain completes within 500ms
- ✅ Per-application injection history tracked

### Phase 3 (Advanced) Success Criteria  
- ✅ Sub-200ms injection latency after silence detection
- ✅ Application-specific injection profiles
- ✅ IME integration for specialized workflows
- ✅ User satisfaction score >4.5/5

## Detailed Injector Designs

### 4) IME Injector (Lower Priority for Batch)

Approach: IME engines are designed for character-by-character input but can commit batch text.

- IBus/Fcitx5:
  - Better suited for streaming transcription than batch
  - Requires user to switch input method during dictation
  - Can commit full strings but less natural for large batches
  
- Why lower priority for session-based:
  - Users expect IMEs for character input, not paragraph injection
  - Switching IME for dictation adds friction
  - Other methods handle batch text more naturally
  
- Still useful for:
  - Users who prefer IME workflow
  - Applications that only accept IME input properly
  - Future streaming mode implementation

### 1) AT‑SPI2 Injector (Primary for Batch - Phase 2)

Approach: Modern implementation using `zbus` for type-safe D-Bus communication.

```rust
use zbus::{proxy, Connection};
use anyhow::Result;

#[proxy(
    interface = "org.a11y.atspi.EditableText",
    default_service = "org.a11y.atspi.Registry",
    default_path = "/org/a11y/atspi/accessible/root"
)]
trait EditableText {
    async fn insert_text(&self, text: &str, position: i32) -> Result<bool>;
    async fn delete_text(&self, start: i32, end: i32) -> Result<bool>;
}

#[proxy(
    interface = "org.a11y.atspi.Component",
    default_service = "org.a11y.atspi.Registry"
)]
trait Component {
    async fn grab_focus(&self) -> Result<bool>;
    async fn get_extents(&self, coord_type: u32) -> Result<(i32, i32, i32, i32)>;
}

pub struct AtspiInjector {
    connection: Connection,
    metrics: Arc<InjectionMetrics>,
}

impl AtspiInjector {
    pub async fn new() -> Result<Self> {
        let connection = Connection::session().await?;
        Ok(Self { 
            connection,
            metrics: Arc::new(InjectionMetrics::default()),
        })
    }
    
    async fn inject_batch(&self, text: &str) -> Result<()> {
        // Get focused element via AT-SPI2
        let focused = self.get_focused_element().await?;
        
        // Try direct text insertion
        let proxy = EditableTextProxy::new(&self.connection, focused).await?;
        match proxy.insert_text(text, -1).await {
            Ok(true) => {
                self.metrics.record_success("atspi_direct");
                return Ok(());
            }
            _ => {}
        }
        
        // Fallback to paste action
        self.trigger_paste_action(focused).await
    }
    
    async fn detect_focus(&self) -> FocusStatus {
        // Query accessibility tree for focused element type
        match self.get_focused_element().await {
            Ok(path) => self.check_element_type(path).await,
            Err(_) => FocusStatus::Unknown,
        }
    }
}

- Pros: No elevated permissions; excellent for batch text; can verify focus state.
- Cons: Not universal; some apps don't expose necessary interfaces.

### 2) Clipboard Injector (Batch Fallback)

Approach: Set clipboard to batch text, then trigger paste via AT‑SPI2 or ydotool.

- Tools: `wl-copy` to set clipboard on Wayland.
- Batch optimization:
  - Entire session buffer set as single clipboard operation
  - Works with AT‑SPI2 paste action or ydotool key simulation
  - Preserves text formatting and handles multi-line content well
  
- Implementation:
  ```rust
  impl ClipboardInjector {
      fn inject_batch(&self, text: &str) -> Result<()> {
          // Save current clipboard if configured
          let saved = if self.config.restore_clipboard {
              Some(self.get_clipboard()?)
          } else { None };
          
          // Set clipboard to batch text
          self.set_clipboard(text)?;
          
          // Trigger paste (relies on AT-SPI2 or ydotool)
          // Note: This injector typically used in conjunction with others
          
          // Restore after delay if configured
          if let Some(saved_text) = saved {
              thread::spawn(move || {
                  thread::sleep(Duration::from_millis(500));
                  let _ = self.set_clipboard(&saved_text);
              });
          }
          
          Ok(())
      }
  }
  ```

- Pros: Reliable for batch text; preserves formatting; user‑space operation.
- Cons: Requires paste trigger from another injector; modifies user clipboard.

### 3) ydotool Injector (Opt-in Fallback)

Approach: Type batch text or trigger paste via synthetic keystrokes.

- Setup:
  - Requires daemon and user opt-in (security consideration)
  - Auto-detect availability at startup
- Batch handling:
  - Can type entire text block: `ydotool type --file -`
  - Or trigger paste: `ydotool key ctrl+v` after clipboard set
- Pros: Universal coverage; works with batch text; system-wide.
- Cons: Requires elevated permissions; security implications; must be explicitly enabled.

### 5) X11/XWayland Path (Legacy Only)

Approach: Only for XWayland applications, not native Wayland.

- Very limited use case in modern KDE Plasma
- Not worth implementing unless specific legacy app requires it
- Consider only if user has specific X11 application needs

---

## Selection Algorithm (Session-Based KDE/KWin)

Building the injector chain for batch text:

```rust
fn build_session_injector_chain(env: &EnvProbe, cfg: &InjectionConfig) -> Vec<Box<dyn TextInjector>> {
    let mut injectors: Vec<Box<dyn TextInjector>> = Vec::new();

    // Primary: AT-SPI2 for direct injection and focus detection
    if cfg.feature_atspi {
        injectors.push(Box::new(AtspiInjector::new()));
    }

    // Clipboard + paste combo (requires AT-SPI2 or ydotool to trigger)
    if cfg.feature_wl_clipboard {
        let clipboard = Box::new(ClipboardInjector::new(cfg.restore_clipboard));
        
        // Try clipboard + AT-SPI2 paste action combo
        if cfg.feature_atspi {
            injectors.push(Box::new(ClipboardWithAtspiPaste::new(clipboard.clone())));
        }
        
        // Try clipboard + ydotool paste combo if allowed
        if cfg.feature_ydotool && cfg.allow_ydotool {
            injectors.push(Box::new(ClipboardWithYdotoolPaste::new(clipboard.clone())));
        }
    }

    // Standalone ydotool if allowed
    if cfg.feature_ydotool && cfg.allow_ydotool {
        injectors.push(Box::new(YdotoolInjector::new()));
    }

    // IME as lower priority for batch
    if cfg.enable_ime {
        if env.ime_is_ibus { injectors.push(Box::new(ImeIbus::new())); }
        if env.ime_is_fcitx5 { injectors.push(Box::new(ImeFcitx5::new())); }
    }

    // Filter to only available injectors
    injectors.into_iter()
        .filter(|i| i.is_available() && i.supports_batch())
        .collect()
}
```

Session execution flow:
1. Accumulate transcriptions until silence timeout
2. Check focus state (best effort)
3. Try injectors in order with full batch text
4. Log success/failure for diagnostics
5. Clear buffer and reset session state

---

## Installation & Setup (Nobara/KDE)

Fedora/Nobara (DNF):

```bash
# IME frameworks (choose one; IBus is default on many setups)
sudo dnf install ibus ibus-gtk ibus-qt
sudo dnf install fcitx5 fcitx5-qt fcitx5-gtk fcitx5-configtool

# Accessibility stack
sudo dnf install at-spi2-core at-spi2-atk

# Clipboard tools
sudo dnf install wl-clipboard

# ydotool (optional fallback)
sudo dnf install ydotool
sudo systemctl enable --now ydotoold
```

Environment (if needed):

```bash
# If switching IME
export QT_IM_MODULE=ibus   # or fcitx5
export GTK_IM_MODULE=ibus  # or fcitx
```

Arch (pacman):

```bash
sudo pacman -S ibus fcitx5 fcitx5-qt fcitx5-gtk at-spi2-core wl-clipboard ydotool
sudo systemctl enable --now ydotoold
```

Debian/Ubuntu (apt):

```bash
sudo apt install ibus fcitx5 fcitx5-frontend-qt fcitx5-frontend-gtk at-spi2-core wl-clipboard ydotool
sudo systemctl enable --now ydotoold || true
```

ydotool permissions (if daemon not used):

```bash
# Example: grant capabilities to ydotool to access /dev/uinput without root
sudo setcap cap_u​input,cap_sys_admin+ep /usr/bin/ydotool
```

---

## Performance & Reliability

- **Session buffering**: Reduces injection frequency by batching transcriptions
- **Single injection attempt**: One batch operation instead of multiple character/word injections
- **Focus check timing**: Performed just before injection, not during buffering
- **Failure handling**: Log but don't retry failed injections to avoid blocking
- **Telemetry**: Track buffer size, injection success rate, and which injectors work

---

## Security & Permissions

- IME and AT‑SPI2 operate in user space; no elevated privileges.
- ydotool uses uinput; requires daemon or capabilities — keep behind an explicit consent flag and document implications.
- Clipboard is safe; restore clipboard if modified unless in a user‑approved streaming mode.

---

## Error Handling & Troubleshooting

Common issues and remedies:

- “No injectors available”
  - Verify Wayland session: `echo $WAYLAND_DISPLAY`.
  - Ensure IME running: `pgrep -a ibus-daemon` or `pgrep -a fcitx5`.
  - Confirm AT‑SPI: `gsettings get org.gnome.desktop.interface toolkit-accessibility` (on KDE, ensure at-spi2-core is installed; accessibility generally on by default).
  - Check `wl-copy` presence: `which wl-copy`.
  - ydotool enabled? `systemctl --user status ydotoold` (or system scope depending on packaging).

- “AT‑SPI2 injection does nothing”
  - Target widget may not implement `EditableText` or may be read‑only.
  - Focus may be on a container, not the text field — try clicking into the field first.

- “IME commits are not appearing”
  - Ensure your IME is the active input method in the system tray/selector.
  - Validate D‑Bus calls succeed (enable debug logs) and that the engine is registered.

- “ydotool permission denied”
  - Ensure `ydotoold` is running; verify `/dev/uinput` permissions; consider `setcap` or group rules.

---

## Integration with ColdVox Pipeline

### Pipeline Integration

```rust
// In main pipeline setup
let (injection_tx, injection_rx) = mpsc::channel(32);

// STT processor sends transcriptions to injection
stt_processor.set_output_channel(injection_tx.clone());

// Create injection processor with session management
let injection_processor = InjectionProcessor::new(
    injection_rx,
    injection_config,
    focus_detector,
    telemetry.clone(),
);

// Run in dedicated task/thread
tokio::spawn(async move {
    if let Err(e) = injection_processor.run().await {
        error!("Injection processor failed: {}", e);
    }
});
```

### UI Controls

- **Silence timeout slider**: Adjust wait time (500ms - 5000ms)
- **Focus check toggle**: Enable/disable focus detection
- **ydotool permission**: Explicit opt-in with security warning
- **Session status indicator**: Show buffering/waiting/injecting state
- **Buffer preview**: Optional display of pending text

### Telemetry Integration

Extend existing `PipelineMetrics`:
```rust
pub struct InjectionMetrics {
    pub session_state: AtomicU8,           // Current session state
    pub buffer_size: AtomicUsize,          // Current buffer character count
    pub transcription_count: AtomicUsize,  // Transcriptions in buffer
    pub last_injection_ms: AtomicU64,      // Time since last injection
    pub successful_injections: AtomicU64,
    pub failed_injections: AtomicU64,
    pub injector_used: RwLock<String>,     // Last successful injector name
}
```

---

## Configuration Model

```toml
# Phase 1: MVP Configuration (Simple, Working Defaults)
[text_injection]
silence_timeout_ms = 1500         # Sweet spot for most users
buffer_join_separator = " "       # Space between transcriptions
max_buffer_size = 5000            # Reasonable limit for dictation
inject_on_unknown_focus = true   # Best-effort injection

# Phase 2: Enhanced Configuration
[text_injection.focus]
enabled = true                    # Try focus detection
allow_non_text = false            # Strict mode: only inject in text fields
feedback_on_uncertain = true      # Notify user when focus unclear

# Phase 2+: Injector Chain
[text_injection.methods]
primary = "atspi"                 # First choice when available
fallback_order = ["clipboard", "ydotool"]
max_retry_ms = 500                # Total time to try all methods

# Phase 3: Advanced Configuration  
[text_injection.advanced]
allow_ydotool = false             # Explicit security opt-in required
restore_clipboard = true          # Preserve user's clipboard
adaptive_timing = false           # ML-based timeout adjustment
per_app_profiles = false          # Remember what works per application

# Feature flags (compile-time)
[features]
default = ["clipboard", "atspi"]
full = ["clipboard", "atspi", "ydotool", "ime", "x11"]
minimal = ["clipboard"]           # Absolute minimum for MVP
```

Runtime overrides:
- Environment variables: `COLDVOX_SILENCE_TIMEOUT_MS`, `COLDVOX_ALLOW_YDOTOOL`
- CLI flags: `--silence-timeout`, `--allow-ydotool`, `--no-focus-check`

---

## Session Handling Edge Cases

### Buffer Management

1. **Rapid continuous speech**: Timer resets on each new transcription
2. **Very long dictation**: Max buffer size triggers injection even without silence
3. **Empty transcriptions**: Filtered out, don't affect session state
4. **Punctuation-only results**: Appended to buffer like normal text
5. **Multiple speakers**: Treated as single session (future: speaker separation)

### Focus Changes

1. **User switches apps during buffering**: Focus checked at injection time
2. **App crashes during session**: Injection fails gracefully, buffer cleared
3. **Screen lock during dictation**: Session paused/cleared based on config
4. **Virtual desktop switch**: Treated like app switch

### Error Recovery

1. **All injectors fail**: Log error, optionally notify user, clear buffer
2. **Partial injection**: Not possible with batch approach (all or nothing)
3. **Clipboard conflicts**: Save/restore with timeout to prevent deadlock
4. **AT-SPI2 timeout**: Move to next injector without blocking

### User Interactions

1. **Manual injection trigger**: Hotkey to force injection before timeout
2. **Cancel current session**: Hotkey to clear buffer without injecting
3. **Pause/resume dictation**: Maintain buffer but pause timeout
4. **Preview before injection**: Optional UI showing pending text

---

## Implementation Roadmap (Revised)

### Phase 1: MVP - Session Management (Week 1-2)
**Goal**: Deliver immediate value with reliable batch text injection

- ✅ Implement session buffer with configurable silence detection
- ✅ Basic clipboard injector using `wl-copy`
- ✅ Integration with existing STT pipeline
- ✅ Session state visualization in TUI dashboard
- ✅ Configuration system for timeouts and buffer limits
- **Deliverable**: Working dictation with manual paste

### Phase 2: Enhanced Injection (Week 3-4)
**Goal**: Automatic injection with focus awareness

- ✅ AT-SPI2 injector using `zbus` for type-safe D-Bus
- ✅ Focus detection to prevent accidental injections
- ✅ Automatic fallback chain (AT-SPI2 → Clipboard → ydotool)
- ✅ Per-application success tracking
- ✅ User feedback for injection status
- **Deliverable**: Hands-free dictation in most applications

### Phase 3: Polish & Advanced Features (Week 5+)
**Goal**: Production-ready with advanced capabilities

- ✅ ydotool integration with security consent flow
- ✅ IME support for specialized workflows
- ✅ Application-specific injection profiles
- ✅ Voice commands for session control
- ✅ ML-based silence timeout optimization
- **Deliverable**: Polished, configurable dictation system

### Future Considerations
- xdg-desktop-portal RemoteDesktop API when available
- Machine learning for optimal timeout detection
- Gesture/hotkey triggered injection override

---

## Appendix A — Rust Implementation Sketches

### Session Management

```rust
use std::time::{Duration, Instant};

impl InjectionSession {
    pub fn new(config: SessionConfig) -> Self {
        Self {
            buffer: Vec::new(),
            last_transcription: Instant::now(),
            silence_timeout: Duration::from_millis(config.silence_timeout_ms),
            state: SessionState::Idle,
            join_separator: config.join_separator,
            max_buffer_size: config.max_buffer_size,
        }
    }
    
    pub fn add_transcription(&mut self, text: String) {
        // Filter empty transcriptions
        if text.trim().is_empty() {
            return;
        }
        
        self.buffer.push(text);
        self.last_transcription = Instant::now();
        self.state = SessionState::Buffering;
        
        // Force injection if buffer too large
        if self.total_chars() > self.max_buffer_size {
            self.state = SessionState::ReadyToInject;
        }
    }
    
    pub fn should_inject(&mut self) -> bool {
        match self.state {
            SessionState::Buffering => {
                if self.last_transcription.elapsed() >= self.silence_timeout {
                    self.state = SessionState::ReadyToInject;
                    true
                } else {
                    false
                }
            }
            SessionState::ReadyToInject => true,
            _ => false,
        }
    }
    
    pub fn take_buffer(&mut self) -> String {
        let text = self.buffer.join(&self.join_separator);
        self.buffer.clear();
        self.state = SessionState::Idle;
        text
    }
    
    fn total_chars(&self) -> usize {
        self.buffer.iter().map(|s| s.len()).sum()
    }
}
```

### Focus Detection with AT-SPI2

```rust
use atspi::{AccessibilityConnection, InterfaceSet};

impl FocusDetector {
    pub fn new() -> Self {
        let conn = AccessibilityConnection::open().ok();
        Self { atspi_conn: conn }
    }
    
    pub fn is_text_field_focused(&self) -> FocusStatus {
        let Some(conn) = &self.atspi_conn else {
            return FocusStatus::Unknown;
        };
        
        let Ok(cache) = conn.cache() else {
            return FocusStatus::Unknown;
        };
        
        let Ok(focus) = cache.focus() else {
            return FocusStatus::Unknown;
        };
        
        // Check if focused element is editable text
        if focus.interfaces().contains(InterfaceSet::EDITABLE_TEXT) {
            FocusStatus::TextFieldConfirmed
        } else if focus.interfaces().contains(InterfaceSet::TEXT) {
            // Read-only text field
            FocusStatus::NonTextElement
        } else {
            FocusStatus::NonTextElement
        }
    }
}
```

### Combined Clipboard + AT-SPI2 Paste

```rust
struct ClipboardWithAtspiPaste {
    clipboard: ClipboardInjector,
    atspi: AtspiInjector,
}

impl TextInjector for ClipboardWithAtspiPaste {
    fn inject_text(&self, text: &str) -> anyhow::Result<()> {
        // Set clipboard
        self.clipboard.set_clipboard(text)?;
        
        // Trigger paste via AT-SPI2
        let conn = self.atspi.conn.as_ref()?;
        let focus = conn.cache()?.focus()?;
        
        if focus.interfaces().contains(InterfaceSet::ACTION) {
            // Look for paste action
            let actions = focus.get_actions()?;
            for action in actions {
                if action.name.to_lowercase().contains("paste") {
                    focus.do_action(&action.name)?;
                    return Ok(());
                }
            }
        }
        
        Err(anyhow!("No paste action available"))
    }
    
    fn supports_batch(&self) -> bool { true }
}
```

Clipboard via `wl-copy`:

```rust
use std::process::{Command, Stdio};

fn wl_copy(text: &str) -> anyhow::Result<()> {
    let mut child = Command::new("wl-copy")
        .stdin(Stdio::piped())
        .spawn()?;
    use std::io::Write;
    child.stdin.as_mut().unwrap().write_all(text.as_bytes())?;
    let status = child.wait()?;
    anyhow::ensure!(status.success(), "wl-copy failed");
    Ok(())
}
```

ydotool type:

```rust
fn ydotool_type(text: &str) -> anyhow::Result<()> {
    // ydotool type --file -
    let mut child = std::process::Command::new("ydotool")
        .arg("type").arg("--file").arg("-")
        .stdin(std::process::Stdio::piped())
        .spawn()?;
    use std::io::Write;
    child.stdin.as_mut().unwrap().write_all(text.as_bytes())?;
    let status = child.wait()?;
    anyhow::ensure!(status.success(), "ydotool type failed");
    Ok(())
}
```

---

## Appendix B — Testing Strategy

### Unit Tests
- Session buffer management and state transitions
- Silence timeout detection accuracy
- Focus detection mock scenarios
- Individual injector availability checks

### Integration Tests
- Full pipeline from STT to injection
- Fallback chain behavior when primary fails
- Clipboard save/restore cycles
- Thread safety of session management

### Manual Testing Scenarios
1. **Basic dictation**: Single sentence with natural pause
2. **Multi-sentence**: Paragraph with multiple pauses
3. **Rapid speech**: No pauses between sentences
4. **App switching**: Change focus during buffering
5. **Mixed content**: Numbers, punctuation, special characters
6. **Error conditions**: No text field focused, all injectors fail

### Performance Benchmarks
- Session buffer memory usage with large text
- Injection latency per method
- CPU usage during silence detection
- Thread synchronization overhead

---

## Summary

This session-based text injection strategy for KDE Plasma on Wayland provides:

1. **Natural dictation flow** through buffered transcriptions with silence detection
2. **Reliable injection** via multiple fallback methods optimized for batch text
3. **Focus awareness** through AT-SPI2 accessibility APIs where possible
4. **Security consciousness** with opt-in for privileged operations
5. **Integration ready** architecture that fits ColdVox's existing pipeline

The key innovation is treating dictation as discrete sessions rather than continuous streams, allowing users to speak naturally while maintaining reliable text injection even in Wayland's restricted security environment.

Next steps:
1. Implement core session management with configurable timeouts
2. Add AT-SPI2 injector with focus detection
3. Create clipboard fallback with paste triggering
4. Integrate with existing STT pipeline
5. Add telemetry and user controls

