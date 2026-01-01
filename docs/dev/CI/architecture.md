# CI Architecture

> **Principle**: The laptop only does what only the laptop can do.

## Overview

ColdVox CI splits workloads between GitHub-hosted and self-hosted runners based on one question:

**Does this task require the physical laptop's hardware (display, audio, clipboard)?**

| Requires Laptop? | Task | Runner |
|------------------|------|--------|
| No | `cargo fmt --check` | GitHub-hosted |
| No | `cargo clippy` | GitHub-hosted |
| No | `cargo audit`, `cargo deny` | GitHub-hosted |
| No | `cargo build` | GitHub-hosted |
| No | `cargo test --workspace` (unit tests) | GitHub-hosted |
| **Yes** | Hardware tests (display, audio, clipboard) | Self-hosted |

---

## Why Split?

### 1. Hardware Isolation

The self-hosted runner is a laptop with **weak hardware but a live display**. GitHub-hosted runners have **powerful hardware but no display**.

- **Laptop**: Only runs tests that need real display/audio/clipboard
- **GitHub**: Handles everything else (lint, security, build, unit tests)

### 2. Parallelism

GitHub-hosted jobs run in parallel on separate machines. Self-hosted queues on one laptop.

```
Push PR:
  GitHub:      [lint] [security] [docs] [build+unit-tests]  ← All parallel
  Self-hosted: [hardware tests]                              ← Only hardware-dependent tests

Total time: max(GitHub jobs, hardware tests)
```

### 3. No Wasted Work

The laptop does minimal work - just the tests that *require* hardware access.

---

## Self-Hosted Runner Environment

### Critical Facts

| Fact | Implication |
|------|-------------|
| **Live KDE Plasma 6.5.3 session** | No Xvfb needed. Use real `$DISPLAY`. |
| **Fedora/Nobara Linux** | `apt-get` does not exist. Use `dnf`. |
| **Always available** | Auto-login configured, survives reboots. |
| **Warm sccache** | Incremental builds are fast (~2-3 min). |
| **Real hardware** | Display, audio capture, clipboard all work. |

### What Breaks CI

| Mistake | Why It Breaks |
|---------|---------------|
| `GabrielBB/xvfb-action` | Internally calls `apt-get` (doesn't exist) |
| `sudo apt-get install` | Wrong package manager |
| `DISPLAY=:99` | Conflicts with real display (`:0`) |
| Running builds on self-hosted | Weak hardware; GitHub-hosted is faster |
| Running unit tests on self-hosted | Wastes limited resources |

---

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                      GITHUB-HOSTED (ubuntu-latest)              │
│              Parallel, powerful, handles most work              │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐             │
│  │    lint     │  │  security   │  │    docs     │             │
│  │             │  │             │  │             │             │
│  │ fmt --check │  │ cargo audit │  │  cargo doc  │             │
│  │ clippy      │  │ cargo deny  │  │             │             │
│  │  ~2 min     │  │  ~2 min     │  │  ~2 min     │             │
│  └─────────────┘  └─────────────┘  └─────────────┘             │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │              build_and_unit_tests                        │   │
│  │  cargo check → cargo build → cargo test --workspace      │   │
│  │  ~10-15 min                                              │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
              ║                            
              ║  (parallel, no waiting)    
              ║                            
┌─────────────────────────────────────────────────────────────────┐
│                 SELF-HOSTED (Fedora/Nobara)                     │
│        Weak hardware BUT has live KDE Plasma display            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                   hardware_tests                         │   │
│  │                                                          │   │
│  │  Environment:                                            │   │
│  │  • DISPLAY=:0 (live session, NOT :99)                   │   │
│  │  • WAYLAND_DISPLAY=wayland-0                            │   │
│  │  • Real audio, real clipboard                           │   │
│  │                                                          │   │
│  │  Tests:                                                  │   │
│  │  • real-injection-tests (xdotool, ydotool, clipboard)   │   │
│  │  • hardware_check (audio capture, display access)       │   │
│  │                                                          │   │
│  │  Total: ~5-10 min (minimal work!)                       │   │
│  │                                                          │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
                     ┌─────────────────┐
                     │   ci-success    │
                     │   (aggregate)   │
                     └─────────────────┘
```

---

## Speed Optimizations (Self-Hosted)

### 1. sccache (Compiler Cache)
```bash
SCCACHE_CACHE_SIZE="20G"
sccache --start-server
export RUSTC_WRAPPER=$(which sccache)
```

### 2. mold Linker (3-5x Faster Linking)
```bash
# Install once:
sudo dnf install mold

# In CI:
RUSTFLAGS="-C link-arg=-fuse-ld=mold"
```

### 3. Incremental Compilation
```bash
CARGO_INCREMENTAL="1"  # Default, but explicit
```

### 4. Targeted Build
```bash
# Instead of full workspace:
cargo build -p coldvox-text-injection -p coldvox-app
```

### 5. No Dependency Waiting
```yaml
hardware:
  runs-on: [self-hosted, Linux, X64, Fedora, Nobara]
  # NO 'needs:' clause - starts immediately
```

---

## Proposed ci.yml

```yaml
name: CI

on:
  push:
    branches: [main, release/*, feature/*, feat/*, fix/*]
  pull_request:
    branches: [main]

concurrency:
  group: ci-${{ github.ref }}
  cancel-in-progress: true

jobs:
  # ═══════════════════════════════════════════════════════════════
  # GITHUB-HOSTED: Fast parallel checks, NO BUILD
  # ═══════════════════════════════════════════════════════════════

  lint:
    runs-on: ubuntu-latest
    timeout-minutes: 5
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --all -- --check
      - run: cargo clippy --all-targets --locked -- -D warnings

  security:
    runs-on: ubuntu-latest
    timeout-minutes: 5
    continue-on-error: true  # Advisory, don't block
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo install cargo-audit cargo-deny --locked || true
      - run: cargo audit || true
      - run: cargo deny check

  # ═══════════════════════════════════════════════════════════════
  # SELF-HOSTED: THE build, THE tests (runs immediately, no waiting)
  # ═══════════════════════════════════════════════════════════════

  hardware:
    runs-on: [self-hosted, Linux, X64, Fedora, Nobara]
    # NO 'needs:' - starts in parallel with GitHub-hosted jobs
    timeout-minutes: 15
    env:
      CARGO_INCREMENTAL: "1"
      SCCACHE_CACHE_SIZE: "20G"
      RUSTFLAGS: "-C link-arg=-fuse-ld=mold"
      RUST_LOG: info
    steps:
      - uses: actions/checkout@v4

      - name: Start sccache
        run: |
          sccache --start-server 2>/dev/null || true
          echo "RUSTC_WRAPPER=$(which sccache)" >> "$GITHUB_ENV"

      - uses: Swatinem/rust-cache@v2
        with:
          shared-key: "nobara-hardware"
          cache-on-failure: true

      - name: Build (cached, incremental)
        run: cargo build -p coldvox-text-injection -p coldvox-app --locked

      - name: Validate display environment
        run: |
          echo "DISPLAY=$DISPLAY"
          echo "WAYLAND_DISPLAY=$WAYLAND_DISPLAY"
          xset -q >/dev/null 2>&1 || { echo "::error::No X display"; exit 1; }
          echo "✓ Display accessible"

      - name: Hardware integration tests
        run: |
          cargo test -p coldvox-text-injection \
            --features real-injection-tests \
            --locked \
            -- --nocapture --test-threads=1

      - name: Hardware capability checks
        env:
          COLDVOX_E2E_REAL_INJECTION: "1"
          COLDVOX_E2E_REAL_AUDIO: "1"
        run: |
          cargo test -p coldvox-app --test hardware_check \
            --locked -- --nocapture --include-ignored

      - name: sccache stats
        if: always()
        run: sccache --show-stats || true

  # ═══════════════════════════════════════════════════════════════
  # AGGREGATOR
  # ═══════════════════════════════════════════════════════════════

  ci-success:
    runs-on: ubuntu-latest
    needs: [lint, hardware]
    if: always()
    steps:
      - name: Check results
        run: |
          echo "## CI Results" >> $GITHUB_STEP_SUMMARY
          echo "| Job | Result |" >> $GITHUB_STEP_SUMMARY
          echo "|-----|--------|" >> $GITHUB_STEP_SUMMARY
          echo "| lint | ${{ needs.lint.result }} |" >> $GITHUB_STEP_SUMMARY
          echo "| hardware | ${{ needs.hardware.result }} |" >> $GITHUB_STEP_SUMMARY

          if [[ "${{ needs.lint.result }}" != "success" ]]; then
            echo "::error::Lint failed"
            exit 1
          fi
          if [[ "${{ needs.hardware.result }}" != "success" ]]; then
            echo "::error::Hardware tests failed"
            exit 1
          fi
          echo "✅ CI passed"
```

---

## Common Mistakes to Avoid

### DON'T: Use Xvfb on self-hosted
```yaml
# WRONG - runner has live display
- uses: GabrielBB/xvfb-action@v1  # Also uses apt-get internally
```

### DON'T: Use apt-get
```yaml
# WRONG - this is Fedora, not Ubuntu
- run: sudo apt-get install -y xdotool
```

### DON'T: Hardcode DISPLAY=:99
```yaml
# WRONG - real display is :0
env:
  DISPLAY: ":99"
```

### DON'T: Make self-hosted wait
```yaml
# WRONG - adds 5-10 min delay
hardware:
  needs: [lint, build]
```

### DON'T: Build on GitHub-hosted
```yaml
# WRONG - wasted work, can't share with Fedora
- run: cargo build --workspace  # On ubuntu-latest
```

---

## History

| Date | Change | Reason |
|------|--------|--------|
| 2025-12-24 | Remove Xvfb, add mold, remove waiting | PR #310 broke CI with apt-get on Fedora |
| 2025-09-19 | Initial self-hosted runner setup | Enable hardware testing |

---

## References

- [Self-hosted runner setup](../../tasks/ci-runner-readiness-proposal.md) (outdated - references Xvfb)
- PR #310: Introduced broken Xvfb infrastructure
- PR #276: Jules draft that caused the issue
