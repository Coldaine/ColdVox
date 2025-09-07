# ColdVox GUI Improvements Roadmap

## Status: Post-Fix Review
**Date:** September 6, 2025
**Current State:** Qt bridge compiles cleanly with proper feature gating

## Confirmed Working Elements

### Core Architecture ✅
- CXX-Qt codegen properly wired and feature-gated
- Generated bridge included only when `qt-ui` enabled
- Qt modules (Gui/Qml/Quick) linked correctly
- QML loaded at runtime with bridge exposed as context property

### Build System ✅
- Feature gating provides clean separation
- Non-Qt builds remain functional
- Build script has proper rerun-if-changed directives
- Generated code encapsulated in minimal `run_ui` module

## Recommended Improvements

### 1. Signal Naming Consistency
**Issue:** Property `level` auto-emits `levelChanged`, but custom signal `levelsChanged` (plural) creates confusion
**Location:** `src/bridge.rs:94`
**Fix:** Either remove the custom signal or rename to match property convention
```rust
// Option 1: Remove redundant signal (preferred)
// Option 2: Rename to levelChanged if manual emission needed
```

### 2. Export Enums to QML
**Issue:** `UiState` enum not accessible from QML, forcing use of raw integers
**Location:** `src/bridge.rs:11`
**Fix:** Add `#[qenum]` attribute
```rust
#[qenum]
#[derive(Clone, Copy, Debug)]
#[repr(i32)]
pub enum UiState {
    Idle = 0,
    Recording = 1,
    Processing = 2,
    Complete = 3,
}
```
**Benefit:** QML can use `UiState.Recording` instead of `1`

### 3. Clarify State Model
**Issue:** Pause concept folded into Idle/Recording toggle
**Location:** `src/bridge.rs:45`
**Options:**
```rust
pub enum UiState {
    Idle = 0,
    Recording = 1,
    Paused = 2,      // Add explicit paused state
    Processing = 3,
    Complete = 4,
}
```

### 4. Add QML Load Validation
**Issue:** No error handling if QML fails to load
**Location:** `src/main.rs:26`
**Fix:**
```rust
engine.load_url(&url);
if engine.root_objects().is_empty() {
    eprintln!("Failed to load QML from: {}", qml_path);
    std::process::exit(1);
}
```

### 5. Resource Packaging Strategy
**Issue:** QML loaded via filesystem path (fragile for distribution)
**Location:** `src/main.rs:24`
**Future Enhancement:**
- Use Qt Resource System (qrc) for embedded QML
- Ensures QML files included in binary
- Prevents path issues after installation

### 6. Method Signature Consistency
**Issue:** Using `Pin<&mut qobject::GuiBridge>` instead of `Pin<&mut Self>`
**Location:** Multiple methods in `src/bridge.rs`
**Fix:**
```rust
// Current
pub fn cmd_start(self: Pin<&mut qobject::GuiBridge>) { }

// Improved
pub fn cmd_start(self: Pin<&mut Self>) { }
```

## Implementation Priority

### Phase 1: API Polish (High Priority)
1. Fix signal naming consistency
2. Export enums to QML
3. Use `Self` in method signatures

### Phase 2: Robustness (Medium Priority)
4. Add QML load validation
5. Clarify state model with explicit Paused state

### Phase 3: Production Readiness (Future)
6. Implement qrc resource packaging
7. Add comprehensive error handling
8. Create installer-friendly paths

## Testing Validation

### Current Status ✅
```bash
cargo clean -p coldvox-gui
cargo check -p coldvox-gui --features qt-ui
# Completes without errors
```

### Next Steps
1. Implement Phase 1 improvements
2. Test QML enum access after adding `#[qenum]`
3. Verify signal emission with corrected naming
4. Add unit tests for bridge methods

## Architecture Notes

### Strengths Retained
- Clean feature separation
- Minimal API surface area
- Clear command/property model
- Demo helpers for UI testing

### Future Considerations
- Backend integration strategy
- Real-time audio level updates
- Transcript streaming approach
- Performance optimization for overlay rendering

## Conclusion

The GUI rebuild successfully addresses the CXX-Qt compilation issues while maintaining clean architecture. The suggested improvements will enhance API consistency, type safety, and production readiness without compromising the current working state.
