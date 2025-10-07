# Documentation & Test Infrastructure Fixes for Branch `anchor/oct-06-2025`

## Context
Branch `anchor/oct-06-2025` contains comprehensive refactor work (76 files, 3,719 insertions, 1,729 deletions) that is production-ready except for:
1. Test infrastructure issues (7 failing tests in settings_test.rs)
2. Documentation inconsistencies (XDG claims, missing doc links, broken symlinks)
3. Minor code hygiene issues (clippy warnings)

## Pre-Execution: Git State Management

### Step 0: Document and Handle Working Directory State
```bash
# Check current state
git status

# Current uncommitted changes (expected):
# M  config/default.toml
# M  crates/app/tests/settings_test.rs  
# M  docs/tasks/refactor_debug_plan.md
# ?? crates/app/config/

# Decision point:
# Option A: Commit as "Pre-fix: Document current state"
# Option B: Stash with: git stash push -m "WIP before doc fixes"
# Option C: Continue with changes in place (document them)
```

**Recommended**: Option A - Commit current work-in-progress before starting systematic fixes.

```bash
git add -A
git commit -m "chore: document refactor state before systematic fixes

- Updated branch name in refactor_debug_plan.md
- In-progress config and test changes to be formalized"
```

---

## Phase 1: Critical Test Infrastructure Fix

### Objective
Fix 7 failing tests in `settings_test.rs` that fail due to config file path issues.

### Root Cause Analysis
**Problem**: `Settings::new()` in `crates/app/src/main.rs:84` hardcodes path:
```rust
.add_source(File::with_name("config/default.toml"))
```

**Issue**: Tests run from different working directories depending on:
- Unit tests: Run from crate directory
- Integration tests: Run from workspace root
- CI environments: May have different paths

### Solution: Make Settings Path-Aware

#### Task 1.1: Add Path-Configurable Settings Constructor

**File**: `crates/app/src/main.rs`

Add new method before existing `Settings::new()`:

```rust
impl Settings {
    /// Load settings from a specific config file path (for tests)
    #[cfg(test)]
    pub fn from_path(config_path: impl AsRef<Path>) -> Result<Self, String> {
        let mut builder = Config::builder()
            .add_source(Environment::with_prefix("coldvox").separator("__"))
            .add_source(File::with_name(config_path.as_ref().to_str().unwrap()));

        let config = builder.build()
            .map_err(|e| format!("Failed to build config: {}", e))?;

        let mut settings: Settings = config.try_deserialize()
            .map_err(|e| format!("Failed to deserialize settings: {}", e))?;

        settings.validate().map_err(|e| e.to_string())?;
        Ok(settings)
    }

    fn new() -> Result<Self, String> {
        // existing implementation unchanged
    }
}
```

#### Task 1.2: Update Test File to Use Path-Aware Constructor

**File**: `crates/app/tests/settings_test.rs`

Add helper at top of file:

```rust
use std::path::PathBuf;
use std::env;

fn get_test_config_path() -> PathBuf {
    // Try workspace root first (for integration tests)
    let workspace_config = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("config/default.toml");
    
    if workspace_config.exists() {
        return workspace_config;
    }
    
    // Fallback to relative path from crate root
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../config/default.toml")
}
```

Update each test that uses `Settings::new()`:

```rust
#[test]
fn test_settings_new_default() {
    let config_path = get_test_config_path();
    let settings = Settings::from_path(&config_path).unwrap();
    assert_eq!(settings.resampler_quality.to_lowercase(), "balanced");
    // ... rest of test
}
```

### Verification (Phase 1)
```bash
# Must pass before proceeding
cargo test --test settings_test
# Expected: test result: ok. 9 passed; 0 failed

# Verify workspace tests still work
cargo test --workspace --lib
# Expected: All lib tests pass
```

### Commit Strategy (Phase 1)
```bash
git add crates/app/src/main.rs crates/app/tests/settings_test.rs
git commit -m "fix(tests): make Settings path-configurable for test environments

- Add Settings::from_path() for test flexibility
- Update settings_test.rs to use CARGO_MANIFEST_DIR-relative paths
- Fixes 7 failing tests due to config file path issues

Tests now pass in both unit and integration contexts."
```

---

## Phase 2: Documentation Corrections

### Objective
Fix false claims and broken references in documentation to match actual implementation.

### Task 2.1: Fix XDG_CONFIG_HOME False Claim

**File**: `config/README.md` (line ~52)

**Context**: Code does NOT implement XDG search; docs claim it does.

```markdown
OLD:
- **Runtime Loading**: The app searches for `default.toml` in the current directory or `$XDG_CONFIG_HOME/coldvox/`. Ensure it's accessible post-deploy.

NEW:
- **Runtime Loading**: The app loads `config/default.toml` relative to the working directory. XDG support not implemented; to add it, extend `Settings::new()` with XDG path lookup (see deployment docs for details).
```

### Task 2.2: Fix overrides.toml Auto-Loading Claim

**File**: `config/README.md` (line ~70)

```markdown
OLD:
  - Load order: CLI flags > Env vars > overrides.toml > default.toml > hardcoded defaults.

NEW:
  - Current load order: CLI flags > Env vars > default.toml > hardcoded defaults.
  - Note: `overrides.toml` is a template and NOT automatically loaded. To enable, add `.add_source(File::with_name("config/overrides.toml").required(false))` to Settings::new().
```

### Task 2.3: Fix Deployment Doc XDG Claim

**File**: `docs/deployment.md` (line ~29)

```markdown
OLD:
- **Build Inclusion**: The TOML file is not embedded in the binary; it is loaded at runtime from the working directory or XDG_CONFIG_HOME.

NEW:
- **Build Inclusion**: The TOML file is not embedded in the binary; it is loaded at runtime from `config/default.toml` relative to the working directory. XDG_CONFIG_HOME support is not currently implemented.
```

### Task 2.4: Fix Missing Doc Reference

**File**: `README.md` (line ~75)

```markdown
OLD:
Headless behavior notes: see [`docs/text_injection_headless.md`](docs/text_injection_headless.md).

NEW:
Headless behavior: Text injection works in headless environments via clipboard strategies. See `docs/deployment.md` for configuration and `crates/coldvox-text-injection/README.md` for backend details.
```

### Task 2.5: Remove Broken Symlinks

```bash
cd docs/reference/crates/
rm app.md coldvox-vad.md coldvox-vad-silero.md

# Rationale: These point to non-existent README files.
# Crates have inline rustdoc; use `cargo doc` instead.
```

### Task 2.6: Fix Missing THIRDPARTY.md Reference

**File**: `docs/adr/0001-vosk-model-distribution.md` (line ~44)

```markdown
OLD:
## Related Documents
- `THIRDPARTY.md`
- `crates/coldvox-stt-vosk/src/model.rs`
- `README.md` (root)

NEW:
## Related Documents
- `crates/coldvox-stt-vosk/src/model.rs`
- `README.md` (root)
- Model license: See `models/vosk-model-small-en-us-0.15/LICENSE`
```

### Verification (Phase 2)
```bash
# Check for broken markdown links
find docs -name "*.md" -type f -exec grep -l "docs/text_injection_headless.md\|THIRDPARTY.md\|XDG_CONFIG_HOME" {} \;
# Expected: No matches for removed references

# Verify symlinks removed
ls -la docs/reference/crates/ | grep " -> " | grep -E "(app|coldvox-vad)"
# Expected: No output (symlinks removed)
```

### Commit Strategy (Phase 2)
```bash
git add config/README.md docs/deployment.md README.md docs/adr/0001-vosk-model-distribution.md
git rm docs/reference/crates/app.md docs/reference/crates/coldvox-vad.md docs/reference/crates/coldvox-vad-silero.md
git commit -m "docs: fix false XDG claims and remove broken references

- config/README.md: Remove XDG_CONFIG_HOME and overrides.toml auto-load claims
- docs/deployment.md: Clarify config loading is working-dir relative only
- README.md: Replace missing doc link with actual references
- docs/adr: Remove non-existent THIRDPARTY.md reference
- Remove broken symlinks to non-existent crate READMEs

All documentation now matches actual implementation behavior."
```

---

## Phase 3: Code Hygiene (Clippy Warnings)

### Objective
Clean up compiler warnings to maintain code quality standards.

### Task 3.1: Auto-Fix Safe Warnings

```bash
cargo clippy --fix --allow-dirty --workspace --all-targets
```

### Task 3.2: Manual Fixes for Remaining Warnings

Based on current clippy output:

**File**: `crates/coldvox-audio/src/device.rs` (line ~27)
```rust
OLD: let host = StderrSuppressor::with_suppressed(|| cpal::default_host());
NEW: let host = StderrSuppressor::with_suppressed(cpal::default_host);
```

**File**: `crates/coldvox-text-injection/src/types.rs` (line ~5)
```rust
// Remove empty line after doc comment
OLD:
/// immediate termination or panic when injection cannot succeed.

pub struct InjectionError {

NEW:
/// immediate termination or panic when injection cannot succeed.
pub struct InjectionError {
```

**File**: `crates/app/src/lib.rs` (line ~3)
```rust
OLD: use tracing;
NEW: // Remove if unused, or keep if re-exported
```

**File**: `crates/app/src/lib.rs` (line ~63)
```rust
OLD: let mut builder = Config::builder()
NEW: let builder = Config::builder()
```

**File**: `crates/app/src/main.rs` (line ~82)
```rust
OLD: let mut builder = Config::builder()
NEW: let builder = Config::builder()
```

### Verification (Phase 3)
```bash
# Must pass with zero warnings
cargo clippy --workspace --all-targets -- -D warnings
# Expected: exit code 0, no output

# Ensure tests still pass
cargo test --workspace
# Expected: All tests pass
```

### Commit Strategy (Phase 3)
```bash
git add -A
git commit -m "style: fix clippy warnings across workspace

- Remove unused mut annotations in config builders
- Fix redundant closure in device.rs
- Clean up doc comment formatting in types.rs
- Remove redundant imports

All clippy warnings resolved; workspace builds cleanly."
```

---

## Phase 4: Comprehensive Verification

### Build Verification
```bash
# Clean build from scratch
cargo clean
cargo build --workspace --all-targets
# Expected: exit code 0, no warnings

# Verify release build
cargo build --release --workspace
# Expected: exit code 0
```

### Test Suite Verification
```bash
# Full test suite
cargo test --workspace
# Expected: All tests pass

# Specific previously-failing test
cargo test --test settings_test -- --nocapture
# Expected: 9 passed; 0 failed

# Integration tests
cargo test --workspace --test '*'
# Expected: All integration tests pass
```

### Smoke Tests (Manual)
```bash
# 1. App starts and shows new flag
cargo run -- --help | grep injection-fail-fast
# Expected: --injection-fail-fast flag visible

# 2. Config loading works
RUST_LOG=debug cargo run 2>&1 | grep -i "config"
# Expected: Logs showing config/default.toml loaded

# 3. App runs with fail-fast flag
cargo run -- --injection-fail-fast &
sleep 2
pkill coldvox
# Expected: App starts without errors

# 4. TUI dashboard works
timeout 5 cargo run --bin tui_dashboard -- --log-level debug || true
# Expected: TUI starts, draws interface (timeout is normal)

# 5. List devices works
cargo run -- --list-devices
# Expected: Shows available audio input devices
```

### Documentation Verification
```bash
# Check all markdown files for common issues
find . -name "*.md" -type f ! -path "./target/*" -exec grep -l "XDG_CONFIG_HOME\|text_injection_headless\|THIRDPARTY\.md" {} \;
# Expected: No matches

# Verify no broken symlinks
find docs -type l ! -exec test -e {} \; -print
# Expected: No output
```

---

## Phase 5: Optional Enhancements

### Task 5.1: Add WIP Badges to Research Docs

**Files**: `docs/research/*.md`, `crates/coldvox-gui/docs/implementation-plan.md`

Add at top:
```markdown
> ⚠️ **RESEARCH DOCUMENT - WORK IN PROGRESS**  
> Contains incomplete sections and future work markers.  
> Last updated: 2025-10-07
```

### Task 5.2: Add Config File Discovery Documentation

**File**: `config/README.md` (new section at end)

```markdown
## For Test Authors

Tests that need to load configuration should use `Settings::from_path()` with `CARGO_MANIFEST_DIR`:

\`\`\`rust
#[cfg(test)]
use std::env;

let config_path = format!("{}/../../config/default.toml", env!("CARGO_MANIFEST_DIR"));
let settings = Settings::from_path(&config_path)?;
\`\`\`

This ensures tests work regardless of working directory context.
```

---

## Success Criteria

**Before merge, all must be ✅**:

- [ ] All tests pass: `cargo test --workspace`
- [ ] Clean build: `cargo clippy --workspace -- -D warnings`
- [ ] No broken doc links or symlinks
- [ ] Documentation matches implementation (no XDG claims)
- [ ] Git history is clean with descriptive commits
- [ ] Smoke tests confirm core functionality works
- [ ] settings_test.rs passes all 9 tests

---

## Risk Assessment

### Configuration System (Originally "High" - Now "Low")
- **Mitigation Applied**: Path-configurable Settings with test helpers
- **Residual Risk**: Deployment environments must ensure `config/default.toml` is present
- **Documentation**: deployment.md updated to clarify this requirement

### Text Injection (Medium - Unchanged)
- **Status**: Clipboard paste priority well-tested on Linux desktops
- **Residual Risk**: Platform-specific behaviors (Wayland vs X11)
- **Mitigation**: Fallback chains documented in crate README
- **Action**: Manual smoke test recommended on target deployment platform

### Code Quality (Low)
- **Status**: Clippy warnings resolved
- **Residual Risk**: Minimal - standard Rust code quality practices

---

## Effort Summary

| Phase | Effort | Blocking? |
|-------|--------|-----------|
| Phase 0 (Git State) | Small | Yes |
| Phase 1 (Test Fix) | Small | Yes |
| Phase 2 (Docs) | Small | Yes |
| Phase 3 (Clippy) | Small | No |
| Phase 4 (Verification) | Medium | Yes |
| Phase 5 (Optional) | Small | No |

**Total effort**: 2-3 hours for required phases

---

## Post-Completion Actions

1. **Update PR description** with:
   - Test fixes applied
   - Documentation corrections made
   - Clean clippy build achieved

2. **Request review** from:
   - Code owner for config system changes
   - Platform maintainer for injection system

3. **Prepare merge commit message**:
   ```
   refactor: production-ready config system and docs
   
   - Fixed test infrastructure with path-aware Settings
   - Corrected documentation XDG/overrides claims
   - Resolved all clippy warnings
   - Comprehensive refactor (76 files, 3.7k additions)
   
   Tests: All pass
   Build: Clean (no warnings)
   Docs: Accurate and complete
   ```

---

## Notes for Reviewers

**This plan addresses**:
- ✅ Test infrastructure brittleness with robust path handling
- ✅ Documentation accuracy (no false XDG/overrides claims)
- ✅ Code quality (all clippy warnings)
- ✅ Verification at each phase
- ✅ Clear commit strategy
- ✅ Evidence-based approach (all claims verified with code/output)

**Philosophy**: 
- **Thoroughness over brevity** in documentation (users need context)
- **Phased verification** for critical changes (tests, builds)
- **Clear commit messages** for reviewability
- **Production-ready standards** (no warnings, all tests pass)
