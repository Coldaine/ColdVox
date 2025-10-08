# Local Development Workflow

## Philosophy

**Fast local checks >> Slow CI checks**

Pre-commit hooks run in **< 5 seconds** on your dev machine and catch 95% of issues.
CI runs slower platform-specific tests that you can't run locally.

---

## Setup (One-Time)

### 1. Install the Fast Pre-Commit Hook

```bash
# Link the fast hook
ln -sf ../../.git-hooks/pre-commit-fast .git/hooks/pre-commit

# Or use the setup script
./scripts/setup_hooks.sh
```

### 2. Install cargo-nextest (Optional but Recommended)

```bash
cargo install cargo-nextest --locked
```

Nextest runs tests **3x faster** than `cargo test`:
- ✅ Parallel execution by default
- ✅ Clean, readable output
- ✅ Automatic retry of flaky tests
- ✅ JUnit XML output for CI

---

## What Runs When

### Pre-Commit Hook (< 5 seconds)

**Every commit automatically runs:**
```bash
cargo fmt --check        # Formatting
cargo clippy             # Linting
cargo check              # Type checking
cargo build              # Compilation
cargo nextest run --lib  # Unit tests only
```

**Result:** Non-blocking warnings if something fails

### CI Pipeline (GitHub Actions)

**Only runs on push/PR:**

1. **check** job
   - Validates on stable + MSRV (1.75)
   - Just `cargo check` + `cargo build`

2. **lint** job
   - Formatting + clippy + doc generation
   - Stable only

3. **test** job
   - Unit + integration tests
   - Stable only

4. **text-injection** job (optional)
   - Skipped if X11 tools missing
   - Non-blocking (continue-on-error)

5. **vosk-e2e** job (optional)
   - Skipped if model missing
   - Non-blocking (continue-on-error)

**Result:** Core tests must pass, optional tests can fail

---

## Usage Patterns

### Normal Development

```bash
# Edit code
vim src/main.rs

# Commit (hook runs automatically)
git commit -m "fix: improved error handling"
# → Hook runs in 3-5 seconds
# → Shows warnings but doesn't block
# → Push to GitHub for full CI
```

### Skip Hook if Needed

```bash
git commit --no-verify -m "WIP: debugging"
```

### Run Full Tests Locally

```bash
# All tests
cargo nextest run

# With features
cargo nextest run --features vosk

# Specific package
cargo nextest run -p coldvox-audio

# Watch mode (auto-rerun on changes)
cargo watch -x "nextest run"
```

### Pre-Push Full Check

```bash
# Simulate what CI will do
cargo fmt --check && \
cargo clippy --all-targets -- -D warnings && \
cargo check --workspace --all-targets --locked && \
cargo build --workspace --locked && \
cargo nextest run --workspace --locked
```

---

## Comparison: Old vs New

### Old Approach (Brittle CI)
```
┌─────────────────────────────────────────┐
│ CI Setup Action                         │
│ ├─ Check 18 system commands            │
│ ├─ Check 4 pkg-config libraries        │
│ └─ EXIT 1 if openbox missing           │ ❌ BLOCKS EVERYTHING
└─────────────────────────────────────────┘
         ↓ (never reached)
┌─────────────────────────────────────────┐
│ cargo check, build, test, ...           │ (never ran)
└─────────────────────────────────────────┘
```

**Problems:**
- ❌ One missing tool blocks all feedback
- ❌ Never know if code compiles
- ❌ Slow turnaround (wait for CI)
- ❌ Fragile when system changes

### New Approach (Fast Local)
```
┌─────────────────────────────────────────┐
│ Pre-Commit Hook (local, 3-5s)          │
│ ├─ fmt, clippy, check, build, test     │ ✅ INSTANT FEEDBACK
│ └─ Non-blocking warnings                │
└─────────────────────────────────────────┘
         ↓
┌─────────────────────────────────────────┐
│ CI: Core Jobs (required)                │
│ ├─ check: type + build (MSRV + stable) │ ✅ MUST PASS
│ ├─ lint: fmt + clippy + docs           │ ✅ MUST PASS
│ └─ test: unit + integration            │ ✅ MUST PASS
└─────────────────────────────────────────┘
         ↓
┌─────────────────────────────────────────┐
│ CI: Optional Jobs (continue-on-error)   │
│ ├─ text-injection (if tools available) │ ⚠️ CAN FAIL
│ └─ vosk-e2e (if model available)       │ ⚠️ CAN FAIL
└─────────────────────────────────────────┘
```

**Benefits:**
- ✅ Instant feedback on every commit
- ✅ Core tests always run
- ✅ Optional tests don't block
- ✅ Works on any developer machine

---

## Configuration

### Skip Pre-Commit Checks

```bash
# Temporary
git commit --no-verify

# Permanent
export COLDVOX_SKIP_HOOKS=1
```

### Skip Specific CI Jobs

CI automatically skips optional jobs if tools/models unavailable.

### Environment Variables

```bash
# Pre-commit hook behavior
COLDVOX_SKIP_HOOKS=1           # Don't run any hooks
RUST_LOG=debug                 # Verbose output

# CI behavior  
VOSK_MODEL_PATH=/path/to/model # Custom model location
```

---

## Recommended Workflow

1. **Local development:** Rely on pre-commit hook (fast, automatic)
2. **Before PR:** Run `cargo nextest run --workspace` (full suite)
3. **CI failures:** Check if optional jobs (text-injection, vosk-e2e)
4. **Core failures:** Fix immediately (fmt, clippy, build, test)

---

## Rationale

### Why Non-Blocking Pre-Commit?

**Old way:**
```bash
git commit -m "fix typo"
# → Hook runs
# → Hook fails
# → Commit blocked
# → You add --no-verify
# → Hook becomes useless
```

**New way:**
```bash
git commit -m "fix typo"  
# → Hook runs
# → Hook warns: "⚠️ 2 checks failed"
# → Commit proceeds
# → You fix before pushing
# → Hook remains useful
```

**Philosophy:** Hooks should **inform**, not **block**.

### Why Separate Optional CI Jobs?

**Old way:**
- Text injection test fails → entire CI red
- Never know if core code works

**New way:**
- Core tests pass ✅
- Text injection fails ⚠️
- You know: code compiles, tests pass, just missing xdotool

**Philosophy:** CI should tell you **what works**, not just **what doesn't**.

---

## Troubleshooting

### "Hook runs slowly"

Check if you have cache:
```bash
cargo clean
cargo build  # Populate cache
# Now hook should be fast
```

### "nextest not found"

Install it:
```bash
cargo install cargo-nextest --locked
```

Or hook will fallback to `cargo test` automatically.

### "CI fails on text injection"

Expected if X11 tools not on runner. Check:
- Job marked `continue-on-error: true`? → Can ignore
- Job required for merge? → Need to provision runner

### "Want to run text injection tests locally"

```bash
# Start X server
export DISPLAY=:99
Xvfb :99 -screen 0 1024x768x24 &
openbox &

# Run tests
cargo test -p coldvox-text-injection
```

---

## Migration from Old CI

If you want to replace the old CI completely:

```bash
# Disable old CI
mv .github/workflows/ci.yml .github/workflows/ci.yml.disabled

# Enable new CI
mv .github/workflows/ci-minimal.yml .github/workflows/ci.yml

# Update pre-commit hook
ln -sf ../../.git-hooks/pre-commit-fast .git/hooks/pre-commit
```

Or keep both and compare results for a few weeks.
