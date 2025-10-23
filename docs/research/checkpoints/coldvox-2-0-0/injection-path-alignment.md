---
doc_type: research
subsystem: text-injection
version: 1.0.0
status: draft
owners: Documentation Working Group
last_reviewed: 2025-10-19
---

# Injection Path Alignment - Implementation Summary

**Date**: 2025-10-09  
**Branch**: `injection-orchestrator-lean`  
**Status**: ✅ Complete - All tests passing

## Problem Statement

The text injection system had multiple misalignments:

1. **Triple Mode Decision Logic** - Paste vs keystroke decision was made in three separate places:
   - `manager.rs` (lines 912-916)
   - `processor.rs` (lines 157-166, 188-197)
   - `atspi.rs` (lines 486-490)

2. **Unused Chunking Methods** - `chunk_and_paste()` and `pace_type_text()` in `manager.rs` were never called

3. **Context Lost in Trait** - `TextInjector` trait only accepted `text: &str`, throwing away pre-warmed data from individual injector methods

4. **Orchestrator Bypass** - `StrategyOrchestrator` called injectors directly, bypassing `StrategyManager`

## Solution: Unified Injection Context

### 1. New Unified Types (`types.rs`)

```rust
/// Injection mode override (paste vs keystroke decision)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InjectionMode {
    Paste,
    Keystroke,
}

/// Unified injection context passed to all injectors
#[derive(Debug, Clone, Default)]
pub struct InjectionContext {
    pub target_app: Option<String>,
    pub window_id: Option<String>,
    pub atspi_focused_node_path: Option<String>,
    pub clipboard_backup: Option<String>,
    pub mode_override: Option<InjectionMode>,  // 👈 Centralized decision
}
```

### 2. Updated TextInjector Trait (`lib.rs`)

**Before:**
```rust
async fn inject_text(&self, text: &str) -> InjectionResult<()>;
```

**After:**
```rust
async fn inject_text(
    &self, 
    text: &str, 
    context: Option<&InjectionContext>  // 👈 Context with mode override
) -> InjectionResult<()>;
```

### 3. Centralized Mode Decision (`manager.rs`)

**Before:** Each component decided paste vs keystroke independently

**After:** `StrategyManager` decides once and passes via context:

```rust
// Determine injection method based on config (ONE PLACE)
let injection_mode = match self.config.injection_mode.as_str() {
    "paste" => InjectionMode::Paste,
    "keystroke" => InjectionMode::Keystroke,
    "auto" => {
        if text.len() > self.config.paste_chunk_chars as usize {
            InjectionMode::Paste
        } else {
            InjectionMode::Keystroke
        }
    }
    _ => /* default auto logic */
};

// Create injection context with mode override
let context = InjectionContext {
    target_app: Some(app_id.clone()),
    mode_override: Some(injection_mode),  // 👈 Pass decision
    ..Default::default()
};

// All injectors receive the context
injector.inject_text(text, Some(&context)).await
```

### 4. Injector Respects Override

**AtspiInjector (`injectors/atspi.rs`):**
```rust
pub async fn inject(&self, text: &str, context: &InjectionContext) -> InjectionResult<()> {
    // Determine injection method based on context override or configuration
    let use_paste = if let Some(mode_override) = context.mode_override {
        match mode_override {
            InjectionMode::Paste => true,
            InjectionMode::Keystroke => false,
        }
    } else {
        // Fall back to config-based decision
        match self.config.injection_mode.as_str() { /* ... */ }
    };
    
    if use_paste {
        self.paste_text(text, context).await
    } else {
        self.insert_text(text, context).await
    }
}
```

### 5. Processor Simplification (`processor.rs`)

**Before:** Processor duplicated mode decision logic in `check_and_inject()` and `force_inject()`

**After:** Removed duplicate logic, delegates to `StrategyManager`:

```rust
pub async fn check_and_inject(&mut self) -> anyhow::Result<()> {
    if self.session.should_inject() {
        // Mode decision is now centralized in StrategyManager
        self.perform_injection().await?;
    }
    Ok(())
}
```

### 6. Orchestrator Alignment (`orchestrator.rs`)

Now passes context to injectors:

```rust
let context = InjectionContext::default();
injector.inject_text(text, Some(&context)).await
```

## Files Changed

### Core Types
- `crates/coldvox-text-injection/src/types.rs` - Added `InjectionMode` and `InjectionContext`
- `crates/coldvox-text-injection/src/lib.rs` - Updated `TextInjector` trait signature

### Injectors
- `crates/coldvox-text-injection/src/injectors/atspi.rs` - Uses unified context, respects mode override
- `crates/coldvox-text-injection/src/injectors/clipboard.rs` - Uses unified context
- `crates/coldvox-text-injection/src/noop_injector.rs` - Updated trait impl
- `crates/coldvox-text-injection/src/ydotool_injector.rs` - Updated trait impl
- `crates/coldvox-text-injection/src/clipboard_paste_injector.rs` - Updated trait impl
- `crates/coldvox-text-injection/src/enigo_injector.rs` - Updated trait impl
- `crates/coldvox-text-injection/src/kdotool_injector.rs` - Updated trait impl

### Managers
- `crates/coldvox-text-injection/src/manager.rs` - Centralized mode decision, passes context
- `crates/coldvox-text-injection/src/processor.rs` - Removed duplicate mode logic
- `crates/coldvox-text-injection/src/orchestrator.rs` - Uses unified context

### Tests
- `crates/coldvox-text-injection/src/confirm.rs` - Fixed test expectations

## Benefits

1. **Single Source of Truth** - Mode decision happens once in `StrategyManager`
2. **Pre-warmed Data Preserved** - Context flows through entire injection path
3. **Extensibility** - Easy to add more context data (focus info, clipboard state, etc.)
4. **Consistency** - All injectors receive same context and make consistent decisions
5. **Testability** - Can inject specific mode overrides for testing

## Test Results

```
test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

All existing tests pass with no behavioral regressions.

## Migration Notes

### For Injector Implementations

Old:
```rust
async fn inject_text(&self, text: &str) -> InjectionResult<()>
```

New:
```rust
async fn inject_text(&self, text: &str, context: Option<&InjectionContext>) -> InjectionResult<()> {
    let default_context = InjectionContext::default();
    let ctx = context.unwrap_or(&default_context);
    // Use ctx.mode_override if present
}
```

### Deprecated Types

- `injectors::atspi::Context` → Use `InjectionContext`
- `injectors::clipboard::Context` → Use `InjectionContext`

Both are aliased for backward compatibility with deprecation warnings.

## Future Work

1. **Pre-warming Integration** - Populate `atspi_focused_node_path` and `clipboard_backup` in context
2. **Remove Unused Methods** - Consider removing `chunk_and_paste()` / `pace_type_text()` if truly unused
3. **Orchestrator Enhancement** - Have orchestrator use `StrategyManager` instead of direct injector calls
4. **Context Builder** - Add builder pattern for complex context construction

## Verification

To verify the alignment:
```bash
cd crates/coldvox-text-injection
cargo test --lib
```

All tests should pass with no warnings about mismatched signatures.
