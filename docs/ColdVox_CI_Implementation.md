# ColdVox CI Implementation

**Last Updated:** 2025-09-01  
**Repository:** ColdVox  
**Status:** Migration Planned

## Overview

This document describes how ColdVox implements CI/CD using the organization's reusable workflow plus project-specific additions. ColdVox requires specialized handling for audio dependencies (ALSA) and feature-gated functionality (Vosk STT) that requires system libraries and models.

## Current State

### Existing CI Components

1. **ci.yml**: Ubuntu-only CI with fmt, clippy, build, test
2. **Composite action**: Local reusability via `.github/actions/rust-check/`
3. **release.yml**: Automated releases with release-plz
4. **No Vosk in baseline**: Correctly excludes system-dependent features

### What Works Well

- Minimal CI that passes reliably
- ALSA dependencies properly installed
- Vosk excluded from baseline (no features specified)
- PAT fallback pattern in release workflow
- Concurrency control

### Technical Debt

- Actions not fully SHA-pinned in ci.yml
- No cargo audit integration
- No cross-platform testing
- No Vosk integration testing
- Local composite action (should migrate to org workflow)

## Target Architecture

```
ColdVox CI Structure:
.github/
├── workflows/
│   ├── ci.yml              # 10-line shim calling org workflow
│   ├── vosk-integration.yml    # Specialized Vosk testing
│   ├── release.yml         # Release automation (unchanged)
│   └── benchmarks.yml      # Performance tracking (future)
└── dependabot.yml          # Dependency updates
```

## Phase 1: Core CI (Using Reusable Workflow)

### File: `.github/workflows/ci.yml`

```yaml
name: CI

on:
  pull_request:
  push:
    branches: [main]

concurrency:
  group: ci-${{ github.ref }}
  cancel-in-progress: true

jobs:
  # Core CI using org workflow
  common-ci:
    uses: coldaine/.github/.github/workflows/lang-ci.yml@v1
    secrets: inherit
    with:
      run_rust: true
      run_python: false
      rust_no_default_features: true  # Avoid Vosk dependency
      install_alsa: true               # Audio library requirement
      test_timeout_minutes: 30

  # Quick smoke test with default features
  feature-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@3df4ab11eba7bda6032a0b82a6bb43b11571feac # v4.1.7
      - uses: dtolnay/rust-toolchain@1482605bfc5719782e1267fd0c0cc350fe7646b8 # v1
        with:
          toolchain: stable
      - uses: Swatinem/rust-cache@23bce251a8cd2ffc3c1075eaa2367cf899916d84 # v2.7.3
      - name: Install ALSA
        run: |
          sudo apt-get update
          sudo apt-get install -y libasound2-dev
      - name: Check text-injection feature
        run: cargo check --features text-injection
      - name: Check examples feature  
        run: cargo check --features examples
```

## Phase 2: Vosk Integration Testing

### File: `.github/workflows/vosk-integration.yml`

```yaml
name: Vosk Integration Tests

on:
  # Run on PRs that modify STT code
  pull_request:
    paths:
      - 'crates/app/src/stt/**'
      - 'crates/app/Cargo.toml'
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
      
      - name: Download Vosk model
        if: steps.cache-vosk-model.outputs.cache-hit != 'true'
        run: |
          mkdir -p models
          cd models
          wget https://alphacephei.com/vosk/models/vosk-model-small-en-us-0.15.zip
          unzip vosk-model-small-en-us-0.15.zip
          rm vosk-model-small-en-us-0.15.zip
      
      - name: Build with Vosk
        run: cargo build --features vosk
      
      - name: Run Vosk tests
        env:
          VOSK_MODEL_PATH: models/vosk-model-small-en-us-0.15
        run: |
          cargo test --features vosk -- --nocapture
      
      - name: Run end-to-end WAV pipeline test
        env:
          VOSK_MODEL_PATH: models/vosk-model-small-en-us-0.15
        run: |
          cargo test --features vosk test_end_to_end_wav_pipeline -- --ignored --nocapture
      
      - name: Test Vosk example
        env:
          VOSK_MODEL_PATH: models/vosk-model-small-en-us-0.15
        run: |
          cargo run --example vosk_test --features vosk,examples
```

## Phase 3: Cross-Platform Testing

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
  matrix-test:
    if: contains(github.event.label.name, 'release') || github.event_name == 'workflow_dispatch'
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
        rust: [stable]
        include:
          - os: ubuntu-latest
            deps_command: |
              sudo apt-get update
              sudo apt-get install -y libasound2-dev
          - os: macos-latest
            deps_command: |
              brew install portaudio
          - os: windows-latest
            deps_command: echo "No additional deps needed"
    
    runs-on: ${{ matrix.os }}
    timeout-minutes: 45
    continue-on-error: true  # Don't block on OS-specific issues
    
    steps:
      - uses: actions/checkout@3df4ab11eba7bda6032a0b82a6bb43b11571feac # v4.1.7
      - uses: dtolnay/rust-toolchain@1482605bfc5719782e1267fd0c0cc350fe7646b8 # v1
        with:
          toolchain: ${{ matrix.rust }}
      - uses: Swatinem/rust-cache@23bce251a8cd2ffc3c1075eaa2367cf899916d84 # v2.7.3
      
      - name: Install platform dependencies
        run: ${{ matrix.deps_command }}
      
      - name: Build
        run: cargo build --workspace --no-default-features
      
      - name: Test
        run: cargo test --workspace --no-default-features
```

## Phase 4: Performance Monitoring

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
      
      - name: Install ALSA
        run: |
          sudo apt-get update
          sudo apt-get install -y libasound2-dev
      
      - name: Run benchmarks
        run: cargo bench --no-default-features -- --output-format bencher | tee output.txt
      
      - name: Store benchmark result
        uses: benchmark-action/github-action-benchmark@v1
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
  
  - package-ecosystem: "cargo"
    directory: "/"
    schedule:
      interval: "weekly"
    commit-message:
      prefix: "chore(deps)"
    labels:
      - "dependencies"
      - "rust"
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

# Prevent crates.io publication
[[package]]
name = "coldvox-app"
publish = false
changelog-path = "crates/app/CHANGELOG.md"
```

## Migration Timeline

### Week 1: Preparation
- [x] Document current state
- [ ] Create org .github repository
- [ ] Implement reusable workflow
- [ ] Tag v1.0.0 and v1

### Week 2: Local Hardening
- [ ] Pin all actions to SHAs in current ci.yml
- [ ] Add dependabot.yml
- [ ] Add release-plz.toml
- [ ] Test locally with act

### Week 3: Migration
- [ ] Replace ci.yml with shim
- [ ] Add vosk-integration.yml
- [ ] Verify all checks pass
- [ ] Update branch protection

### Week 4: Enhancement
- [ ] Add cross-platform workflow
- [ ] Add benchmark workflow
- [ ] Document in README
- [ ] Remove legacy composite action

## CI Requirements Summary

### Baseline (Required)
- ✅ Format checking (cargo fmt)
- ✅ Linting (cargo clippy)
- ✅ Build validation (no default features)
- ✅ Test execution (no default features)
- ✅ Security audit (cargo audit)
- ✅ ALSA installation

### Extended (Optional)
- ⏳ Vosk integration tests (separate workflow)
- ⏳ Cross-platform matrix (on-demand)
- ⏳ Benchmark tracking (main branch only)
- ⏳ Coverage reporting (future)

### Excluded from CI
- ❌ Live hardware tests (requires physical microphone)
- ❌ TUI dashboard tests (requires terminal)
- ❌ Text injection tests (requires display server)

## Local Development

### Running CI Locally

```bash
# Install act for local GitHub Actions testing
brew install act  # or your package manager

# Run CI workflow locally
act -W .github/workflows/ci.yml

# Run with specific event
act pull_request -W .github/workflows/ci.yml
```

### Pre-commit Validation

```bash
# Create pre-commit hook
cat > .git/hooks/pre-commit << 'EOF'
#!/bin/bash
set -e

echo "Running pre-commit checks..."

# Format check
cargo fmt --all -- --check

# Clippy
cargo clippy --workspace --all-targets --no-default-features -- -D warnings

# Test
cargo test --workspace --no-default-features --lib --bins

echo "Pre-commit checks passed!"
EOF

chmod +x .git/hooks/pre-commit
```

## Troubleshooting

### Common Issues

**Issue**: Vosk tests fail in CI  
**Solution**: Ensure VOSK_MODEL_PATH is set and model is cached

**Issue**: ALSA not found on Linux  
**Solution**: Add `install_alsa: true` to workflow inputs

**Issue**: Windows builds fail  
**Solution**: Use `continue-on-error: true` initially, fix incrementally

**Issue**: Cargo audit fails  
**Solution**: Run `cargo update` or add exemptions for false positives

## Success Metrics

### Phase 1 (Baseline)
- [ ] CI runs in < 5 minutes
- [ ] Zero false positives
- [ ] All PRs have CI checks

### Phase 2 (Extended)
- [ ] Vosk tests run weekly
- [ ] Cross-platform validation before releases
- [ ] Benchmark regressions detected

### Phase 3 (Mature)
- [ ] < 2% flaky test rate
- [ ] 80%+ code coverage
- [ ] Automated dependency updates

## Appendix: Current vs Future

### Current ci.yml Structure
- 47 lines
- Local composite action
- Ubuntu-only
- No security scanning

### Future ci.yml Structure
- 10-15 lines
- Org reusable workflow
- Extensible via inputs
- Security by default

### Benefits of Migration
1. **Consistency**: Same CI across all projects
2. **Maintenance**: Updates in one place
3. **Security**: Centralized action pinning
4. **Flexibility**: Per-project customization
5. **Scalability**: Easy to add new checks