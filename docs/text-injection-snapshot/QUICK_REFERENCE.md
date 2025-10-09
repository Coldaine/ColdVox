# Text Injection System - Quick Reference

## 🚀 One-Minute Overview

The ColdVox text injection system is a **multi-strategy, adaptive text insertion framework** that reliably injects speech-to-text transcriptions into focused applications across Linux desktop environments.

**Total Size**: 279KB, 7,504 lines of Rust code  
**Status**: Production-ready with comprehensive tests  
**Architecture**: Plugin-based with intelligent fallback chain

---

## 📊 Current Implementation at a Glance

### Injection Methods (Priority Order)

```
1️⃣  AT-SPI Insert          [PRIMARY]   Direct accessibility API
2️⃣  kdotool               [OPT-IN]    KDE window activation helper  
3️⃣  Enigo                 [OPT-IN]    Cross-platform input simulation
4️⃣  Clipboard+Paste       [FALLBACK]  Clipboard seeding + paste
5️⃣  NoOp                  [ALWAYS]    Telemetry-only fallback
```

### Key Statistics

| Component | Lines | Files | Purpose |
|-----------|-------|-------|---------|
| Core Logic | 4,500 | 8 | Manager, processor, session, focus |
| Injectors | 1,800 | 8 | AT-SPI, clipboard, enigo, kdotool, etc. |
| Infrastructure | 1,200 | 5 | Backend detection, window mgmt, logging |
| Tests | 2,100 | 13 | Unit, integration, real injection |

---

## 🔑 Key Files to Understand

### 1. **manager.rs** (1,391 lines) - The Brain
**What**: Central orchestrator for all injection strategies  
**Key Features**:
- Intelligent method selection based on success rates
- Per-app, per-method cooldown system
- Exponential backoff on failures
- Privacy-first text redaction

**Key Functions**:
```rust
pub async fn inject_text(&mut self, text: &str) -> Result<()>
fn compute_method_order(&self, app_id: &str) -> Vec<InjectionMethod>
fn update_success_record(&mut self, app_id: &str, method: InjectionMethod, success: bool)
```

### 2. **processor.rs** (362 lines) - The Pipeline
**What**: Async processor that bridges STT → Injection  
**Key Features**:
- Event-driven transcription handling
- Buffering and timing coordination
- Metrics collection
- Graceful shutdown

**Key Functions**:
```rust
pub async fn run(&mut self) -> Result<()>
async fn handle_transcription(&mut self, event: TranscriptionEvent)
```

### 3. **session.rs** (421 lines) - The Buffer
**What**: State machine for transcription accumulation  
**States**: `Idle → Buffering → WaitingForSilence → ReadyToInject`

**Key Features**:
- Configurable silence detection (default: 2s)
- Smart transcription joining
- Punctuation-based flushing
- Buffer size limits

### 4. **focus.rs** (1,088 lines) - The Targeting
**What**: AT-SPI-based focus detection and validation  
**Key Features**:
- Real-time focused window tracking
- Editable field detection
- Application ID extraction
- Focus change monitoring

### 5. **atspi_injector.rs** (378 lines) - The Workhorse
**What**: Primary injection method via Linux accessibility APIs  
**Success Rate**: ~90% on supported applications  
**How**: Direct text insertion via `EditableText` interface

---

## 🎯 How It Works (Simplified Flow)

```
┌─────────────────────────────────────────────────────────────┐
│  1. STT emits TranscriptionEvent("Hello world")             │
└──────────────────────┬──────────────────────────────────────┘
                       ↓
┌─────────────────────────────────────────────────────────────┐
│  2. InjectionProcessor receives event                        │
│     → Forwards to InjectionSession for buffering             │
└──────────────────────────┬──────────────────────────────────┘
                       ↓
┌─────────────────────────────────────────────────────────────┐
│  3. InjectionSession accumulates text                        │
│     State: Idle → Buffering → WaitingForSilence             │
│     After 2s silence → ReadyToInject                         │
└──────────────────────────┬──────────────────────────────────┘
                       ↓
┌─────────────────────────────────────────────────────────────┐
│  4. StrategyManager.inject_text("Hello world")               │
│     → Checks focus: FocusProvider.get_current_app_id()       │
│     → Gets method order: compute_method_order("Firefox")     │
└──────────────────────────┬──────────────────────────────────┘
                       ↓
┌─────────────────────────────────────────────────────────────┐
│  5. Try methods in order:                                    │
│     ① AT-SPI Insert → SUCCESS! ✓                            │
│     (skips remaining methods)                                │
└──────────────────────────┬──────────────────────────────────┘
                       ↓
┌─────────────────────────────────────────────────────────────┐
│  6. Update success record & metrics                          │
│     Firefox/AtspiInsert: 95% success rate (19/20)           │
└─────────────────────────────────────────────────────────────┘
```

---

## 🔧 Configuration Points

### Cargo Features
```toml
default = ["atspi", "wl_clipboard", "linux-desktop"]
atspi           # Linux accessibility API
wl_clipboard    # Clipboard operations
enigo           # Input simulation (opt-in)
kdotool         # KDE automation (opt-in)
all-backends    # Enable everything
```

### Runtime Config (InjectionConfig)
```rust
InjectionConfig {
    allow_kdotool: false,        // Enable kdotool
    allow_enigo: false,          // Enable Enigo
    allow_clipboard: true,       // Allow clipboard fallback
    redact_text_in_logs: true,   // Privacy protection
    focus_enforcement: Strict,   // Require valid focus
    app_allowlist: vec![],       // Specific apps only
    app_blocklist: vec![],       // Never inject here
}
```

---

## 📈 Success Tracking System

**Per-App, Per-Method Records**:
```rust
SuccessRecord {
    success_count: 19,
    fail_count: 1,
    success_rate: 0.95,     // 95%
    last_success: Some(Instant),
    last_failure: Some(Instant),
}
```

**Cooldown on Failure**:
- First failure: 10s cooldown
- Second: 20s (exponential backoff)
- Third: 40s
- Max: 5 minutes

---

## 🧪 Testing Philosophy

**Test Distribution**:
- 70% Service integration (real AT-SPI, clipboard)
- 15% Trace-based (multi-app verification)
- 10% Contract (protocol compliance)
- 5% Pure logic (algorithms)

**Test Categories**:
```
tests/
├── Unit tests              50+ tests (always run)
├── Integration tests       Full pipeline (CI)
├── Real injection tests    Hardware-dependent (feature-gated)
└── Smoke tests            Quick validation (pre-commit)
```

---

## 🐛 Common Issues & Solutions

### Issue: AT-SPI Not Working
```bash
# Check if at-spi2-core is running
ps aux | grep at-spi

# Enable accessibility in Qt apps
export QT_LINUX_ACCESSIBILITY_ALWAYS_ON=1

# Test AT-SPI connection
busctl --user list | grep a11y
```

### Issue: Clipboard Not Restoring
```rust
// Config option (default: 50ms)
config.clipboard_restore_delay_ms = 100;
```

### Issue: Wrong App Targeted
```rust
// Enable strict focus enforcement
config.focus_enforcement = FocusEnforcement::Strict;

// Add to allowlist
config.app_allowlist.push("Firefox".to_string());
```

---

## 📚 Deep Dive Resources

1. **SNAPSHOT_INDEX.md** - Complete file manifest and architecture
2. **README.md** - API documentation and usage examples
3. **TESTING.md** - Test execution and debugging
4. **manager.rs** - Read the strategy selection logic
5. **focus.rs** - Understand focus detection internals

---

## 🔄 Current vs Future

### ✅ What's Working Now
- AT-SPI Insert (primary, 90%+ success)
- Clipboard+Paste fallback (universal but disruptive)
- Enigo input simulation (cross-platform)
- kdotool window activation (KDE)
- Adaptive success tracking
- Per-app method learning

### 📋 Planned Enhancements (from InjectionMaster.md)
- AT-SPI Paste (clipboard + AT-SPI paste_text)
- Portal/EIS typing (xdg-desktop-portal + libei)
- KDE fake-input helper (privileged)
- Pre-warming (prepare methods before injection)
- Event-based confirmation (<75ms timeouts)
- Sub-50ms stage budgets

---

## 💡 Pro Tips

1. **Enable Debug Logging**: `RUST_LOG=coldvox_text_injection=debug`
2. **Test in Isolation**: `cargo test -p coldvox-text-injection`
3. **Real Hardware Tests**: `cargo test --features real-injection-tests`
4. **Check Success Rates**: Logs show per-app statistics
5. **Privacy Mode**: `redact_text_in_logs: true` (default)

---

**Last Updated**: October 9, 2025  
**Snapshot Location**: `docs/text-injection-snapshot/`  
**Line Count**: 7,504 lines (279KB)
