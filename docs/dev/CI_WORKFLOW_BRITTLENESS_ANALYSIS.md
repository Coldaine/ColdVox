# CI Workflow Brittleness Analysis

**Date**: 2025-10-08  
**Issue**: CI pipeline fails before any code testing due to overly strict dependency checking  
**Impact**: Cannot verify if code changes work because setup blocks everything

---

## Current Problem

### What Happens Now
```
┌─────────────────────────────────────┐
│ setup-coldvox action                │
│ ├─ Check 18 system commands        │  ❌ FAILS if openbox missing
│ ├─ Check 4 pkg-config libraries    │  ❌ FAILS if any missing
│ └─ exit 1 if ANY missing           │  🔥 ENTIRE CI DIES
└─────────────────────────────────────┘
         ↓ (never reached)
┌─────────────────────────────────────┐
│ cargo check (type checking)         │  ← Doesn't need X11 tools
│ cargo build (compilation)           │  ← Doesn't need X11 tools  
│ cargo test (unit tests)             │  ← 90% don't need X11 tools
└─────────────────────────────────────┘
```

**Result**: You never find out if your Rust code compiles because the workflow dies checking for `openbox`.

### Real Example (Today)
1. ✅ User installs `openbox` package
2. ❌ `pulseaudio` command not found (only libs installed)
3. 🔥 CI fails at setup
4. ❓ User doesn't know if config.toml changes even parse correctly

---

## Dependency Reality Check

### Actually Required for Compilation
| Tool | Why | Missing = Build Fails? |
|------|-----|------------------------|
| gcc, g++, make | Compile C dependencies | YES ✅ |
| pkg-config | Find system libraries | YES ✅ |
| alsa (pkg-config) | Audio capture | YES ✅ |

### Only Required for Specific Tests
| Tool | Why | Missing = Build Fails? |
|------|-----|------------------------|
| xdotool | Text injection tests | NO ❌ (skip tests) |
| Xvfb, openbox | Headless X11 server | NO ❌ (skip tests) |
| wl-paste, xclip | Clipboard tests | NO ❌ (skip tests) |
| ydotool | Wayland injection tests | NO ❌ (skip tests) |
| gtk+-3.0, at-spi-2.0 | Accessibility tests | NO ❌ (skip tests) |

### Never Used by Build
| Tool | Current Check | Actual Usage |
|------|---------------|--------------|
| wget, unzip | ✅ Verified | Used by setup scripts only |
| dbus-launch | ✅ Verified | Test infrastructure only |
| wmctrl, xprop | ✅ Verified | Test infrastructure only |

---

## Impact Analysis

### Jobs Blocked by Current Approach
```yaml
build_and_check:  # ❌ Dies if openbox missing
  - cargo check   # ← Doesn't need openbox
  - cargo build   # ← Doesn't need openbox
  - cargo test    # ← 90% of tests don't need openbox
  - cargo doc     # ← Doesn't need openbox
  - cargo clippy  # ← Doesn't need openbox

text_injection_tests:  # ❌ Dies if openbox missing  
  - Text injection tests  # ← ONLY job that needs openbox
```

### What You Lose
1. **No compilation feedback** - Can't tell if Rust code compiles
2. **No type checking** - Can't tell if code is type-safe
3. **No unit test results** - Can't tell if core logic works
4. **No linting feedback** - Can't tell if code has warnings
5. **All or nothing** - One missing tool blocks everything

---

## Recommended Solutions

### Option 1: Fail Fast Only on Build-Critical Tools (RECOMMENDED)

**Philosophy**: Let CI tell you what it CAN verify, not just what it CAN'T.

```yaml
# .github/actions/setup-coldvox/action.yml (refactored)
steps:
  - name: Verify core build dependencies
    run: |
      # HARD FAIL - can't compile without these
      required: gcc g++ make pkg-config
      required_pkgs: alsa
      
      if missing → exit 1 ❌
  
  - name: Check text injection dependencies
    if: inputs.verify-text-injection == 'true'
    run: |
      # SOFT WARN - tests can adapt/skip
      optional: xdotool Xvfb openbox wl-paste xclip ydotool
      optional_pkgs: gtk+-3.0 at-spi-2.0 xtst
      
      if missing → warning ⚠️  (continue)
```

**Benefits**:
- ✅ Compilation always runs
- ✅ Unit tests always run
- ⚠️ Text injection tests skip if tools missing
- 📊 You get partial results instead of total failure

### Option 2: Job-Level Dependency Checks

Move verification to only the jobs that need it:

```yaml
build_and_check:
  steps:
    - uses: ./.github/actions/setup-coldvox
      # No verification - just Rust toolchain
    - run: cargo build  # ← Always works

text_injection_tests:
  steps:
    - uses: ./.github/actions/setup-coldvox
      with:
        verify-text-injection: true  # ← Only check here
    - run: |
        if ! command -v xdotool; then
          echo "Skipping tests - xdotool not available"
          exit 0
        fi
        cargo test -p coldvox-text-injection
```

**Benefits**:
- ✅ Build job never blocked by text injection tools
- ✅ Text injection tests self-validate
- ✅ Other jobs unaffected

### Option 3: Test-Level Capability Detection (BEST FOR MATURE CI)

Make tests themselves check for capabilities:

```rust
// coldvox-text-injection/tests/integration_test.rs
#[test]
#[ignore = "requires_xdotool"]
fn test_x11_injection() {
    if !has_xdotool() {
        eprintln!("Skipping: xdotool not available");
        return;
    }
    // actual test
}
```

Run in CI:
```bash
cargo test -- --include-ignored  # Runs all if tools available
cargo test                       # Skips tool-dependent tests
```

**Benefits**:
- ✅ Tests self-document requirements
- ✅ Works on any machine
- ✅ Developers can run locally without full setup

---

## Comparison Table

| Approach | Build Always Works | Tests Adapt | Setup Complexity |
|----------|-------------------|-------------|------------------|
| **Current** (hard fail) | ❌ No | ❌ No | 🟢 Simple |
| **Option 1** (soft fail) | ✅ Yes | ⚠️ Partial | 🟢 Simple |
| **Option 2** (job-level) | ✅ Yes | ✅ Yes | 🟡 Moderate |
| **Option 3** (test-level) | ✅ Yes | ✅ Yes | 🔴 Complex |

---

## Immediate Action Plan

### Phase 1: Stop Blocking Builds (Today)
1. Replace `action.yml` with `action-refactored.yml`
2. Update `ci.yml` to pass `verify-text-injection: true` only to text_injection_tests job
3. Push and verify builds work even without openbox

### Phase 2: Document Runner Provisioning (This Week)
1. Create `docs/dev/RUNNER_SETUP.md` with exact dnf commands
2. Add to runner health check script
3. Make it clear which tools are required vs optional

### Phase 3: Smart Test Skipping (Future)
1. Add capability detection to text injection tests
2. Use `#[ignore = "requires_X"]` attributes
3. CI runs all tests, local devs can skip expensive ones

---

## Questions to Consider

1. **Do you actually need ALL text injection backends tested on every PR?**
   - Maybe just test one (AT-SPI) and integration test the manager?

2. **Should text injection tests be separate from main CI?**
   - Could run on nightly schedule instead of every commit

3. **Is the self-hosted runner the right approach?**
   - Pro: Fast, persistent cache, exactly matches your system
   - Con: Brittle when system changes, hard to reproduce elsewhere

4. **What's the actual test coverage of text injection?**
   - If tests are mostly "does it compile?" → don't need full X11 setup
   - If tests validate actual injection → need proper setup

---

## Recommendation

**Use Option 1 (Fail Fast Only on Build-Critical Tools) immediately.**

Why:
- ✅ Smallest change to existing workflow
- ✅ Unblocks you TODAY
- ✅ Still catches real provisioning issues
- ✅ Gives you data about what works even when something breaks
- ⚠️ Clear warnings show what's missing without killing everything

Then evaluate Option 3 (test-level detection) for long-term maintainability.

---

## See Also
- `.github/actions/setup-coldvox/action-refactored.yml` - Proposed implementation
- `docs/dev/RUNNER_SETUP.md` - Runner provisioning guide (TODO)
- `TESTING.md` - Test strategy and requirements
