# Optional Tests in ColdVox

**Summary**: Tests that depend on external resources, display servers, or specific system configurations.

---

## Categories of Optional Tests

### 1. Text Injection Tests (Require Display Server)

#### Location: `crates/coldvox-text-injection/src/tests/`

**Feature gate**: `#[cfg(all(test, feature = "real-injection-tests"))]`

**Environment requirement**: `DISPLAY` or `WAYLAND_DISPLAY` must be set

**Tests**:

- **`real_injection_smoke.rs`** - Smoke test for all real injection backends
  - Test function: `real_injection_smoke()`
  - Requires: `RUN_REAL_INJECTION_SMOKE=1` environment variable
  - Backends tested: AT-SPI, Clipboard (wl-clipboard), ydotool, enigo
  - Duration: ~5-10 seconds (fast, adaptive timeouts)
  - Purpose: Quick validation that backends work

- **`real_injection.rs`** - Comprehensive real injection tests
  - Test functions:
    - `harness_self_test_launch_gtk_app()` - Verify GTK test app launches
    - `run_atspi_test()` - AT-SPI backend integration
    - `run_clipboard_paste_test()` - Clipboard + paste workflow
    - `run_ydotool_test()` - ydotool backend integration
    - `run_enigo_typing_test()` - enigo typing (not paste)
  - Duration: ~30-60 seconds (full suite)
  - Purpose: Thorough validation of each backend

**Why optional**:
- ❌ Requires X11/Wayland display server
- ❌ Requires GTK3 dev libraries
- ❌ Requires running accessibility services (AT-SPI)
- ❌ Requires ydotool daemon (for ydotool tests)
- ❌ CI environments may not have GUI

**How to run**:
```bash
# Smoke test only (fast)
RUN_REAL_INJECTION_SMOKE=1 cargo test -p coldvox-text-injection \
  --features real-injection-tests,atspi,wl_clipboard,enigo,ydotool \
  -- real_injection_smoke

# Full suite
cargo test -p coldvox-text-injection \
  --features real-injection-tests,atspi,wl_clipboard,enigo,ydotool
```

**Current CI strategy**:
- ✅ Marked `continue-on-error: true` in `ci-minimal.yml`
- ✅ Skipped if display server not available
- ✅ Non-blocking (warns but doesn't fail PR)

---

### 2. Vosk E2E Tests (Require Model Files)

#### Location: `crates/app/src/stt/tests/end_to_end_wav.rs`

**Feature gate**: `#[cfg(feature = "vosk")]`

**Environment requirement**: Vosk model directory must exist

**Tests**:

- **`test_end_to_end_wav_pipeline()`** - Main E2E test
  - **Default**: Runs 1 random WAV file (~5-10 seconds)
  - **All mode**: `TEST_WAV_MODE=all` runs all WAV files (~2-5 minutes)
  - Requires: WAV files in `test_data/` with matching `.txt` transcripts
  - Model: `VOSK_MODEL_PATH` or default `models/vosk-model-small-en-us-0.15`
  - Purpose: Validate full audio → VAD → STT → transcription pipeline

- **`test_end_to_end_with_real_injection()`** - E2E with text injection
  - **Status**: `#[ignore]` (skipped by default)
  - Requires: Vosk model + display server + text injection backends
  - Duration: ~20-30 seconds
  - Purpose: Validate full pipeline including text injection output

- **`test_atspi_injection()`** - AT-SPI specific E2E
  - **Status**: Not ignored, but guards with availability checks
  - Requires: Display server + AT-SPI services + terminal emulator
  - Duration: ~10 seconds
  - Purpose: Test AT-SPI injection in isolation

- **`test_clipboard_injection()`** - Clipboard specific E2E
  - **Status**: Not ignored, but guards with availability checks
  - Requires: Display server + clipboard backend (wl-paste/xclip)
  - Duration: ~10 seconds
  - Purpose: Test clipboard injection workflow

**Why optional**:
- ❌ Requires Vosk model download (40MB - 1.8GB)
- ❌ Model extraction takes time (~30 seconds for large model)
- ❌ Some tests require display server (text injection variants)
- ❌ Long-running for full test suite

**How to run**:
```bash
# Default (1 random WAV, ~5-10s)
cargo test -p coldvox-app --features vosk test_end_to_end_wav_pipeline -- --nocapture

# All WAVs (full suite, ~2-5 min)
TEST_WAV_MODE=all cargo test -p coldvox-app --features vosk test_end_to_end_wav_pipeline -- --nocapture

# With real injection (requires display)
cargo test -p coldvox-app --features vosk,text-injection test_end_to_end_with_real_injection -- --ignored --nocapture
```

**Current CI strategy**:
- ✅ Marked `continue-on-error: true` in `ci-minimal.yml`
- ✅ Skipped if model not available
- ✅ Non-blocking (warns but doesn't fail PR)
- ✅ Runs default mode (1 WAV) for speed

---

### 3. Integration Tests (Require Multiple Systems)

#### Location: `crates/app/tests/integration/`

**Tests**:

- **`text_injection_integration_test.rs`**
  - Test: `test_text_injection_end_to_end()`
  - Requires: STT system + text injection system + metrics tracking
  - Duration: ~5 seconds
  - Purpose: Validate cross-crate integration

**Why partially optional**:
- ⚠️ Works with mocked backends (always runs)
- ❌ Real injection requires display server (optional)
- ✅ Core integration logic testable without GUI

**How to run**:
```bash
cargo test -p coldvox-app --test text_injection_integration_test
```

**Current CI strategy**:
- ✅ Runs by default (uses mocked backends)
- ✅ Real injection gated by display availability

---

### 4. Pre-Commit Optional Tests

#### Location: `.git-hooks/pre-commit-injection-tests`

**Tests run**:
1. **Mock tests** (always, required):
   ```bash
   cargo test -p coldvox-text-injection --lib
   ```
   - Duration: ~2 seconds
   - Status: **Required** (blocks commit if fails when `ENFORCE_REAL_SMOKE=1`)

2. **Real injection smoke test** (optional, if display available):
   ```bash
   RUN_REAL_INJECTION_SMOKE=1 cargo test -p coldvox-text-injection \
     --features real-injection-tests,atspi,wl_clipboard,enigo,ydotool \
     -- real_injection_smoke --test-threads=1 --quiet
   ```
   - Duration: ~5-10 seconds
   - Status: **Non-blocking** (warns only, unless `ENFORCE_REAL_SMOKE=1`)

3. **Vosk E2E test** (optional, if model available):
   - Chained from another pre-commit hook
   - Duration: ~5-10 seconds (1 WAV)
   - Status: **Non-blocking**

**How pre-commit handles optional tests**:
```bash
# Check environment
if [[ -z "${DISPLAY:-}" && -z "${WAYLAND_DISPLAY:-}" ]]; then
    echo "[smoke] Skipping real injection smoke test (no display)"
else
    # Run but don't block
    set +e
    cargo test ... -- real_injection_smoke
    rc=$?
    if [[ $rc -ne 0 ]]; then
        if [[ "${ENFORCE_REAL_SMOKE:-}" == "1" ]]; then
            exit $rc  # Block commit
        else
            echo "[smoke] Warning: failed – continuing"  # Just warn
        fi
    fi
    set -e
fi
```

---

## Summary Table

| Test Suite | Location | Duration | Requires | Blocking? |
|------------|----------|----------|----------|-----------|
| **Unit tests** | All crates | 2-5s | Nothing | ✅ Yes |
| **Integration tests** | `crates/app/tests/` | 5-10s | Mocked backends | ✅ Yes |
| **Text injection smoke** | `coldvox-text-injection/src/tests/real_injection_smoke.rs` | 5-10s | Display server | ❌ No |
| **Text injection full** | `coldvox-text-injection/src/tests/real_injection.rs` | 30-60s | Display + AT-SPI + ydotool | ❌ No |
| **Vosk E2E (1 WAV)** | `crates/app/src/stt/tests/end_to_end_wav.rs` | 5-10s | Vosk model | ❌ No |
| **Vosk E2E (all WAVs)** | `crates/app/src/stt/tests/end_to_end_wav.rs` | 2-5min | Vosk model | ❌ No |
| **Vosk + injection** | `crates/app/src/stt/tests/end_to_end_wav.rs::test_end_to_end_with_real_injection` | 20-30s | Vosk + display | ❌ No (ignored) |

---

## Decision Logic: When Tests Run

### Local Pre-Commit Hook (`.git-hooks/pre-commit-fast`)
```
✅ Always runs:
  - cargo fmt --check
  - cargo clippy
  - cargo check
  - cargo build
  - cargo nextest run --lib (unit tests only)

❌ Never runs:
  - Integration tests
  - Text injection tests
  - E2E tests
```

### Local Pre-Commit Hook (`.git-hooks/pre-commit-injection-tests`)
```
✅ Always runs:
  - cargo test -p coldvox-text-injection --lib (mock tests)

⚠️ Conditionally runs (non-blocking):
  - Real injection smoke (if DISPLAY set)
  - Vosk E2E (if model exists)
```

### CI - Core Jobs (Required)
```
✅ Always runs:
  - cargo check (MSRV + stable)
  - cargo clippy
  - cargo fmt --check
  - cargo test --workspace (unit + integration)
  - cargo doc
```

### CI - Optional Jobs (Non-Blocking)
```
⚠️ Runs if available:
  - text-injection tests (if xdotool/Xvfb/openbox available)
  - vosk-e2e tests (if model available)

Result: continue-on-error: true
```

---

## How to Control Optional Tests

### Environment Variables

| Variable | Effect |
|----------|--------|
| `RUN_REAL_INJECTION_SMOKE=1` | Enable smoke test in pre-commit |
| `ENFORCE_REAL_SMOKE=1` | Make smoke test blocking |
| `TEST_WAV_MODE=all` | Run all WAV files in E2E test |
| `VOSK_MODEL_PATH=/path` | Override model location |
| `DISPLAY=:99` | Required for text injection tests |
| `WAYLAND_DISPLAY=wayland-0` | Alternative to DISPLAY |

### Feature Flags

| Feature | Enables |
|---------|---------|
| `real-injection-tests` | Real text injection test suite |
| `vosk` | Vosk STT tests |
| `text-injection` | Text injection system |
| `atspi` | AT-SPI backend + tests |
| `wl_clipboard` | Clipboard backend + tests |
| `enigo` | Enigo backend + tests |
| `ydotool` | ydotool backend + tests |

### Test Attributes

| Attribute | Behavior |
|-----------|----------|
| `#[ignore]` | Skip unless `--ignored` passed |
| `#[cfg(feature = "X")]` | Only compile if feature enabled |
| `#[cfg(all(test, feature = "X"))]` | Test-only code with feature |

---

## Recommendations

### For Developers

**Daily workflow**:
```bash
# Fast pre-commit hook runs automatically
git commit -m "fix: something"
# → Takes 5-10 seconds, non-blocking

# Before pushing, run full unit tests
cargo nextest run --workspace
# → Takes 30-60 seconds

# Optional: Run text injection smoke test
RUN_REAL_INJECTION_SMOKE=1 cargo test -p coldvox-text-injection \
  --features real-injection-tests -- real_injection_smoke
# → Takes 10 seconds if you have display
```

**Before major PR**:
```bash
# Full local CI simulation
cargo fmt --check && \
cargo clippy --all-targets -- -D warnings && \
cargo check --workspace --all-targets && \
cargo nextest run --workspace && \
cargo test -p coldvox-app --features vosk test_end_to_end_wav_pipeline -- --nocapture
# → Takes 2-3 minutes
```

### For CI

**Current setup (recommended)**:
- ✅ Core tests (fmt, clippy, check, test) are **required**
- ⚠️ Optional tests (text-injection, vosk-e2e) use `continue-on-error: true`
- ✅ Clear distinction between "must pass" and "nice to have"

**Alternative (stricter)**:
- Make text-injection required after provisioning runner with all tools
- Make vosk-e2e required after ensuring model is always available
- Trade-off: Less flexible, more brittle

**Alternative (looser)**:
- Move all optional tests to nightly scheduled workflow
- Only run core tests on PR
- Trade-off: Slower feedback on regressions

---

## Implementation Details

### How Tests Self-Guard

**Text injection tests**:
```rust
let env = TestEnvironment::current();
if !env.can_run_real_tests() {
    eprintln!("Skipping: no display server found.");
    return;  // ← Test passes without running
}
```

**Vosk tests**:
```rust
let model_path = resolve_vosk_model_path();
if !std::path::Path::new(&model_path).exists() {
    eprintln!("Skipping test: Model not found at {}", model_path);
    return;  // ← Test passes without running
}
```

**Backend-specific tests**:
```rust
let injector = AtspiInjector::new(config);
if !injector.is_available().await {
    eprintln!("Skipping AT-SPI test: Backend not available");
    return;  // ← Test passes without running
}
```

### How CI Self-Guards

**Job-level gating**:
```yaml
- name: Check if tools available
  id: check_tools
  run: |
    has_tools=true
    for tool in xdotool Xvfb openbox; do
      if ! command -v $tool; then
        has_tools=false
      fi
    done
    echo "available=$has_tools" >> $GITHUB_OUTPUT

- name: Run tests
  if: steps.check_tools.outputs.available == 'true'
  run: cargo test -p coldvox-text-injection

- name: Skip message
  if: steps.check_tools.outputs.available != 'true'
  run: echo "⚠️ Skipping - tools not available"
```

**Job-level continue-on-error**:
```yaml
text-injection:
  continue-on-error: true  # ← Don't fail PR if this fails
  steps:
    - run: cargo test -p coldvox-text-injection
```

---

## See Also

- `docs/dev/LOCAL_DEV_WORKFLOW.md` - Complete development workflow guide
- `docs/dev/CI_WORKFLOW_BRITTLENESS_ANALYSIS.md` - Why optional tests exist
- `docs/dev/CI_PHILOSOPHY_SHIFT.md` - Fast local > slow remote philosophy
- `docs/TESTING.md` - Full testing documentation
- `.github/workflows/ci-minimal.yml` - Minimal CI implementation
- `.git-hooks/pre-commit-fast` - Fast local checks
