# CI Architecture

> **Principle**: The laptop only does what only the laptop can do.

## Overview

ColdVox CI splits workloads between GitHub-hosted and self-hosted runners based on one question:

**Does this task require the physical laptop's hardware?**

| Requires Laptop? | Task | Runner |
|------------------|------|--------|
| No | `cargo fmt --check` | GitHub-hosted |
| No | `cargo clippy` | GitHub-hosted |
| No | `cargo audit`, `cargo deny` | GitHub-hosted |
| **Yes** | `cargo build` (warm cache) | Self-hosted |
| **Yes** | Hardware tests (display, audio, clipboard) | Self-hosted |

---

## Why Split?

### 1. CPU Dedication

If the laptop runs lint, build, AND tests sequentially, they compete for CPU.

With the split:
- **Laptop**: 100% CPU on building + hardware tests
- **GitHub**: Handles lint/security on their infrastructure (free)

### 2. No Redundant Builds

| Bad Pattern | Good Pattern |
|-------------|--------------|
| GitHub: `cargo build` (discarded) | GitHub: `cargo clippy` (type checks only) |
| Self-hosted: `cargo build` (again) | Self-hosted: `cargo build` (THE build) |

`clippy` does full type checking without generating binaries. Same error detection, no wasted compilation.

### 3. Parallelism

GitHub-hosted jobs run in parallel on separate machines. Self-hosted queues on one laptop.

```
Push PR:
  GitHub:      [lint] [security] [docs]     ← All parallel, 2 min each
  Self-hosted: [build + hardware tests]     ← Starts immediately, 8-12 min

Total time: ~12 min (not 2 + 2 + 2 + 12 = 18 min)
```

### 4. No Waiting

Self-hosted has **no `needs:` dependency**. It starts immediately in parallel with GitHub-hosted jobs.

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
| `needs: [lint, build]` | Delays self-hosted start by 5-10 min |
| `cargo build` on GitHub-hosted | Wasted work (can't share artifacts with Fedora) |

---

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                      GITHUB-HOSTED (ubuntu-latest)              │
│                  Parallel, free, NO BUILD artifacts             │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐             │
│  │    lint     │  │  security   │  │    docs     │             │
│  │             │  │             │  │  (optional) │             │
│  │ fmt --check │  │ cargo audit │  │  cargo doc  │             │
│  │ clippy      │  │ cargo deny  │  │             │             │
│  │             │  │             │  │             │             │
│  │  ~2 min     │  │  ~2 min     │  │  ~2 min     │             │
│  │  NO BUILD   │  │  NO BUILD   │  │             │             │
│  └─────────────┘  └─────────────┘  └─────────────┘             │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
              ║                            ║
              ║  (parallel, no waiting)    ║
              ║                            ║
┌─────────────────────────────────────────────────────────────────┐
│                 SELF-HOSTED (Fedora/Nobara)                     │
│              Live KDE Plasma - THE build, THE tests             │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                      hardware                            │   │
│  │                                                          │   │
│  │  Environment:                                            │   │
│  │  • DISPLAY=$DISPLAY (from session, NOT :99)             │   │
│  │  • WAYLAND_DISPLAY=$WAYLAND_DISPLAY                     │   │
│  │  • Real audio, real clipboard                           │   │
│  │                                                          │   │
│  │  Steps:                                                  │   │
│  │  1. cargo build (incremental, sccache, mold) → 2-3 min  │   │
│  │  2. Hardware tests (injection, audio)        → 5-8 min  │   │
│  │                                                          │   │
│  │  Total: ~8-12 min                                        │   │
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
