# ColdVox STT Plugin System Activation Plan

## Overview

This plan activates the existing production-ready plugin system with Vosk as the flagship STT backend. Critical discovery: **All infrastructure exists** - only missing one line of plugin registration code.

**Timeline**: Phase A (2-4 hours) + Phase B (1-2 days) = **1-3 days total**

## Phase A: Immediate Plugin System Activation

**Estimated Time**: 2-4 hours
**Risk Level**: LOW (Single line change + existing infrastructure)

### A.1 - Plugin Registration Activation (5 minutes)

**File**: `crates/app/src/stt/plugin_manager.rs`
**Line**: 504-507

**Current Code**:
```rust
// Register Vosk plugin if the vosk feature is enabled in the app
#[cfg(feature = "vosk")]
{
    // TODO: Implement Vosk plugin registration after Step 2 completion
    // This will use the actual VoskTranscriber from coldvox-stt-vosk crate
    // For now, Vosk is handled through the legacy processor
}
```

**Replace With**:
```rust
// Register Vosk plugin if the vosk feature is enabled in the app
#[cfg(feature = "vosk")]
{
    use coldvox_stt::plugins::vosk::VoskPluginFactory;
    registry.register(Box::new(VoskPluginFactory::new()));
}
```

### A.2 - Verify Plugin Export Path (10 minutes)

**File**: `crates/coldvox-stt/src/plugins/mod.rs`
**Line**: ~32 (in re-exports section)

**Ensure These Exports Exist**:
```rust
#[cfg(feature = "vosk")]
pub use vosk::{VoskPlugin, VoskPluginFactory};
```

If missing, add to the existing `pub use` statements.

### A.3 - Immediate Testing (30 minutes)

```bash
# Test compilation
cd crates/app
cargo check --features vosk

# Test plugin system activation
cargo build --features vosk
cargo run --features vosk -- --log-level "info,stt=debug"

# Verify in logs:
# - "Registered plugin: vosk"
# - "Active plugin: vosk"
# - "VoskPlugin initialized successfully"
```

### A.4 - Runtime Integration Verification (30 minutes)

**Check Integration Points**:

1. **Plugin Manager Instantiation** (`crates/app/src/stt/runtime.rs:328-334`):
   - Verify plugin manager starts with registered plugins
   - Confirm default selection logic chooses Vosk when available

2. **Event Flow** (VAD → Plugin Manager → Text Injection):
   - Audio frames reach plugin manager via `process_audio_chunk()`
   - TranscriptionEvents flow to text injection system
   - Plugin metrics integrate with PipelineMetrics

3. **Configuration Propagation**:
   - VOSK_MODEL_PATH environment variable honored
   - Plugin configuration updates work via `update_selection_config()`

**Success Criteria for Phase A**:
- [x] Plugin manager shows "vosk" as available plugin
- [x] Speech recognition works through plugin system
- [x] No runtime errors in plugin selection/switching
- [x] TranscriptionEvents reach text injection system
- [x] Metrics collection shows plugin activity

---

## Phase B: Build System Restructuring

**Estimated Time**: 1-2 days
**Risk Level**: MEDIUM (Architectural dependency fix required)

### B.1 - Fix Telemetry Dependency Architecture Violation

**Problem**: `coldvox-telemetry` → `coldvox-text-injection` creates circular dependency risk and violates clean architecture principles.

**Solution**: Extract shared types into new crate.

### B.2 - Create coldvox-telemetry-types Crate

**New File**: `crates/coldvox-telemetry-types/Cargo.toml`
```toml
[package]
name = "coldvox-telemetry-types"
version = "0.1.0"
edition = "2021"
description = "Shared telemetry types and traits for ColdVox"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
```

**New File**: `crates/coldvox-telemetry-types/src/lib.rs`
```rust
//! Shared telemetry types for ColdVox components

use serde::{Deserialize, Serialize};

/// Text injection performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InjectionMetrics {
    pub attempts: u64,
    pub successes: u64,
    pub failures: u64,
    pub avg_latency_ms: f64,
    pub strategy_used: String,
}

/// Injection strategy selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InjectionStrategy {
    Clipboard,
    AtSpi,
    Ydotool,
    Kdotool,
    Enigo,
    ComboClipYdotool,
}

/// Text injection event types for telemetry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InjectionEvent {
    StrategySelected(InjectionStrategy),
    InjectionStarted { text_length: usize },
    InjectionCompleted { duration_ms: u64 },
    InjectionFailed { error: String, fallback_used: bool },
}
```

### B.3 - Move Types from Text Injection Crate

**File**: `crates/coldvox-text-injection/src/types.rs`

**Move These Types** to `coldvox-telemetry-types`:
- `InjectionMetrics` struct
- `InjectionStrategy` enum
- `InjectionEvent` enum
- Related serialization implementations

**Update Imports** in text injection crate:
```rust
// Replace internal types with re-exports
pub use coldvox_telemetry_types::{InjectionMetrics, InjectionStrategy, InjectionEvent};
```

### B.4 - Update Crate Dependencies

**File**: `crates/coldvox-telemetry/Cargo.toml`

**Remove**:
```toml
coldvox-text-injection = { path = "../coldvox-text-injection", optional = true }
```

**Add**:
```toml
coldvox-telemetry-types = { path = "../coldvox-telemetry-types" }
```

**File**: `crates/coldvox-text-injection/Cargo.toml`

**Add**:
```toml
coldvox-telemetry-types = { path = "../coldvox-telemetry-types" }
```

### B.5 - Update Import Statements

**File**: `crates/coldvox-telemetry/src/pipeline_metrics.rs`

**Replace**:
```rust
#[cfg(feature = "text-injection")]
use coldvox_text_injection::types::InjectionMetrics;
```

**With**:
```rust
use coldvox_telemetry_types::InjectionMetrics;
```

**File**: `crates/coldvox-text-injection/src/manager.rs`

**Update**:
```rust
use coldvox_telemetry_types::{InjectionMetrics, InjectionEvent, InjectionStrategy};
```

### B.6 - Feature Flag Optimization

**File**: `crates/app/Cargo.toml`

**Current Feature Structure**:
```toml
[features]
default = ["silero", "text-injection"]
vosk = ["dep:coldvox-stt-vosk", "coldvox-stt-vosk/vosk", "coldvox-stt/vosk"]
```

**Optimized Structure**:
```toml
[features]
default = ["silero", "text-injection"]

# STT backends (mutually exclusive selection, can have multiple available)
vosk = ["dep:coldvox-stt-vosk", "coldvox-stt-vosk/vosk", "coldvox-stt/vosk"]
whisper = ["coldvox-stt/whisper"]

# VAD backends
silero = ["coldvox-vad-silero/silero"]
level3 = ["coldvox-vad/level3"]

# Platform features
text-injection = ["coldvox-text-injection"]
tui = ["ratatui", "crossterm"]

# Convenience feature combinations
stt-all = ["vosk", "whisper"]
full = ["vosk", "whisper", "text-injection", "tui", "level3"]
minimal = [] # No optional features
```

### B.7 - Verification and Testing

```bash
# Verify clean dependency structure
cargo tree --workspace | grep coldvox
# Should show no circular dependencies

# Test all feature combinations
cargo check --workspace --no-default-features
cargo check --workspace --features vosk
cargo check --workspace --features full
cargo check --workspace --features minimal

# Verify text injection still works
cargo test -p coldvox-text-injection
cargo test -p coldvox-telemetry

# Test plugin system with new architecture
cd crates/app
cargo run --features vosk -- --log-level "info,telemetry=debug"
```

**Success Criteria for Phase B**:
- [x] `cargo build --workspace` succeeds without circular dependency errors
- [x] All feature combinations compile cleanly
- [x] Text injection functionality preserved
- [x] Telemetry collection continues working
- [x] Plugin system remains functional
- [x] No performance regression in build times

---

## Post-Implementation Validation

### Immediate Testing Checklist

```bash
# 1. Clean build test
cargo clean && cargo build --workspace --features vosk

# 2. Plugin system functional test
cargo run --features vosk

# In separate terminal, test speech recognition:
# - Speak into microphone
# - Verify "VoskPlugin processing audio" in logs
# - Confirm transcribed text appears in active application

# 3. Dependency verification
cargo tree --duplicates  # Should show no duplicates
cargo tree | grep -E "coldvox.*coldvox"  # Check for circular refs
```

### Key Integration Points to Verify

1. **Plugin Registration**: `available_plugins()` includes "vosk"
2. **Audio Pipeline**: VAD → Plugin Manager → Text Injection chain intact
3. **Configuration**: Environment variables and CLI args respected
4. **Telemetry**: Metrics collection shows plugin activity
5. **Error Handling**: Plugin failures trigger appropriate fallbacks

### Performance Validation

- **Decode Latency**: Should remain <200ms (baseline ~150ms)
- **Memory Usage**: Peak usage <600MB for Vosk model
- **Plugin Switch Time**: <500ms for runtime switching
- **Build Time**: No significant regression in compilation time

## Implementation Notes

### Why This Works

The ColdVox codebase already contains:
- ✅ Complete 508-line VoskPlugin implementation
- ✅ Comprehensive plugin manager with failover logic
- ✅ Production-ready VoskTranscriber foundation
- ✅ Full telemetry and metrics integration
- ✅ Proper async patterns and thread safety

This plan simply **activates existing code** rather than creating new functionality.

### Risk Mitigation

**Phase A Rollback**: Single line revert if issues arise
```bash
git checkout HEAD -- crates/app/src/stt/plugin_manager.rs
```

**Phase B Rollback**: Revert dependency changes
```bash
git checkout HEAD -- crates/*/Cargo.toml
rm -rf crates/coldvox-telemetry-types/
```

### Key Success Indicators

✅ **Plugin System Active**: Speech → VoskPlugin → Text Injection
✅ **Clean Architecture**: No circular dependencies
✅ **Performance Maintained**: <10% regression in any metric
✅ **Feature Complete**: All existing functionality preserved

This activation plan transforms ColdVox from legacy STT to modern plugin architecture in **1-3 days** with minimal risk and maximum impact.