# Phase 2+ — Adaptive Text Injection for KDE/Wayland (Parallel Modules + Strategy Manager)

This plan upgrades ColdVox’s session-based text injection for KDE Plasma/Wayland with modular injectors, an adaptive selection/fallback manager, and context-aware decisioning. It stays pragmatic (no privileged paths by default), but leaves room for opt-in “power” methods and Phase 3 IME integration.

Goals (KDE/Wayland first):
- Deliver reliable batch dictation into common apps with bounded latency.
- Use context signals (AT‑SPI2) to reduce mis-injection.
- Run multiple injector modules independently; choose the best available at runtime.
- Adapt when a method fails: skip it temporarily (cooldown) and prefer success-proven methods per app.
- Keep everything behind feature flags; default off unless explicitly enabled.

Out of scope for this phase:
- Full IME integration (Fcitx5/IBus), per-app ML policies. These are Phase 3.


## Architecture overview

- Session buffer (existing): collects STT text until silence timeout.
- Parallel injector modules (independent):
  - AtspiInjector (direct insert or Paste action)
  - ClipboardInjector (Wayland-native clipboard)
  - ClipboardWithAtspiPaste (composition helper)
  - YdotoolInjector (opt-in fallback)
  - [Phase 3] ImeInjector (text-input v3 + input-method v2)
  - [Exploratory] PortalEisInjector (ashpd/libei if KDE exposes it)
  - [Experimental] VkmInjector (zwp_virtual_keyboard_manager_v1; likely unauthorized on KWin)
- Strategy Manager:
  - Builds an ordered chain based on features, environment, and recent success per app.
  - Executes 1 method at a time (no duplicate pastes), but can probe availability in parallel.
  - Applies timeouts, overall latency budget, and adaptive cooldown/backoff.
  - Records telemetry per app+method (success, latency, errors).
- Focus Tracker (AT‑SPI2):
  - Provides FocusStatus: ConfirmedEditable | NonEditable | Unknown.
  - Also extracts app identifiers (class/title) to key the heuristic cache.


## Core contracts

Trait (crate-internal API):

```rust
// Option 1: Async trait (recommended for tokio-based injectors)
pub trait TextInjector: Send + Sync {
    fn name(&self) -> &'static str;
    async fn is_available(&self) -> bool;        // fast check
    fn supports_batch(&self) -> bool { true }
    async fn inject(&self, text: &str) -> anyhow::Result<()>;  // respects per-call timeout
}

// Option 2: Sync trait with internal async handling
pub trait TextInjector: Send + Sync {
    fn name(&self) -> &'static str;
    fn is_available(&self) -> bool;        // fast check
    fn supports_batch(&self) -> bool { true }
    fn inject(&self, text: &str) -> anyhow::Result<()> {
        // Use a runtime handle or blocking bridge
        self.inject_blocking(text)
    }
    fn inject_blocking(&self, text: &str) -> anyhow::Result<()>;
}

// Error types for better handling
#[derive(Debug, thiserror::Error)]
pub enum InjectionError {
    #[error("No editable focus found")]
    NoEditableFocus,
    
    #[error("Method not available: {0}")]
    MethodNotAvailable(String),
    
    #[error("Timeout after {0}ms")]
    Timeout(u64),
    
    #[error("All methods failed: {0}")]
    AllMethodsFailed(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
}
```

Strategy inputs/outputs:
- Input: UTF‑8 text (already normalized); empty/whitespace → no-op.
- Context: FocusStatus + (app_class, window_title?) for cache keying.
- Output: Ok on first successful injector; otherwise error with reasons.
- Errors: timeouts, permissions, non-editable focus, tool missing.


## Injector modules (parallel, feature-gated)

1) AT‑SPI2 Injector (Primary on KDE)
- Feature: `text-injection-atspi`
- Deps: `atspi = { version = "0.28", features = ["connection", "proxies"], optional = true }`
- Behavior:
  - Resolve focused object.
  - If EditableText: try insert_text/set_text_contents.
  - Else if Action available: look for localized "paste" action and perform it.
  - Guard each D‑Bus call with 150–300 ms timeout.
- Availability probe: session bus + at-spi registry reachable.
- Pros: User-space, focus-aware; good for batch text.
- Cons: Not universal; some widgets lack EditableText/Action.

Implementation example:
```rust
// atspi_injector.rs
use atspi::{proxy::accessible::AccessibleProxy, Interface, Action};

pub struct AtspiInjector {
    connection: Arc<AccessibilityConnection>,
}

impl AtspiInjector {
    async fn find_paste_action(object: &AccessibleProxy<'_>) -> Option<usize> {
        if let Ok(actions) = object.get_actions().await {
            for (i, action) in actions.iter().enumerate() {
                let name = action.get_name(i as i32).await.unwrap_or_default();
                let lower = name.to_lowercase();
                if lower.contains("paste") || lower.contains("insert") {
                    return Some(i);
                }
            }
        }
        None
    }
}

#[async_trait]
impl TextInjector for AtspiInjector {
    fn name(&self) -> &'static str { "atspi" }
    
    async fn is_available(&self) -> bool {
        // Check if AT-SPI bus is accessible
        AccessibilityConnection::new().await.is_ok()
    }
    
    async fn inject(&self, text: &str) -> anyhow::Result<()> {
        let focused = self.connection.get_focused_object().await?;
        let interfaces = focused.get_interfaces().await?;
        
        // Try EditableText interface first
        if interfaces.contains(&Interface::EditableText) {
            focused.set_text_contents(text).await?;
            return Ok(());
        }
        
        // Try Action interface for paste
        if interfaces.contains(&Interface::Action) {
            if let Some(paste_idx) = Self::find_paste_action(&focused).await {
                // Set clipboard first (if clipboard injector available)
                // Then trigger paste action
                focused.do_action(paste_idx as i32).await?;
                return Ok(());
            }
        }
        
        Err(anyhow::anyhow!("No suitable injection method for focused element"))
    }
}
```

2) Clipboard Injector (Wayland-native)
- Feature: `text-injection-clipboard`
- Deps: `wl-clipboard-rs = { version = "0.9", optional = true }`
- Behavior:
  - Set clipboard owner with full session text; keep owner alive briefly.
  - Optional restore previous clipboard after a short delay.
- Availability probe: Wayland display present; clipboard seat available.
- Pros: Reliable for batch; preserves formatting/newlines.
- Cons: Needs a paste trigger from another injector (AT‑SPI2/ydotool) or user.

Implementation with proper lifetime management:
```rust
// clipboard_injector.rs
use wl_clipboard_rs::{copy::{MimeType, Options, Source}};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct ClipboardInjector {
    // Keep the last clipboard source alive
    _last_source: Arc<Mutex<Option<Source>>>,
}

impl ClipboardInjector {
    pub fn new() -> Self {
        Self {
            _last_source: Arc::new(Mutex::new(None)),
        }
    }
}

#[async_trait]
impl TextInjector for ClipboardInjector {
    fn name(&self) -> &'static str { "clipboard" }
    
    async fn is_available(&self) -> bool {
        // Check for Wayland display
        std::env::var("WAYLAND_DISPLAY").is_ok()
    }
    
    async fn inject(&self, text: &str) -> anyhow::Result<()> {
        let mut opts = Options::new();
        opts.clipboard(wl_clipboard_rs::copy::ClipboardType::Regular);
        
        // Create source that will stay alive
        let source = Source::Bytes(text.to_string().into_bytes().into());
        
        // Copy to clipboard
        opts.copy(source.clone(), MimeType::Text)?;
        
        // Keep source alive for paste to work
        *self._last_source.lock().await = Some(source);
        
        // Keep alive for at least 1 second
        tokio::time::sleep(Duration::from_millis(1000)).await;
        
        Ok(())
    }
}
```

3) Clipboard + AT‑SPI Paste (Composition)
- Feature: `text-injection-atspi,text-injection-clipboard` (both)
- Behavior:
  - Set clipboard via ClipboardInjector, then trigger Action::Paste via AT‑SPI2.
- Pros: Works even when EditableText is missing but Paste exists.
- Cons: Same AT‑SPI caveats; ensure clipboard owner lifetime.

4) Ydotool Injector (Opt-in fallback)
- Feature: none new; behind config `allow_ydotool`.
- Runtime dep: `ydotool` daemon with permissions.
- Behavior:
  - Either type text (`ydotool type --file -`) or send paste chord (Ctrl+V) after clipboard set.
- Availability probe: check `ydotool` and socket; refuse if not enabled.
- Pros: Broad coverage when enabled.
- Cons: Security/permission implications; keep explicit opt-in.

5) IMe Injector (Phase 3)
- Feature: `text-injection-ime` (future).
- Deps: Wayland text-input v3 + input-method v2 via `wayland-protocols`/`smithay-client-toolkit` or zbus bridges to Fcitx5/IBus.
- Behavior: Commit text with semantic context (preedit, surrounding text, hints).
- Pros: Most context-aware.
- Cons: Larger integration cost; user UX considerations.

6) Portal EIS Injector (Exploratory)
- Feature: `text-injection-portal-eis` (off by default).
- Deps: `ashpd` (xdg-desktop-portal), `reis` (pure-Rust libei) if applicable.
- Behavior: User-consented key events; still not semantic like IME.
- Caveat: Availability on KWin varies; treat as experimental.

7) VKM Injector (Experimental)
- Feature: `text-injection-vkm` (off by default).
- Deps: Wayland client + unstable VKM protocol bindings.
- Caveat: KWin typically restricts VKM to trusted clients; expect unauthorized.

8) Kdotool Assist (KDE-specific, opt-in)
- Feature: `text-injection-kdotool` (off by default).
- Deps: external CLI `kdotool` (no Rust crate dependency).
- Behavior:
  - Window control only: find/activate/raise/move windows via KWin scripting.
  - Use to assist focus/activation before AT‑SPI insert/paste; do not send keys via kdotool.
- Availability probe: Running under Plasma/KWin; binary present; DBus to KWin reachable.
- Pros: KDE-native focus/window control path; can improve success for AT‑SPI paste.
- Cons: Desktop-specific; not for keyboard/mouse synthesis; subject to KDE changes.

9) Enigo Injector (uinput, invasive opt-in)
- Feature: `text-injection-enigo` (off by default).
- Deps: `enigo` (optional = true) — cross-platform input simulation via uinput on Linux.
- Behavior:
  - Create virtual keyboard and type the batch string; or send paste chord after clipboard set.
- Availability probe: Can open `/dev/uinput` without root; udev rules applied.
- Pros: Mature, widely used; compositor-agnostic.
- Cons: Requires uinput permissions; kernel-level injection; potential interference with real input if misused.

10) MKI Injector (mouse-keyboard-input, uinput, invasive opt-in)
- Feature: `text-injection-mki` (off by default).
- Deps: `mouse-keyboard-input` (optional = true).
- Behavior:
  - Similar to Enigo; simple API to emit key sequences for batch text.
- Availability probe: Same as Enigo (`/dev/uinput`).
- Pros: Thin wrapper; predictable; compositor-agnostic.
- Cons: Same uinput caveats as Enigo.


## Strategy Manager (adaptive, bounded latency)

Responsibilities:
- Construct an ordered chain from available injectors based on:
  - Feature flags
  - FocusStatus (ConfirmedEditable first tries AT‑SPI direct, etc.)
  - Per-app success cache (prefer methods that worked recently for this app)
- Enforce per-call timeouts and a global budget (default ≤ 800 ms).
- Apply adaptive cooldown/circuit breaker per method and per app.

Implementation example:

```rust
// manager.rs
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct StrategyManager {
    injectors: Vec<Box<dyn TextInjector>>,
    success_cache: HashMap<(String, String), SuccessRecord>,
    cooldowns: HashMap<(String, String), CooldownState>,
    config: InjectionConfig,
}

#[derive(Debug)]
struct SuccessRecord {
    success_count: u32,
    fail_count: u32,
    last_success: Instant,
    avg_latency_ms: f64,
}

#[derive(Debug)]
struct CooldownState {
    until: Instant,
    backoff_level: u32,
    last_error: String,
}

impl StrategyManager {
    pub async fn try_inject(&mut self, text: &str, context: &FocusInfo) -> anyhow::Result<()> {
        let app_key = context.app_name.clone().unwrap_or_else(|| "unknown".to_string());
        let deadline = Instant::now() + Duration::from_millis(self.config.max_total_latency_ms);
        
        // Build candidate chain
        let mut chain = self.build_chain(&app_key, &context.status);
        
        // Try each method
        for injector in chain.iter() {
            let method_key = (app_key.clone(), injector.name().to_string());
            
            // Skip if in cooldown
            if let Some(cooldown) = self.cooldowns.get(&method_key) {
                if Instant::now() < cooldown.until {
                    tracing::debug!("Skipping {} - in cooldown for {}s", 
                        injector.name(),
                        (cooldown.until - Instant::now()).as_secs());
                    continue;
                }
            }
            
            // Check deadline
            if Instant::now() > deadline {
                return Err(anyhow::anyhow!("Injection timeout - exceeded {}ms budget", 
                    self.config.max_total_latency_ms));
            }
            
            // Try injection with timeout
            let start = Instant::now();
            match tokio::time::timeout(
                self.get_method_timeout(injector.name()),
                injector.inject(text)
            ).await {
                Ok(Ok(())) => {
                    // Success - update cache
                    let latency = start.elapsed();
                    self.record_success(&method_key, latency);
                    tracing::info!("Successfully injected via {} in {:?}", injector.name(), latency);
                    return Ok(());
                }
                Ok(Err(e)) | Err(_) => {
                    // Failure - update cooldown
                    self.record_failure(&method_key, e.to_string());
                    tracing::warn!("Method {} failed: {}", injector.name(), e);
                }
            }
        }
        
        Err(anyhow::anyhow!("All injection methods failed"))
    }
    
    fn build_chain(&self, app: &str, focus: &FocusStatus) -> Vec<&Box<dyn TextInjector>> {
        let mut chain = Vec::new();
        
        // Order by focus status
        let base_order = match focus {
            FocusStatus::ConfirmedEditable => {
                vec!["atspi", "clipboard_atspi", "clipboard", "ydotool"]
            }
            FocusStatus::NonEditable => {
                vec!["clipboard_atspi", "clipboard", "ydotool"]
            }
            FocusStatus::Unknown if self.config.inject_on_unknown_focus => {
                vec!["atspi", "clipboard_atspi", "clipboard", "ydotool"]
            }
            _ => return chain,
        };
        
        // Add available injectors
        for name in base_order {
            if let Some(injector) = self.injectors.iter().find(|i| i.name() == name) {
                if injector.is_available().await {
                    chain.push(injector);
                }
            }
        }
        
        // Reorder by recent success for this app
        chain.sort_by_cached_key(|inj| {
            let key = (app.to_string(), inj.name().to_string());
            self.success_cache.get(&key)
                .map(|r| (-(r.success_count as i32), r.avg_latency_ms as i32))
                .unwrap_or((0, i32::MAX))
        });
        
        chain
    }
    
    fn calculate_cooldown(&self, failure_count: u32) -> Duration {
        let base = self.config.cooldown_on_failure_ms;
        let factor = self.config.cooldown_backoff_factor.powi(failure_count as i32);
        let ms = (base as f32 * factor).min(self.config.cooldown_max_ms as f32) as u64;
        Duration::from_millis(ms)
    }
}
```

Algorithm (per session finalize):
1) Compute context = {FocusStatus, app_class, window_title?}.
2) Build candidate list:
  - If `ConfirmedEditable`: [AtspiInsert] → [Clipboard+AtspiPaste] → [Kdotool?] → [Enigo?/MKI?] → [ClipboardOnly] → [Ydotool?]
  - If `NonEditable`: [Clipboard+AtspiPaste] → [Kdotool?] → [Enigo?/MKI?] → [ClipboardOnly] → [Ydotool?]
  - If `Unknown` and cfg.inject_on_unknown_focus: [AtspiInsert] → [Clipboard+AtspiPaste] → [Kdotool?] → [Enigo?/MKI?] → [ClipboardOnly] → [Ydotool?]
3) Reorder by recent success for (app_class, method) with decay; drop methods in cooldown.
4) For each method in order:
   - Skip if `!is_available()` or in active cooldown.
   - Attempt `inject(text)` with per-method timeout.
   - On success: record success/latency; stop.
   - On failure/timeout: record error; start/update cooldown for this (app, method) with exponential backoff (e.g., 30s → 2m → 10m; max 1h).
5) If all fail: emit error, keep buffer (optional) or drop based on policy.

Cooldown/circuit breaker:
- Per (app_class, method) entries with timestamps.
- Decay success score over time; clear cooldown after “probation” success.

Telemetry:
- Counters: attempts/success/failure per method; per-app success rate.
- Gauges: last latency per method; current cooldowns; last chosen method.
- Logs: focused role/type, reason for skips, error summaries.


## Focus Tracker Implementation

Event-driven focus tracking with AT-SPI2:

```rust
// focus.rs
use atspi::{
    events::{Event, EventProperties, FocusEvent},
    AccessibilityConnection,
    CoordType, Interface, Role, StateSet,
};
use tokio::sync::RwLock;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub enum FocusStatus {
    ConfirmedEditable,
    NonEditable,
    Unknown,
}

#[derive(Clone, Debug)]
pub struct FocusInfo {
    pub status: FocusStatus,
    pub app_name: Option<String>,
    pub window_title: Option<String>,
    pub role: Option<Role>,
    pub interfaces: Vec<Interface>,
}

pub struct FocusTracker {
    connection: Arc<AccessibilityConnection>,
    current_focus: Arc<RwLock<FocusInfo>>,
}

impl FocusTracker {
    pub async fn new() -> anyhow::Result<Self> {
        let connection = AccessibilityConnection::new().await?;
        
        // Register for focus events
        connection.register_event::<FocusEvent>().await?;
        
        let tracker = Self {
            connection: Arc::new(connection),
            current_focus: Arc::new(RwLock::new(FocusInfo {
                status: FocusStatus::Unknown,
                app_name: None,
                window_title: None,
                role: None,
                interfaces: vec![],
            })),
        };
        
        // Start event listener
        let focus_clone = tracker.current_focus.clone();
        let conn_clone = tracker.connection.clone();
        tokio::spawn(async move {
            let mut event_stream = conn_clone.event_stream();
            while let Some(event) = event_stream.recv().await {
                if let Ok(Event::Focus(focus_event)) = event {
                    // Update focus info
                    if let Ok(object) = focus_event.object() {
                        let mut info = focus_clone.write().await;
                        info.role = object.role().await.ok();
                        info.interfaces = object.interfaces().await.unwrap_or_default();
                        info.app_name = object.application().await
                            .and_then(|app| app.name().ok());
                        
                        // Determine status based on interfaces and role
                        info.status = if info.interfaces.contains(&Interface::EditableText) {
                            FocusStatus::ConfirmedEditable
                        } else if info.interfaces.contains(&Interface::Text) {
                            // Text interface might be editable
                            match info.role {
                                Some(Role::Text) | Some(Role::Entry) | Some(Role::PasswordText) => {
                                    FocusStatus::ConfirmedEditable
                                }
                                Some(Role::Terminal) => FocusStatus::NonEditable,
                                _ => FocusStatus::Unknown
                            }
                        } else if info.role == Some(Role::Terminal) {
                            FocusStatus::NonEditable
                        } else {
                            FocusStatus::Unknown
                        };
                    }
                }
            }
        });
        
        Ok(tracker)
    }
    
    pub async fn current_focus(&self) -> FocusInfo {
        self.current_focus.read().await.clone()
    }
    
    // One-shot probe for current focus without event subscription
    pub async fn probe_focus(&self) -> anyhow::Result<FocusInfo> {
        let desktop = self.connection.desktop().await?;
        if let Ok(focused) = desktop.get_active_descendant().await {
            let role = focused.role().await.ok();
            let interfaces = focused.interfaces().await.unwrap_or_default();
            let app_name = focused.application().await
                .and_then(|app| app.name().ok());
            
            let status = if interfaces.contains(&Interface::EditableText) {
                FocusStatus::ConfirmedEditable
            } else if role == Some(Role::Terminal) {
                FocusStatus::NonEditable
            } else {
                FocusStatus::Unknown
            };
            
            Ok(FocusInfo {
                status,
                app_name,
                window_title: None,
                role,
                interfaces,
            })
        } else {
            Ok(FocusInfo {
                status: FocusStatus::Unknown,
                app_name: None,
                window_title: None,
                role: None,
                interfaces: vec![],
            })
        }
    }
}
```


## Configuration (InjectionConfig)

Extend config:
```rust
pub struct InjectionConfig {
    pub silence_timeout_ms: u64,              // finalize session after silence
    pub inject_on_unknown_focus: bool,        // try even if focus unknown
    pub allow_ydotool: bool,                  // opt-in privileged fallback
    pub restore_clipboard: bool,              // attempt clipboard restore
    pub max_total_latency_ms: u64,            // overall fallback budget (≤ 800)
    pub method_timeouts_ms: MethodTimeouts,   // per-method caps
    pub cooldown_on_failure_ms: MethodCooldowns, // initial cooldowns
    pub cooldown_backoff_factor: f32,         // e.g., 2.0
    pub cooldown_max_ms: u64,                 // e.g., 3600_000
    pub per_app_opt_out: Vec<String>,         // optional list of app classes to skip injection
    // invasive/desktop-specific toggles (all default false)
    pub allow_kdotool: bool,                  // KDE-specific invasive path
    pub allow_enigo: bool,                    // uinput path via enigo
    pub allow_mki: bool,                      // uinput path via mouse-keyboard-input
}

impl InjectionConfig {
    pub fn from_env() -> Self {
        Self {
            silence_timeout_ms: std::env::var("INJECTION_SILENCE_TIMEOUT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(500),
            inject_on_unknown_focus: std::env::var("INJECTION_UNKNOWN_FOCUS")
                .ok()
                .map(|s| s == "true")
                .unwrap_or(true),
            allow_ydotool: std::env::var("INJECTION_ALLOW_YDOTOOL")
                .ok()
                .map(|s| s == "true")
                .unwrap_or(false),
            restore_clipboard: std::env::var("INJECTION_RESTORE_CLIPBOARD")
                .ok()
                .map(|s| s == "true")
                .unwrap_or(false),
            max_total_latency_ms: std::env::var("INJECTION_MAX_LATENCY")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(800),
            method_timeouts_ms: MethodTimeouts::default(),
            cooldown_on_failure_ms: MethodCooldowns::default(),
            cooldown_backoff_factor: 2.0,
            cooldown_max_ms: 3600_000,
            per_app_opt_out: std::env::var("INJECTION_OPT_OUT_APPS")
                .ok()
                .map(|s| s.split(',').map(String::from).collect())
                .unwrap_or_default(),
            allow_kdotool: false,
            allow_enigo: false,
            allow_mki: false,
        }
    }
}
```

Reasonable defaults:
- max_total_latency_ms: 800
- timeouts: AT‑SPI 300, Clipboard set 250, AT‑SPI Paste 250, Ydotool 400
- cooldown initial: AT‑SPI 30s, Clipboard 15s, Ydotool 60s; backoff ×2, max 1h
- For invasive injectors: Enigo/MKI 60s initial cooldown; Kdotool 45s


## Module layout (crate: `crates/app`)

```
src/
  text_injection/
    mod.rs                 // re-exports; feature gates
    types.rs               // TextInjector trait, FocusStatus, InjectionError, metrics
    manager.rs             // StrategyManager (selection, cooldowns, metrics)
    session.rs             // Session buffer + silence timeout (reuse Phase 1)
    focus.rs               // AT‑SPI2 focus tracker (feature = atspi)
    config.rs              // InjectionConfig and loading logic
    
    // Core injectors
    atspi_injector.rs      // AT‑SPI2 direct + paste (feature = atspi)
    clipboard_injector.rs  // wl-clipboard-rs (feature = clipboard)
    combo_clip_atspi.rs    // composition helper (features = atspi+clipboard)
    ydotool_injector.rs    // opt-in (runtime presence)
    
    // Invasive injectors (separate subdirectory for clarity)
    invasive/
      kdotool_injector.rs  // KDE-specific via kdotool (feature = kdotool)
      enigo_injector.rs    // uinput via enigo (feature = enigo)
      mki_injector.rs      // uinput via mouse-keyboard-input (feature = mki)
    
    // Future/experimental
    experimental/
      ime_injector.rs      // Phase 3 (feature = ime)
      portal_eis_injector.rs // exploratory (feature = portal-eis)
      vkm_injector.rs      // experimental (feature = vkm)
    
    // Testing support
    tests/
      helpers.rs           // Mock injectors and test utilities
      integration.rs       // Integration tests
```


## Cargo features & dependencies

`crates/app/Cargo.toml` (sketch):
  - `text-injection = []`
  - `text-injection-atspi = ["text-injection", "atspi"]`
  - `text-injection-clipboard = ["text-injection", "wl-clipboard-rs"]`
  - `text-injection-ydotool = ["text-injection"]` (no crate dep; runtime tool)
  - `text-injection-ime = ["text-injection"]` (future)
  - `text-injection-portal-eis = ["text-injection", "ashpd"]` (exploratory)
  - `text-injection-vkm = ["text-injection"]` (experimental)
  - `text-injection-kdotool = ["text-injection"]` (KDE assist via external CLI)
  - `text-injection-enigo = ["text-injection", "enigo"]` (uinput invasive)
  - `text-injection-mki = ["text-injection", "mouse-keyboard-input"]` (uinput invasive)
  - `atspi = { version = "0.28", features = ["tokio"], optional = true }`  // Pure-Rust AT‑SPI2 (zbus-based)
  - `zbus = { version = "5", default-features = false, features = ["tokio"], optional = true }` // Only if used directly
  - `wl-clipboard-rs = { version = "0.9", optional = true }`               // Wayland clipboard for non-GUI apps
  - `ashpd = { version = "0.9", features = ["tokio", "wayland"], optional = true }` // XDG portals wrapper
  - `reis = { version = "0.4", features = ["tokio"], optional = true }`    // libei/eis (experimental)
  - `enigo = { version = "0.2", default-features = false, features = ["wayland", "libei_tokio"], optional = true }` // input sim
  - `mouse-keyboard-input = { version = "0.9", optional = true }`           // uinput wrapper
  - `thiserror = { version = "2.0" }`                                       // Error type derivation
  - `async-trait = { version = "0.1" }`                                     // Async trait support
  - (No crate dep for kdotool; it’s a CLI. Use tokio::process to run if enabled.)
  - `anyhow`, `tracing` (already present)

Verified crates (references):
- atspi — Pure-Rust AT‑SPI2 (https://lib.rs/crates/atspi)
- zbus — D‑Bus (https://lib.rs/crates/zbus)
- wl-clipboard-rs — Wayland clipboard (https://lib.rs/crates/wl-clipboard-rs)
- ashpd — XDG portals (https://lib.rs/crates/ashpd)
- reis — libei/eis protocol (https://lib.rs/crates/reis)
- enigo — input simulation (Wayland/libei features) (https://lib.rs/crates/enigo)
- mouse-keyboard-input — uinput (https://lib.rs/crates/mouse-keyboard-input)
- kdotool — KDE Wayland xdotool-like CLI (https://lib.rs/crates/kdotool)


## Selection & timeouts (defaults)

Order builder (KDE focus):
1) AT‑SPI2 EditableText/Action (if atspi enabled & available)
2) Clipboard + AT‑SPI Paste (if both atspi & clipboard enabled)
3) Kdotool (if allowed & available)
4) Enigo/MKI uinput path (if allowed & available)
5) Clipboard only (trace; user paste)
6) Ydotool (if allowed & available)
- Wrap each attempt with its per-method timeout and enforce `max_total_latency_ms`.
- Debounce focus changes by ~75 ms before injection.


## Minimal wiring

- New `InjectionConfig` exposed via CLI/env; default disabled.
- In main pipeline setup, spawn `InjectionProcessor` only if `--enable-text-injection`.
- Wire `StrategyManager` with:
  - references to constructed injectors (based on enabled features)
  - `FocusTracker` (optional: only if atspi feature)
  - `PipelineMetrics` to publish injection metrics
- Log method chosen, latency, and success/fail per injection.


## Local testing (manual)

Baseline KDE checks:
- Wayland session: `echo $WAYLAND_DISPLAY` is set
- Accessibility enabled (DE settings)
- Focus a text field in Kate/Firefox/LibreOffice → run the demo

Demo runs:
- Build with features: `text-injection`, `text-injection-atspi`, `text-injection-clipboard`
- Exercise: send fixed string through `StrategyManager` and verify:
  - AT‑SPI2 direct insert works where EditableText exists
  - Clipboard + AT‑SPI Paste works where only Paste is present
  - Clipboard-only path sets clipboard (manual paste)
  - Ydotool path works when opted-in and daemon is running
  - Kdotool path works on KDE when enabled
  - Enigo/MKI uinput paths work when `/dev/uinput` permissions are configured

Clipboard tips:
- Keep provider alive ~500–1500 ms; consider optional restore after 500 ms.

uinput setup (for Enigo/MKI):
- Ensure `/dev/uinput` is present; create udev rule to grant group access (e.g., `MODE="0660", GROUP="input"`).
- Add user to `input` group and relogin; avoid running as root in production.
- Verify by running a minimal uinput test to emit a key and observing in a text field.


## Testing Utilities

Test helpers and mock implementations:

```rust
// tests/helpers.rs
pub async fn create_test_focus_context(editable: bool) -> FocusInfo {
    FocusInfo {
        status: if editable { 
            FocusStatus::ConfirmedEditable 
        } else { 
            FocusStatus::NonEditable 
        },
        app_name: Some("test_app".to_string()),
        window_title: Some("Test Window".to_string()),
        role: Some(if editable { Role::Text } else { Role::Label }),
        interfaces: if editable {
            vec![Interface::Text, Interface::EditableText]
        } else {
            vec![Interface::Text]
        },
    }
}

#[cfg(test)]
mod mock_injectors {
    use super::*;
    
    pub struct MockSuccessInjector;
    
    #[async_trait]
    impl TextInjector for MockSuccessInjector {
        fn name(&self) -> &'static str { "mock_success" }
        async fn is_available(&self) -> bool { true }
        async fn inject(&self, _: &str) -> anyhow::Result<()> { Ok(()) }
    }
    
    pub struct MockFailInjector;
    
    #[async_trait]
    impl TextInjector for MockFailInjector {
        fn name(&self) -> &'static str { "mock_fail" }
        async fn is_available(&self) -> bool { true }
        async fn inject(&self, _: &str) -> anyhow::Result<()> { 
            Err(anyhow::anyhow!("Mock failure"))
        }
    }
    
    pub struct MockTimeoutInjector;
    
    #[async_trait]
    impl TextInjector for MockTimeoutInjector {
        fn name(&self) -> &'static str { "mock_timeout" }
        async fn is_available(&self) -> bool { true }
        async fn inject(&self, _: &str) -> anyhow::Result<()> {
            tokio::time::sleep(Duration::from_secs(10)).await;
            Ok(())
        }
    }
}

// Test for Strategy Manager cooldown behavior
#[tokio::test]
async fn test_cooldown_backoff() {
    let mut manager = StrategyManager::new(InjectionConfig::default());
    manager.add_injector(Box::new(MockFailInjector));
    
    let context = create_test_focus_context(true).await;
    
    // First failure
    let result1 = manager.try_inject("test", &context).await;
    assert!(result1.is_err());
    
    // Immediate retry should skip due to cooldown
    let result2 = manager.try_inject("test", &context).await;
    assert!(result2.is_err());
    
    // Verify cooldown exists
    let key = ("test_app".to_string(), "mock_fail".to_string());
    assert!(manager.cooldowns.contains_key(&key));
}
```


## Telemetry

- Counters per method: attempts, successes, failures, timeouts
- Gauges: last_latency_ms, cooldown_remaining_ms(app, method)
- Histograms (optional): latency per method
- Tracing: focused role/type, action chosen, reason for skip/cooldown


## Risks & mitigations

- Many terminals lack EditableText → expect clipboard-based paths.
- Focus may be on containers → try Action::Paste before giving up.
- Localized action names → lowercase match for “paste” in name/description.
- Wayland clipboard semantics → keep owner alive; restore optionally.
- Firefox/Electron quirks on Wayland → consider `MOZ_ENABLE_WAYLAND=1`, ozone flags.
- Ydotool permissions → require explicit opt-in; document setup; disable by default.
- VKM likely restricted on KWin → keep experimental and disabled by default.
- Invasive paths (Kdotool/Enigo/MKI) → require explicit opt-in flags; show clear UI/CLI warnings; add cooldown/backoff to reduce disruption if a method misbehaves.
- Kernel-level injection (uinput) → race with real input; rate-limit typing and cap keypress frequency; prefer clipboard+paste for large batches.


## Acceptance criteria (Phase 2+)

- KDE apps (Firefox text areas, Kate, LibreOffice): successful hands-free batch injection via AT‑SPI2 or clipboard+paste.
- Fallback completes within overall latency budget (≤ 0.8 s by default).
- Adaptive skipping works: a repeatedly failing method for an app is cooled down; subsequent injections try other methods first.
- No crashes when buses/tools are missing; `is_available()` filters candidates.
- Telemetry shows chosen method and success rate per app.


## Phase 3 and beyond

- IME integration (Fcitx5/IBus) for semantic, context-aware composition.
- Portal/libei path if KDE exposes user-consented injection; keep behind feature.
- Per-app heuristics cache persisted across runs (optional).
- Configurable per-app preferences and opt-outs.


## Appendix A — Implementation sketch (high-level)

- Focus tracker (AT‑SPI2): subscribe to focus events; expose `current_focus()`.
- StrategyManager:
  - `build_chain(ctx) -> Vec<&dyn TextInjector>` based on features, focus, success cache
  - `try_inject(text, ctx)` applying per-method timeout and global budget
  - `record_result(app, method, result, latency)` updating cooldowns
- Cooldown store: `HashMap<(String /*app*/ , String /*method*/), CooldownState>`
- Metrics: integrate with `PipelineMetrics` or add `InjectionMetrics`.


## Appendix B — Cargo changes (consolidated)

- Add features:
  - `text-injection = []`
  - `text-injection-atspi = ["text-injection", "atspi"]`
  - `text-injection-clipboard = ["text-injection", "wl-clipboard-rs"]`
  - `text-injection-ydotool = ["text-injection"]`
  - `text-injection-ime = ["text-injection"]`
  - `text-injection-portal-eis = ["text-injection", "ashpd"]`
  - `text-injection-vkm = ["text-injection"]`
- Add optional deps:
  - `atspi`, `wl-clipboard-rs`, `ashpd`, `reis`, `zbus`


## Appendix C — Tiny contracts

- Input: non-empty UTF‑8 text.
- Success: any injector returns Ok within budget.
- Failure: emit error; update cooldown; surface status in UI logs.
- Safety: prefer user-space paths; privileged tools opt-in only.

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
