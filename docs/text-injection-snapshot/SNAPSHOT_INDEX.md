# ColdVox Text Injection System - Code Snapshot

**Snapshot Date**: October 9, 2025  
**Branch**: InjectionRefactor  
**Total Lines of Code**: ~7,504 lines

This directory contains a complete snapshot of the ColdVox text injection system as implemented.

## Directory Structure

```
text-injection-snapshot/
├── SNAPSHOT_INDEX.md          # This file
├── README.md                   # Original crate documentation
├── TESTING.md                  # Testing guidelines
├── Cargo.toml                  # Dependencies and features
├── build.rs                    # Build-time platform detection
│
├── Core System Files (*.rs)
│   ├── lib.rs                  # Public API and exports (124 lines)
│   ├── types.rs                # Core types and enums (491 lines)
│   ├── manager.rs              # Strategy manager & fallback logic (1391 lines)
│   ├── processor.rs            # Async injection processor (362 lines)
│   ├── session.rs              # Buffering & state machine (421 lines)
│   ├── focus.rs                # Focus tracking & AT-SPI integration (1088 lines)
│   ├── backend.rs              # Backend detection (235 lines)
│   ├── window_manager.rs       # Window info & compositor detection (262 lines)
│   └── log_throttle.rs         # Rate-limited logging (146 lines)
│
├── Injector Implementations
│   ├── atspi_injector.rs       # AT-SPI accessibility API (378 lines)
│   ├── clipboard_injector.rs   # Basic clipboard operations (249 lines)
│   ├── clipboard_paste_injector.rs  # Clipboard + paste combo (284 lines)
│   ├── enigo_injector.rs       # Cross-platform input simulation (177 lines)
│   ├── kdotool_injector.rs     # KDE/X11 window activation (261 lines)
│   ├── ydotool_injector.rs     # Linux uinput automation (213 lines)
│   ├── combo_clip_ydotool.rs   # Combined clipboard+ydotool (208 lines)
│   └── noop_injector.rs        # No-op fallback for testing (63 lines)
│
└── tests/                      # Comprehensive test suite (2,133 lines)
    ├── mod.rs                  # Test module organization
    ├── real_injection.rs       # Real injection tests (356 lines)
    ├── real_injection_smoke.rs # Quick smoke tests (326 lines)
    ├── test_adaptive_strategy.rs    # Strategy adaptation tests (87 lines)
    ├── test_allow_block.rs     # Allow/block list tests (54 lines)
    ├── test_async_processor.rs # Async processor tests (53 lines)
    ├── test_focus_enforcement.rs    # Focus validation tests (116 lines)
    ├── test_focus_tracking.rs  # Focus tracking tests (96 lines)
    ├── test_harness.rs         # Test infrastructure (252 lines)
    ├── test_integration.rs     # Integration tests (183 lines)
    ├── test_mock_injectors.rs  # Mock injector tests (71 lines)
    ├── test_permission_checking.rs  # Permission tests (50 lines)
    ├── test_regex_metrics.rs   # Regex metrics tests (25 lines)
    ├── test_util.rs            # Test utilities (206 lines)
    └── test_window_manager.rs  # Window manager tests (63 lines)
```

## Key Components Overview

### 1. Core Architecture

**StrategyManager** (`manager.rs` - 1391 lines)
- Central orchestrator for all injection methods
- Intelligent fallback chain with success tracking
- Per-app method prioritization
- Exponential backoff cooldown system
- Privacy-first text redaction

**InjectionProcessor** (`processor.rs` - 362 lines)
- Async event processor for transcription events
- Integrates with STT pipeline
- Handles partial and final transcriptions
- Manages injection timing and buffering

**InjectionSession** (`session.rs` - 421 lines)
- State machine: Idle → Buffering → WaitingForSilence → ReadyToInject
- Configurable silence detection
- Transcription accumulation with smart joining
- Punctuation-based flushing

### 2. Injection Methods (Current Implementation)

| Priority | Method | File | Lines | Status |
|----------|--------|------|-------|--------|
| 1 | AT-SPI Insert | `atspi_injector.rs` | 378 | ✅ Production |
| 2 | kdotool | `kdotool_injector.rs` | 261 | ✅ Opt-in |
| 3 | Enigo | `enigo_injector.rs` | 177 | ✅ Opt-in |
| 4 | Clipboard+Paste | `clipboard_paste_injector.rs` | 284 | ✅ Fallback |
| 5 | NoOp | `noop_injector.rs` | 63 | ✅ Always |

### 3. Focus & Window Management

**FocusProvider** (`focus.rs` - 1088 lines)
- AT-SPI-based focus detection
- Real-time focus status tracking
- Application ID and window title extraction
- Editable field detection

**WindowManager** (`window_manager.rs` - 262 lines)
- Compositor detection (Wayland vs X11)
- KDE Plasma, Hyprland, GNOME detection
- Window class and PID extraction
- Desktop environment identification

### 4. Feature Flags & Build System

**Cargo.toml Features**:
```toml
atspi          # AT-SPI accessibility (default on Linux)
wl_clipboard   # Clipboard operations (default on Linux)
enigo          # Cross-platform input simulation
kdotool        # KDE/X11 window activation
regex          # Allow/block pattern matching
all-backends   # Enable all available backends
linux-desktop  # Recommended Linux desktop backends
real-injection-tests  # Enable hardware-dependent tests
```

**build.rs** (Platform Detection):
- Detects Wayland vs X11 at compile time
- KDE environment detection
- Enables appropriate backends based on environment

### 5. Testing Infrastructure (2,133 lines)

**Test Categories**:
- ✅ Unit tests: Component isolation (50+ tests)
- ✅ Integration tests: Full injection flows
- ✅ Real injection tests: Hardware-dependent (feature-gated)
- ✅ Smoke tests: Quick validation
- ✅ Adaptive strategy tests: Success rate tracking
- ✅ Focus enforcement tests: Application targeting

**Test Harness** (`test_harness.rs` - 252 lines):
- GTK test application launcher
- Headless X11/Wayland support
- Injection verification
- Cleanup automation

## Current vs Planned Architecture

### ✅ **Currently Implemented** (This Snapshot)

```
Priority Order:
1. AT-SPI Insert (atspi_injector.rs)
2. kdotool (kdotool_injector.rs) - if allow_kdotool
3. Enigo (enigo_injector.rs) - if allow_enigo
4. Clipboard+Paste (clipboard_paste_injector.rs)
5. NoOp (noop_injector.rs)
```

### 📋 **Planned Future** (InjectionMaster.md)

```
Priority Order (from planning docs):
1. AT-SPI Insert
2. AT-SPI Paste (with clipboard seed)
3. Portal/EIS Type (xdg-desktop-portal + libei)
4. KDE fake-input helper
```

**Key Differences**:
- ❌ **AT-SPI Paste**: Not implemented (separate from clipboard paste)
- ❌ **Portal/EIS**: Not implemented (aspirational)
- ❌ **KDE fake-input**: Not implemented
- ✅ **Enigo**: Implemented but not in plans
- ✅ **kdotool**: Implemented (different from planned fake-input)

## Configuration

**Runtime Config** (`types.rs`):
```rust
InjectionConfig {
    allow_kdotool: bool,
    allow_enigo: bool,
    allow_ydotool: bool,
    allow_clipboard: bool,
    redact_text_in_logs: bool,
    focus_enforcement: FocusEnforcement,
    app_allowlist: Vec<String>,
    app_blocklist: Vec<String>,
    // ... more fields
}
```

## Metrics & Observability

**PipelineMetrics** (`processor.rs`):
- Injection attempts/successes/failures
- Per-method statistics
- Latency tracking
- Cooldown state

**LogThrottle** (`log_throttle.rs`):
- Duplicate log suppression
- Configurable time windows
- Per-key throttling

## Key Algorithms

### Success Rate Tracking
- Per-app, per-method success records
- Exponential moving average
- Cooldown on repeated failures
- Adaptive method reordering

### Clipboard Hygiene
- Backup before injection
- Restore after completion
- Optional Klipper history cleanup (KDE)
- Configurable restoration delay

### Focus Validation
- Pre-injection focus checks
- Application ID matching
- Allowlist/blocklist enforcement
- Unknown focus handling

## Dependencies (Key Crates)

```toml
atspi = "0.21"           # Linux accessibility
wl-clipboard-rs = "0.8"  # Wayland clipboard
enigo = "0.6"            # Input simulation
x11rb = "0.13"           # X11 window management
tokio = "1.0"            # Async runtime
tracing = "0.1"          # Structured logging
```

## Usage Examples

### Basic Injection
```rust
let config = InjectionConfig::default();
let manager = StrategyManager::new(config).await?;
manager.inject_text("Hello world").await?;
```

### With Processor
```rust
let processor = AsyncInjectionProcessor::new(config, rx_transcriptions).await?;
processor.run().await?;  // Handles buffering & timing
```

## File Sizes Summary

```
Core System:     ~4,500 lines (manager, processor, session, focus, types)
Injectors:       ~1,800 lines (8 implementations)
Infrastructure:  ~1,200 lines (backend, window, logging, build)
Tests:           ~2,100 lines (13 test modules)
---
Total:           ~7,500 lines
```

## Notes

1. **Enigo is LIVE**: Fully implemented and tested, despite not being in planning docs
2. **Platform-aware**: Build system auto-detects and enables appropriate backends
3. **Production-ready**: Comprehensive test coverage and error handling
4. **Extensible**: Plugin-style injector architecture
5. **Observable**: Rich telemetry and structured logging

## Related Documentation

- `README.md` - Crate overview and API documentation
- `TESTING.md` - Testing guidelines and test execution
- `../../summary/injection-stack.md` - Architecture flowcharts
- `../../plans/InjectionMaster.md` - Future architecture plans
- `../../plans/InjectionTest1008.md` - Test strategy
- `../../plans/OpusCodeInject.md` - Implementation details

---

**Snapshot Integrity**: All files copied from `crates/coldvox-text-injection/` on October 9, 2025.  
**Purpose**: Documentation, reference, and preservation of current implementation.
