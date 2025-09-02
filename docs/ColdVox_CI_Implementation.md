# ColdVox CI Architecture

**Last Updated:** 2025-09-02  
**Repository:** ColdVox  
**Status:** Fully Implemented

## Overview

This document describes ColdVox's multi-crate workspace CI/CD implementation using the organization's reusable workflow plus project-specific additions. The workspace contains 10 specialized crates with different system dependencies and feature requirements:

- **Core crates**: app, foundation, telemetry, audio, gui (placeholder)
- **VAD crates**: vad (abstractions), vad-silero (ML implementation)
- **STT crates**: stt (abstractions), stt-vosk (Vosk implementation)
- **Text injection**: text-injection

Platform priority: **Linux first** (must pass), **Windows second** (best effort).

## Current State

### Multi-Crate Workspace Structure

```toml
[workspace]
members = [
    "crates/app",
    "crates/coldvox-foundation",
    "crates/coldvox-telemetry",
    "crates/coldvox-audio",
    "crates/coldvox-vad",
    "crates/coldvox-vad-silero",
    "crates/coldvox-text-injection",
    "crates/coldvox-stt",
    "crates/coldvox-stt-vosk",
    "crates/coldvox-gui",
]
```

### Crate-Specific Dependencies

- **coldvox-audio**: Requires ALSA (libasound2-dev) on Linux
- **coldvox-stt-vosk**: Requires Vosk model download and caching
- **coldvox-text-injection**: Uses ydotool, atspi, and clipboard backends (may require libxdo-dev, libxtst-dev for some backends)
- **coldvox-gui**: Placeholder crate with no current dependencies

### Current Features

- ✅ Workspace-aware testing (per-crate parallel execution)
- ✅ MSRV validation
- ✅ cargo-nextest integration
- ✅ cargo-deny for license/security checks
- ✅ Lockfile enforcement (--locked)
- ✅ Feature combination testing with cargo-hack
- ✅ SHA-pinned actions (security requirement)

## Target Architecture

```
ColdVox CI Structure:
.github/
├── workflows/
│   ├── ci.yml                   # Shim calling org workflow with workspace config
│   ├── vosk-integration.yml     # Specialized Vosk testing (path-filtered)
│   ├── feature-matrix.yml       # cargo-hack feature combination testing
│   ├── cross-platform.yml       # Linux-first, Windows-second testing
│   ├── release.yml              # Release automation (unchanged)
│   └── benchmarks.yml           # Performance tracking
├── dependabot.yml               # Automated dependency updates
└── deny.toml                    # cargo-deny configuration
```

## Core CI (Using Reusable Workflow)

### File: `.github/workflows/ci.yml`

```yaml
name: CI

on:
  pull_request:
  push:
    branches: [main, develop]
  workflow_dispatch:  # Allow manual triggering for agents

concurrency:
  group: ci-${{ github.ref }}
  cancel-in-progress: true

jobs:
  # Core CI using org workflow with workspace support
  common-ci:
    uses: coldaine/.github/.github/workflows/lang-ci.yml@v1
    secrets: inherit
    with:
      run_rust: true
      run_python: false
      rust_no_default_features: true  # Baseline without Vosk
      rust_msrv: "1.70.0"             # Minimum supported version
      use_nextest: true                # Faster test execution
      use_sccache: true                # Build caching
      run_cargo_deny: true             # License/security checks
      test_timeout_minutes: 30
      max_parallel: 4                  # Limit parallel crate builds
      # Crate-specific system dependencies
      crate_system_deps: '{
        "coldvox-audio": "libasound2-dev",
        "coldvox-text-injection": "libxdo-dev libxtst-dev",
        "coldvox-gui": ""
      }'

  # Validate lockfile is committed and up-to-date
  lockfile-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@3df4ab11eba7bda6032a0b82a6bb43b11571feac # v4.1.7
      - uses: moonrepo/setup-rust@b8edcc56728bbc002beca25e7f6723d1aab343f8 # v1.2.1
      - name: Validate lockfile
        run: |
          cargo check --locked --workspace
          if git diff --exit-code Cargo.lock; then
            echo "✅ Lockfile is up to date"
          else
            echo "❌ Cargo.lock is out of date"
            exit 1
          fi

  # Quick validation of key features
  feature-smoke-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@3df4ab11eba7bda6032a0b82a6bb43b11571feac # v4.1.7
      - uses: moonrepo/setup-rust@b8edcc56728bbc002beca25e7f6723d1aab343f8 # v1.2.1
        with:
          bins: cargo-nextest
      - uses: Swatinem/rust-cache@23bce251a8cd2ffc3c1075eaa2367cf899916d84 # v2.7.3
      - name: Install system deps
        run: |
          sudo apt-get update
          sudo apt-get install -y libasound2-dev libxdo-dev libxtst-dev
      - name: Check key features
        run: |
          cargo check --locked -p coldvox-text-injection
          cargo check --locked -p app --features text-injection
          cargo check --locked -p app --features examples
```

Note on auto-detection: Avoid using `hashFiles()` in job-level `if:` (it is evaluated on the server before a runner exists and will error). Use one of:
- Step-level `if:` with `hashFiles()` after checkout; or
- A small precheck job that sets `has_rust`/`has_python` outputs and gate jobs with `needs.precheck.outputs.*`.

## Vosk Integration Testing

### File: `.github/workflows/vosk-integration.yml`

```yaml
name: Vosk Integration Tests

on:
  # Run on PRs that modify STT-related code
  pull_request:
    paths:
      - 'crates/coldvox-stt/**'
      - 'crates/coldvox-stt-vosk/**'
      - 'crates/app/**'
      - 'examples/vosk_*.rs'
      - '.github/workflows/vosk-integration.yml'
  # Weekly scheduled run
  schedule:
    - cron: '0 0 * * 0'
  # Manual trigger
  workflow_dispatch:

jobs:
  vosk-tests:
    name: Vosk STT Integration
    runs-on: ubuntu-latest
    timeout-minutes: 45
    
    steps:
      - name: Checkout code
        uses: actions/checkout@3df4ab11eba7bda6032a0b82a6bb43b11571feac # v4.1.7
        with:
          fetch-depth: 0
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@1482605bfc5719782e1267fd0c0cc350fe7646b8 # v1
        with:
          toolchain: stable
      
      - name: Cache Cargo
        uses: Swatinem/rust-cache@23bce251a8cd2ffc3c1075eaa2367cf899916d84 # v2.7.3
      
      - name: Install system dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y \
            libasound2-dev \
            python3 \
            python3-pip \
            wget \
            unzip
      
      - name: Cache Vosk model
        id: cache-vosk-model
        uses: actions/cache@13aacd865c20de90d75de3b17ebe84f7a17d57d2 # v4.0.0
        with:
          path: models/vosk-model-small-en-us-0.15
          key: vosk-model-small-en-us-0.15
          restore-keys: |
            vosk-model-small-en-us-
            vosk-model-
      
      - name: Download Vosk model
        if: steps.cache-vosk-model.outputs.cache-hit != 'true'
        run: |
          mkdir -p models
          cd models
          # Retry logic for robustness
          for i in 1 2 3; do
            wget https://alphacephei.com/vosk/models/vosk-model-small-en-us-0.15.zip && break
            echo "Download attempt $i failed, retrying..."
            sleep 5
          done
          unzip vosk-model-small-en-us-0.15.zip
          rm vosk-model-small-en-us-0.15.zip
      
      - name: Install cargo-nextest
        uses: taiki-e/install-action@5ff18d7fb42b9cb96e9b08bd87f965bb411b4daf # v2.44.45
        with:
          tool: cargo-nextest
      
      - name: Build with Vosk
        run: |
          # Build both crates that use Vosk feature
          cargo build --locked -p coldvox-stt-vosk --features vosk
          cargo build --locked -p app --features vosk
      
      - name: Run Vosk tests
        env:
          VOSK_MODEL_PATH: models/vosk-model-small-en-us-0.15
          RUST_TEST_THREADS: 1  # Vosk may have threading issues
        run: |
          cargo nextest run --locked -p coldvox-stt-vosk --features vosk
      
      - name: Run end-to-end WAV pipeline test
        env:
          VOSK_MODEL_PATH: models/vosk-model-small-en-us-0.15
        run: |
          cargo test --locked -p app --features vosk test_end_to_end_wav_pipeline -- --ignored --nocapture
      
      - name: Test Vosk examples
        env:
          VOSK_MODEL_PATH: models/vosk-model-small-en-us-0.15
        run: |
          # Run Vosk examples if they exist
          if ls examples/vosk_*.rs 1> /dev/null 2>&1; then
            for example in examples/vosk_*.rs; do
              name=$(basename $example .rs)
              cargo run --locked --example $name --features vosk,examples || true
            done
          fi
      
      - name: Upload artifacts on failure
        if: failure()
        uses: actions/upload-artifact@50769540e7f4bd5e21e526ee35c689e35e0d6874 # v4.4.0
        with:
          name: vosk-test-artifacts
          path: |
            target/debug/**/*.log
            logs/
            transcripts/
          retention-days: 7
```

## Cross-Platform Testing

### File: `.github/workflows/cross-platform.yml`

```yaml
name: Cross-Platform Tests

on:
  # Only on release preparation
  pull_request:
    branches: [main]
    types: [labeled]
  # Manual trigger
  workflow_dispatch:

jobs:
  # Linux is primary platform - MUST pass
  linux-test:
    if: contains(github.event.label.name, 'release') || github.event_name == 'workflow_dispatch'
    runs-on: ubuntu-latest
    timeout-minutes: 30
    steps:
      - uses: actions/checkout@3df4ab11eba7bda6032a0b82a6bb43b11571feac # v4.1.7
      - uses: moonrepo/setup-rust@b8edcc56728bbc002beca25e7f6723d1aab343f8 # v1.2.1
        with:
          bins: cargo-nextest
      - uses: Swatinem/rust-cache@23bce251a8cd2ffc3c1075eaa2367cf899916d84 # v2.7.3
      
      - name: Install Linux dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y libasound2-dev libxdo-dev libxtst-dev
      
      - name: Build
        run: cargo build --workspace --locked --no-default-features
      
      - name: Test
        run: cargo nextest run --workspace --locked --no-default-features
  
  # Windows is secondary platform - best effort
  windows-test:
    if: contains(github.event.label.name, 'release') || github.event_name == 'workflow_dispatch'
    runs-on: windows-latest
    timeout-minutes: 45
    continue-on-error: true  # Don't block releases on Windows issues
    
    steps:
      - uses: actions/checkout@3df4ab11eba7bda6032a0b82a6bb43b11571feac # v4.1.7
      - uses: moonrepo/setup-rust@b8edcc56728bbc002beca25e7f6723d1aab343f8 # v1.2.1
        with:
          bins: cargo-nextest
      - uses: Swatinem/rust-cache@23bce251a8cd2ffc3c1075eaa2367cf899916d84 # v2.7.3
        with:
          key: windows
      
      - name: Build (Windows)
        run: cargo build --workspace --locked --no-default-features
      
      - name: Test (Windows)
        run: cargo nextest run --workspace --locked --no-default-features
      
      - name: Upload Windows artifacts on failure
        if: failure()
        uses: actions/upload-artifact@50769540e7f4bd5e21e526ee35c689e35e0d6874 # v4.4.0
        with:
          name: windows-failure-artifacts
          path: |
            target/debug/**/*.log
          retention-days: 3
```

## Feature Matrix Testing

### File: `.github/workflows/feature-matrix.yml`

```yaml
name: Feature Matrix Tests

on:
  # Weekly comprehensive testing
  schedule:
    - cron: '0 2 * * 1'
  # Manual trigger
  workflow_dispatch:

jobs:
  feature-combinations:
    runs-on: ubuntu-latest
    timeout-minutes: 60
    steps:
      - uses: actions/checkout@3df4ab11eba7bda6032a0b82a6bb43b11571feac # v4.1.7
      - uses: moonrepo/setup-rust@b8edcc56728bbc002beca25e7f6723d1aab343f8 # v1.2.1
        with:
          bins: cargo-hack
      - uses: Swatinem/rust-cache@23bce251a8cd2ffc3c1075eaa2367cf899916d84 # v2.7.3
      
      - name: Install system dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y libasound2-dev libxdo-dev libxtst-dev
      
      - name: Test feature combinations for critical crates
        run: |
          # Test each feature in isolation and combinations
          cargo hack test --each-feature -p coldvox-audio
          cargo hack test --each-feature -p coldvox-vad
          cargo hack test --each-feature -p coldvox-text-injection
          
          # Skip GUI and Vosk crates (require special setup)
          cargo hack test --each-feature --workspace \
            --exclude coldvox-gui \
            --exclude coldvox-stt-vosk
```

## Performance Monitoring

### File: `.github/workflows/benchmarks.yml`

```yaml
name: Benchmarks

on:
  push:
    branches: [main]
  pull_request:
    types: [opened, synchronize]

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@3df4ab11eba7bda6032a0b82a6bb43b11571feac # v4.1.7
      - uses: dtolnay/rust-toolchain@1482605bfc5719782e1267fd0c0cc350fe7646b8 # v1
        with:
          toolchain: stable
      - uses: Swatinem/rust-cache@23bce251a8cd2ffc3c1075eaa2367cf899916d84 # v2.7.3
      
      - name: Install system dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y libasound2-dev
      
      - name: Run benchmarks
        run: cargo bench --locked --no-default-features -- --output-format bencher | tee output.txt
      
      - name: Store benchmark result
        uses: benchmark-action/github-action-benchmark@4de1bed97a47495fc4c5404952da0499e31f5c29 # v1.20.3
        with:
          tool: 'cargo'
          output-file-path: output.txt
          github-token: ${{ secrets.GITHUB_TOKEN }}
          auto-push: false
          comment-on-alert: true
          alert-threshold: '120%'
          fail-on-alert: false
```

## Supporting Files

### File: `.github/dependabot.yml`

```yaml
version: 2
updates:
  - package-ecosystem: "github-actions"
    directory: "/"
    schedule:
      interval: "weekly"
    commit-message:
      prefix: "chore(deps)"
    labels:
      - "dependencies"
      - "github-actions"
    # Group updates to reduce PR noise
    groups:
      actions:
        patterns:
          - "*"
  
  - package-ecosystem: "cargo"
    directory: "/"
    schedule:
      interval: "weekly"
    commit-message:
      prefix: "chore(deps)"
    labels:
      - "dependencies"
      - "rust"
    # Don't update workspace members
    ignore:
      - dependency-name: "coldvox-*"
```

### File: `deny.toml`

```toml
[licenses]
unlawful = ["GPL-3.0", "AGPL-3.0"]
allow = ["MIT", "Apache-2.0", "BSD-3-Clause", "ISC", "Unicode-DFS-2016"]

[bans]
multiple-versions = "warn"
highlighted = ["openssl", "native-tls"]
skip = []  # Add crates to skip if needed

[advisories]
db-path = "~/.cargo/advisory-db"
db-urls = ["https://github.com/rustsec/advisory-db"]
yank = "warn"

[sources]
unknown-registry = "warn"
unknown-git = "warn"
```

### File: `release-plz.toml`

```toml
[workspace]
allow-dirty = false
changelog-include = ["crates/*"]
pr-labels = ["release"]

[git]
tag-prefix = "v"
release-commit-message = "chore: release {{package_name}} v{{version}}"

# Configure all workspace members
[[package]]
name = "app"
publish = false
changelog-path = "crates/app/CHANGELOG.md"

[[package]]
name = "coldvox-foundation"
publish = false

[[package]]
name = "coldvox-audio"
publish = false

[[package]]
name = "coldvox-vad"
publish = false

[[package]]
name = "coldvox-vad-silero"
publish = false

[[package]]
name = "coldvox-stt"
publish = false

[[package]]
name = "coldvox-stt-vosk"
publish = false

[[package]]
name = "coldvox-text-injection"
publish = false

[[package]]
name = "coldvox-telemetry"
publish = false

[[package]]
name = "coldvox-gui"
publish = false
```


## CI Requirements Summary

### Baseline (Required - Linux)
- ✅ Format checking (cargo fmt)
- ✅ Linting (cargo clippy)
- ✅ Build validation (--locked --no-default-features)
- ✅ Test execution (cargo nextest)
- ✅ Doc tests (cargo test --doc)
- ✅ Security audit (rustsec + cargo-deny)
- ✅ MSRV validation
- ✅ Lockfile enforcement
- ✅ Per-crate parallel testing

### Extended (Scheduled/On-Demand)
- ✅ Vosk integration tests (path-filtered)
- ✅ Feature matrix testing (cargo-hack)
- ✅ Cross-platform validation (Windows best-effort)
- ✅ Benchmark tracking (main branch)
- ⏳ Coverage reporting (future)
- ⏳ Minimal-versions testing (nightly)

### Excluded from CI
- ❌ Live hardware tests (requires physical microphone)
- ❌ TUI dashboard tests (requires terminal)
- ❌ Text injection runtime tests (requires display server)
- ❌ GUI runtime tests (requires display)
- ❌ Models in git (use cache + download instead)

## Local Development

### Running CI Locally

```bash
# Install tools
cargo install cargo-nextest cargo-hack cargo-deny
cargo install cargo-binstall  # For faster tool installation

# Run baseline checks
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo nextest run --workspace --locked
cargo deny check all

# Test feature combinations
cargo hack test --each-feature -p coldvox-audio

# Use act for GitHub Actions testing
act -W .github/workflows/ci.yml --container-architecture linux/amd64
```

### Pre-commit Validation

```bash
# Create pre-commit hook
cat > .git/hooks/pre-commit << 'EOF'
#!/bin/bash
set -e

echo "Running pre-commit checks..."

# Verify lockfile
if ! cargo check --locked --workspace 2>/dev/null; then
    echo "Error: Cargo.lock is out of date"
    exit 1
fi

# Format check
cargo fmt --all -- --check

# Clippy
cargo clippy --workspace --all-targets --locked --no-default-features -- -D warnings

# Quick tests with nextest if available
if command -v cargo-nextest &> /dev/null; then
    cargo nextest run --workspace --locked --no-default-features --lib
else
    cargo test --workspace --locked --no-default-features --lib
fi

echo "Pre-commit checks passed!"
EOF

chmod +x .git/hooks/pre-commit
```

## Troubleshooting

### Common Issues

**Issue**: Lockfile out of date  
**Solution**: Run `cargo update` locally and commit `Cargo.lock`

**Issue**: Vosk tests fail in CI  
**Solution**: Check model cache with restore-keys, set RUST_TEST_THREADS=1

**Issue**: Per-crate deps not installing  
**Solution**: Verify crate_system_deps JSON format and crate names match

**Issue**: Windows builds fail  
**Solution**: Expected (continue-on-error: true), fix incrementally

**Issue**: cargo-deny license conflicts  
**Solution**: Update deny.toml with allowed licenses or skip specific crates

**Issue**: Feature matrix takes too long  
**Solution**: Exclude heavy crates or run only on schedule/manual trigger

## Success Metrics

### Current Status
- ✅ CI runs efficiently with nextest + sccache
- ✅ All actions SHA-pinned for security
- ✅ Per-crate parallel testing active
- ✅ Lockfile enforcement enabled
- ✅ MSRV validated on every PR
- ✅ cargo-deny catching license issues
- ✅ Vosk tests path-filtered correctly
- ✅ Linux always passes, Windows best-effort
- ✅ Feature matrix covers critical paths
- ✅ Automated dependency updates via Dependabot
- ✅ Build cache optimization active

## Appendix: Multi-Crate Architecture Benefits

### Current Workspace Structure
- 10 specialized crates
- Clear separation of concerns
- Independent versioning possible
- Parallel compilation benefits

### CI Improvements
1. **Per-crate testing**: Faster feedback with parallel execution
2. **Targeted dependencies**: Only install what each crate needs
3. **Feature isolation**: Test combinations without full workspace impact
4. **Path filtering**: Run only relevant tests on changes
5. **Cache efficiency**: Per-crate cache keys reduce invalidation

### Platform Strategy
1. **Linux-first**: Primary platform, all tests must pass
2. **Windows-second**: Best effort, continue-on-error
3. **macOS**: Optional, for release validation only
4. **WASM**: Future consideration for coldvox-gui