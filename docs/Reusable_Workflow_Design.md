# Org-Level Reusable Workflow Design

**Last Updated:** 2025-09-01  
**Purpose:** Define a centralized, reusable CI workflow for organization-wide use  
**Repository:** `<org>/.github`

## Overview

This document specifies the design of an organization-level reusable workflow that provides baseline CI capabilities for Rust and Python projects. Individual repositories call this workflow with a minimal shim, inheriting standardized CI practices while maintaining flexibility for project-specific needs.

## Architecture

```
<org>/.github/                    # Organization .github repository
├── .github/
│   └── workflows/
│       └── lang-ci.yml           # Reusable workflow (workflow_call)
├── README.md
└── CHANGELOG.md
```

## Reusable Workflow Specification

### File: `<org>/.github/.github/workflows/lang-ci.yml`

```yaml
name: Common CI

on:
  workflow_call:
    inputs:
      # Language toggles
      run_rust:
        description: "Run Rust CI jobs"
        type: boolean
        default: true
      run_python:
        description: "Run Python CI jobs"
        type: boolean
        default: false
      
      # Rust configuration
      rust_toolchain:
        description: "Rust toolchain (stable/beta/nightly)"
        type: string
        default: "stable"
      rust_msrv:
        description: "Minimum Supported Rust Version (e.g., 1.70.0)"
        type: string
        default: ""
      rust_features:
        description: "Space-separated Cargo features"
        type: string
        default: ""
      rust_no_default_features:
        description: "Pass --no-default-features to cargo"
        type: boolean
        default: false
      rust_workspace_members:
        description: "Space-separated workspace members to test (empty = all)"
        type: string
        default: ""
      
      # Python configuration
      python_version:
        description: "Python version (e.g., 3.11)"
        type: string
        default: "3.11"
      python_requirements:
        description: "Path to requirements file"
        type: string
        default: "requirements.txt"
      
      # System dependencies
      install_alsa:
        description: "Install ALSA audio libraries (Linux)"
        type: boolean
        default: false
      install_apt_packages:
        description: "Space-separated apt packages to install"
        type: string
        default: ""
      crate_system_deps:
        description: "JSON mapping crate names to system deps (e.g., {\"coldvox-audio\": \"libasound2-dev\"})"
        type: string
        default: "{}"
      
      # Testing configuration
      test_timeout_minutes:
        description: "Test timeout in minutes"
        type: number
        default: 30
      continue_on_error:
        description: "Continue workflow even if tests fail"
        type: boolean
        default: false
      use_nextest:
        description: "Use cargo-nextest for faster test execution"
        type: boolean
        default: false
      use_sccache:
        description: "Use sccache for build caching"
        type: boolean
        default: false
      run_cargo_deny:
        description: "Run cargo-deny for license/security checks"
        type: boolean
        default: true
      max_parallel:
        description: "Maximum parallel jobs in matrix (0 = unlimited)"
        type: number
        default: 0

permissions:
  contents: read

jobs:
  # ============================================================
  # WORKSPACE MEMBER DETECTION
  # ============================================================
  detect-members:
    name: Detect Workspace Members
    if: inputs.run_rust && hashFiles('**/Cargo.toml') != ''
    runs-on: ubuntu-latest
    outputs:
      members: ${{ steps.detect.outputs.members }}
      matrix: ${{ steps.detect.outputs.matrix }}
    steps:
      - uses: actions/checkout@3df4ab11eba7bda6032a0b82a6bb43b11571feac # v4.1.7
      
      - name: Detect workspace members
        id: detect
        run: |
          if [ -n "${{ inputs.rust_workspace_members }}" ]; then
            # Use provided members
            MEMBERS="${{ inputs.rust_workspace_members }}"
          elif [ -f "Cargo.toml" ] && grep -q "\[workspace\]" Cargo.toml; then
            # Auto-detect from cargo metadata
            MEMBERS=$(cargo metadata --no-deps --format-version 1 | \
              jq -r '.workspace_members[] | split(" ")[0] | split("/")[-1]' | \
              tr '\n' ' ')
          else
            # Single package project
            MEMBERS="."
          fi
          
          # Create matrix JSON
          MATRIX=$(echo $MEMBERS | jq -R -c 'split(" ") | map(select(length > 0)) | {"member": .}')
          echo "members=$MEMBERS" >> $GITHUB_OUTPUT
          echo "matrix=$MATRIX" >> $GITHUB_OUTPUT

  # ============================================================
  # RUST CI JOB (PER-CRATE MATRIX)
  # ============================================================
  rust:
    name: Rust CI - ${{ matrix.member }}
    needs: detect-members
    if: inputs.run_rust && hashFiles('**/Cargo.toml') != ''
    runs-on: ubuntu-latest
    timeout-minutes: ${{ inputs.test_timeout_minutes }}
    continue-on-error: ${{ inputs.continue_on_error }}
    strategy:
      matrix: ${{ fromJSON(needs.detect-members.outputs.matrix) }}
      max-parallel: ${{ inputs.max_parallel > 0 && inputs.max_parallel || 999 }}
    
    steps:
      - name: Checkout code
        uses: actions/checkout@3df4ab11eba7bda6032a0b82a6bb43b11571feac # v4.1.7
        with:
          fetch-depth: 0
      
      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@1482605bfc5719782e1267fd0c0cc350fe7646b8 # v1
        with:
          toolchain: ${{ inputs.rust_toolchain }}
          components: clippy, rustfmt
      
      - name: Setup sccache
        if: inputs.use_sccache
        uses: mozilla-actions/sccache-action@c94e5a96e0ba0fb6816ceae10c4cd8e800724ddd # v0.0.6
      
      - name: Configure sccache
        if: inputs.use_sccache
        run: |
          echo "SCCACHE_GHA_ENABLED=true" >> $GITHUB_ENV
          echo "RUSTC_WRAPPER=sccache" >> $GITHUB_ENV
      
      - name: Cache Cargo dependencies
        uses: Swatinem/rust-cache@23bce251a8cd2ffc3c1075eaa2367cf899916d84 # v2.7.3
        with:
          key: ${{ matrix.member }}-${{ inputs.rust_features }}
      
      - name: Install ALSA libraries
        if: inputs.install_alsa
        run: |
          sudo apt-get update
          sudo apt-get install -y libasound2-dev
      
      - name: Install additional apt packages
        if: inputs.install_apt_packages != ''
        run: |
          sudo apt-get update
          sudo apt-get install -y ${{ inputs.install_apt_packages }}
      
      - name: Install crate-specific system dependencies
        if: inputs.crate_system_deps != '{}'
        run: |
          DEPS=$(echo '${{ inputs.crate_system_deps }}' | jq -r '."${{ matrix.member }}" // ""')
          if [ -n "$DEPS" ]; then
            sudo apt-get update
            sudo apt-get install -y $DEPS
          fi
      
      - name: Install cargo-nextest
        if: inputs.use_nextest
        uses: taiki-e/install-action@5ff18d7fb42b9cb96e9b08bd87f965bb411b4daf # v2.44.45
        with:
          tool: cargo-nextest
      
      - name: Check formatting
        run: |
          if [ "${{ matrix.member }}" = "." ]; then
            cargo fmt --all -- --check
          else
            cargo fmt -p ${{ matrix.member }} -- --check
          fi
      
      - name: Verify lockfile
        run: |
          cargo check --locked --workspace
          if git diff --exit-code Cargo.lock; then
            echo "Lockfile is up to date"
          else
            echo "::error::Cargo.lock is out of date. Run 'cargo update' locally and commit."
            exit 1
          fi
      
      - name: Run Clippy
        run: |
          PACKAGE_ARG=""
          if [ "${{ matrix.member }}" != "." ]; then
            PACKAGE_ARG="-p ${{ matrix.member }}"
          else
            PACKAGE_ARG="--workspace"
          fi
          
          FEATURES_ARG=""
          if [ "${{ inputs.rust_no_default_features }}" = "true" ]; then
            FEATURES_ARG="--no-default-features"
          fi
          if [ -n "${{ inputs.rust_features }}" ]; then
            FEATURES_ARG="$FEATURES_ARG --features ${{ inputs.rust_features }}"
          fi
          cargo clippy $PACKAGE_ARG --all-targets $FEATURES_ARG -- -D warnings
      
      - name: Build
        run: |
          PACKAGE_ARG=""
          if [ "${{ matrix.member }}" != "." ]; then
            PACKAGE_ARG="-p ${{ matrix.member }}"
          else
            PACKAGE_ARG="--workspace"
          fi
          
          FEATURES_ARG=""
          if [ "${{ inputs.rust_no_default_features }}" = "true" ]; then
            FEATURES_ARG="--no-default-features"
          fi
          if [ -n "${{ inputs.rust_features }}" ]; then
            FEATURES_ARG="$FEATURES_ARG --features ${{ inputs.rust_features }}"
          fi
          cargo build $PACKAGE_ARG --all-targets --locked $FEATURES_ARG
      
      - name: Run tests
        run: |
          PACKAGE_ARG=""
          if [ "${{ matrix.member }}" != "." ]; then
            PACKAGE_ARG="-p ${{ matrix.member }}"
          else
            PACKAGE_ARG="--workspace"
          fi
          
          FEATURES_ARG=""
          if [ "${{ inputs.rust_no_default_features }}" = "true" ]; then
            FEATURES_ARG="--no-default-features"
          fi
          if [ -n "${{ inputs.rust_features }}" ]; then
            FEATURES_ARG="$FEATURES_ARG --features ${{ inputs.rust_features }}"
          fi
          
          if [ "${{ inputs.use_nextest }}" = "true" ]; then
            cargo nextest run $PACKAGE_ARG --locked $FEATURES_ARG
          else
            cargo test $PACKAGE_ARG --locked $FEATURES_ARG -- --nocapture
          fi
      
      - name: Run doc tests
        run: |
          PACKAGE_ARG=""
          if [ "${{ matrix.member }}" != "." ]; then
            PACKAGE_ARG="-p ${{ matrix.member }}"
          else
            PACKAGE_ARG="--workspace"
          fi
          
          FEATURES_ARG=""
          if [ "${{ inputs.rust_no_default_features }}" = "true" ]; then
            FEATURES_ARG="--no-default-features"
          fi
          if [ -n "${{ inputs.rust_features }}" ]; then
            FEATURES_ARG="$FEATURES_ARG --features ${{ inputs.rust_features }}"
          fi
          
          cargo test --doc $PACKAGE_ARG --locked $FEATURES_ARG
      
      - name: Generate documentation
        run: |
          PACKAGE_ARG=""
          if [ "${{ matrix.member }}" != "." ]; then
            PACKAGE_ARG="-p ${{ matrix.member }}"
          else
            PACKAGE_ARG="--workspace"
          fi
          
          FEATURES_ARG=""
          if [ "${{ inputs.rust_no_default_features }}" = "true" ]; then
            FEATURES_ARG="--no-default-features"
          fi
          if [ -n "${{ inputs.rust_features }}" ]; then
            FEATURES_ARG="$FEATURES_ARG --features ${{ inputs.rust_features }}"
          fi
          
          RUSTDOCFLAGS="-D warnings" cargo doc $PACKAGE_ARG --no-deps --locked $FEATURES_ARG
      
      - name: Check examples
        run: |
          PACKAGE_ARG=""
          if [ "${{ matrix.member }}" != "." ]; then
            PACKAGE_ARG="-p ${{ matrix.member }}"
          else
            PACKAGE_ARG="--workspace"
          fi
          
          FEATURES_ARG=""
          if [ "${{ inputs.rust_no_default_features }}" = "true" ]; then
            FEATURES_ARG="--no-default-features"
          fi
          if [ -n "${{ inputs.rust_features }}" ]; then
            FEATURES_ARG="$FEATURES_ARG --features ${{ inputs.rust_features }}"
          fi
          
          cargo check $PACKAGE_ARG --examples --locked $FEATURES_ARG || true
      
      - name: Check package metadata
        run: |
          if [ "${{ matrix.member }}" != "." ]; then
            cargo package -p ${{ matrix.member }} --no-verify --allow-dirty || true
          fi
      
      - name: Upload artifacts on failure
        if: failure()
        uses: actions/upload-artifact@50769540e7f4bd5e21e526ee35c689e35e0d6874 # v4.4.0
        with:
          name: failure-artifacts-${{ matrix.member }}
          path: |
            target/debug/deps/*.log
            **/*.log
            **/test-output/
          retention-days: 7

  # ============================================================
  # RUST MSRV JOB
  # ============================================================
  rust-msrv:
    name: Rust MSRV Check
    if: inputs.run_rust && inputs.rust_msrv != '' && hashFiles('**/Cargo.toml') != ''
    runs-on: ubuntu-latest
    timeout-minutes: ${{ inputs.test_timeout_minutes }}
    continue-on-error: true
    
    steps:
      - uses: actions/checkout@3df4ab11eba7bda6032a0b82a6bb43b11571feac # v4.1.7
      
      - name: Install MSRV toolchain
        uses: dtolnay/rust-toolchain@1482605bfc5719782e1267fd0c0cc350fe7646b8 # v1
        with:
          toolchain: ${{ inputs.rust_msrv }}
      
      - name: Cache Cargo dependencies
        uses: Swatinem/rust-cache@23bce251a8cd2ffc3c1075eaa2367cf899916d84 # v2.7.3
        with:
          key: msrv-${{ inputs.rust_msrv }}
      
      - name: Install system dependencies
        if: inputs.install_alsa || inputs.install_apt_packages != ''
        run: |
          sudo apt-get update
          if [ "${{ inputs.install_alsa }}" = "true" ]; then
            sudo apt-get install -y libasound2-dev
          fi
          if [ -n "${{ inputs.install_apt_packages }}" ]; then
            sudo apt-get install -y ${{ inputs.install_apt_packages }}
          fi
      
      - name: Build with MSRV
        run: cargo build --workspace --locked --no-default-features
      
      - name: Test with MSRV
        run: cargo test --workspace --locked --no-default-features

  # ============================================================
  # CARGO DENY JOB
  # ============================================================
  cargo-deny:
    name: Cargo Deny Check
    if: inputs.run_rust && inputs.run_cargo_deny && hashFiles('**/Cargo.toml') != ''
    runs-on: ubuntu-latest
    timeout-minutes: 10
    
    steps:
      - uses: actions/checkout@3df4ab11eba7bda6032a0b82a6bb43b11571feac # v4.1.7
      
      - name: Run cargo-deny
        uses: EmbarkStudios/cargo-deny-action@8371184bd11e21dcf8ac82ebf8c9c9f74ebf7268 # v2.0.1
        with:
          command: check all
          arguments: --workspace

  # ============================================================
  # DEPENDENCY ANALYSIS JOB
  # ============================================================
  dependency-analysis:
    name: Dependency Analysis
    if: inputs.run_rust && hashFiles('**/Cargo.toml') != ''
    runs-on: ubuntu-latest
    timeout-minutes: 10
    
    steps:
      - uses: actions/checkout@3df4ab11eba7bda6032a0b82a6bb43b11571feac # v4.1.7
      
      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@1482605bfc5719782e1267fd0c0cc350fe7646b8 # v1
        with:
          toolchain: stable
      
      - name: Check for duplicate dependencies
        run: |
          cargo tree --workspace --duplicates --locked
      
      - name: Security audit
        uses: rustsec/audit-check@dd51754611baa5c0affe6c19adb60f61f165e6e4 # v2.0.0
        with:
          token: ${{ secrets.GITHUB_TOKEN }}
      
      - name: Check minimal versions (nightly)
        continue-on-error: true
        run: |
          rustup toolchain install nightly
          cargo +nightly update -Z minimal-versions
          cargo +nightly build --workspace --all-targets || true

  # ============================================================
  # PYTHON CI JOB
  # ============================================================
  python:
    name: Python CI
    if: inputs.run_python && (hashFiles('**/pyproject.toml') != '' || hashFiles('**/requirements.txt') != '' || hashFiles('**/setup.py') != '')
    runs-on: ubuntu-latest
    timeout-minutes: ${{ inputs.test_timeout_minutes }}
    continue-on-error: ${{ inputs.continue_on_error }}
    
    steps:
      - name: Checkout code
        uses: actions/checkout@3df4ab11eba7bda6032a0b82a6bb43b11571feac # v4.1.7
        with:
          fetch-depth: 0
      
      - name: Setup Python
        uses: actions/setup-python@0b93645e9fea7318ecaed2b359559ac225c90a2b # v5.3.0
        with:
          python-version: ${{ inputs.python_version }}
          cache: 'pip'
      
      - name: Install additional apt packages
        if: inputs.install_apt_packages != ''
        run: |
          sudo apt-get update
          sudo apt-get install -y ${{ inputs.install_apt_packages }}
      
      - name: Install Python dependencies
        run: |
          python -m pip install --upgrade pip
          # Try multiple common dependency files
          if [ -f "${{ inputs.python_requirements }}" ]; then
            pip install -r "${{ inputs.python_requirements }}"
          elif [ -f "requirements.txt" ]; then
            pip install -r requirements.txt
          fi
          if [ -f "pyproject.toml" ]; then
            pip install -e .
          elif [ -f "setup.py" ]; then
            pip install -e .
          fi
          # Install common CI tools
          pip install ruff pytest pytest-cov
      
      - name: Format check with ruff
        run: ruff format --check .
      
      - name: Lint with ruff
        run: ruff check .
      
      - name: Type check with mypy
        run: |
          pip install mypy
          mypy . || true  # Don't fail on type errors initially
        continue-on-error: true
      
      - name: Run tests with pytest
        run: pytest -v --cov=. --cov-report=term-missing
      
      - name: Security check with bandit
        run: |
          pip install bandit[toml]
          bandit -r . -ll || true  # Low severity, don't fail initially
        continue-on-error: true

  # ============================================================
  # DEPENDENCY CHECK JOB (Both Languages)
  # ============================================================
  dependency-check:
    name: Dependency Security Check
    if: (inputs.run_rust && hashFiles('**/Cargo.toml') != '') || (inputs.run_python && hashFiles('**/requirements.txt') != '')
    runs-on: ubuntu-latest
    timeout-minutes: 10
    
    steps:
      - name: Checkout code
        uses: actions/checkout@3df4ab11eba7bda6032a0b82a6bb43b11571feac # v4.1.7
      
      - name: Run Trivy security scanner
        uses: aquasecurity/trivy-action@6e7b7d1fd3e4fef0c5fa8cce1229c54b2c9bd0d8 # v0.24.0
        with:
          scan-type: 'fs'
          scan-ref: '.'
          severity: 'CRITICAL,HIGH'
          exit-code: '0'  # Don't fail the build initially
```

## Versioning Strategy

### Semantic Versioning

- **v1**: Initial stable release
- **v1.x**: Backward-compatible improvements
- **v2**: Breaking changes requiring shim updates

### Tagging Process

```bash
# In the org/.github repository
git tag -a v1.0.0 -m "Initial release of common CI workflow"
git push origin v1.0.0

# Create major version tag for stability
git tag -a v1 -m "v1 stable" v1.0.0^{}
git push origin v1
```

## Security Considerations

### Action Pinning (2025 Best Practice)

All third-party actions MUST be pinned to full-length commit SHAs with version comments:
- Prevents supply chain attacks and tag mutation
- Managed via Dependabot/Renovate for automated updates
- Pattern: `uses: owner/action@SHA # vX.Y.Z`
- Enforce via organization policy settings
- Verify SHAs are from official repos, not forks

### Permissions

- Minimal permissions (`contents: read`)
- No write permissions in reusable workflow
- Calling workflows can extend permissions if needed

### Secrets

- Use `secrets: inherit` in calling workflow
- No hardcoded secrets in reusable workflow
- Support for organization-level secrets

## Performance Optimizations

### Caching

- **Rust**: Swatinem/rust-cache for Cargo dependencies
- **sccache**: Optional distributed compilation cache
- **Python**: Built-in pip caching via setup-python
- **Docker**: Layer caching for custom images
- **Model/Asset caching**: Use cache action with restore-keys for large files

### Concurrency

Calling workflows should implement:
```yaml
concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true
```

### Auto-detection

- Language detection via `hashFiles()`
- Workspace member detection via `cargo metadata`
- Skip jobs when language files absent
- Override with explicit inputs
- Per-crate system dependency mapping

## Usage Examples

### Basic Rust Project

```yaml
# .github/workflows/ci.yml
name: CI
on: [push, pull_request]

jobs:
  ci:
    uses: myorg/.github/.github/workflows/lang-ci.yml@v1
    with:
      run_rust: true
      run_python: false
```

### Multi-Crate Workspace Project

```yaml
# .github/workflows/ci.yml
name: CI
on: [push, pull_request]

jobs:
  ci:
    uses: myorg/.github/.github/workflows/lang-ci.yml@v1
    with:
      run_rust: true
      rust_workspace_members: "coldvox-audio coldvox-vad coldvox-stt"
      crate_system_deps: '{"coldvox-audio": "libasound2-dev"}'
      use_nextest: true
      use_sccache: true
```

### Workspace with Feature Combinations

```yaml
# .github/workflows/ci.yml
name: CI
on: [push, pull_request]

jobs:
  # Base CI
  ci:
    uses: myorg/.github/.github/workflows/lang-ci.yml@v1
    with:
      run_rust: true
      rust_no_default_features: true
      use_nextest: true
      
  # Feature combination testing
  feature-matrix:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: moonrepo/setup-rust@v1
        with:
          bins: cargo-hack
      - run: cargo hack test --each-feature --workspace-excludes coldvox-gui
```

### Python Project with System Dependencies

```yaml
# .github/workflows/ci.yml
name: CI
on: [push, pull_request]

jobs:
  ci:
    uses: myorg/.github/.github/workflows/lang-ci.yml@v1
    with:
      run_rust: false
      run_python: true
      python_version: "3.12"
      install_apt_packages: "libpq-dev"
```

### Mixed Language Project

```yaml
# .github/workflows/ci.yml
name: CI
on: [push, pull_request]

jobs:
  ci:
    uses: myorg/.github/.github/workflows/lang-ci.yml@v1
    with:
      run_rust: true
      run_python: true
      rust_features: "cli"
      python_version: "3.11"
```

### Platform Priority Configuration

```yaml
# Linux-first, Windows-second approach
jobs:
  linux-ci:  # Primary platform - must pass
    uses: myorg/.github/.github/workflows/lang-ci.yml@v1
    with:
      run_rust: true
      use_nextest: true
      install_alsa: true  # Linux audio support
      
  windows-ci:  # Secondary platform - best effort
    runs-on: windows-latest
    continue-on-error: true
    steps:
      - uses: actions/checkout@v4
      - uses: moonrepo/setup-rust@v1
      # No system deps needed - WASAPI works out of the box
      - run: cargo test --workspace --locked
  
  macos-ci:  # Optional - for releases only
    if: startsWith(github.ref, 'refs/tags/')
    runs-on: macos-latest
    continue-on-error: true
    steps:
      - uses: actions/checkout@v4
      - uses: moonrepo/setup-rust@v1
      # CoreAudio works without deps, portaudio optional
      - run: cargo test --workspace --locked
```

## Platform-Specific Dependencies

### Audio Libraries by Platform

- **Linux**: ALSA (libasound2-dev) - required for CPAL
- **Windows**: WASAPI - built-in, no deps needed
- **macOS**: CoreAudio - built-in, portaudio optional

### Example Configuration

```yaml
crate_system_deps: '{
  "coldvox-audio": "libasound2-dev",        # Linux only
  "coldvox-text-injection": "libxdo-dev libxtst-dev",  # X11
  "coldvox-gui": "libgl1-mesa-dev"          # OpenGL
}'
```

Note: The workflow automatically skips Linux-specific deps on Windows/macOS.

## Extension Points

### Adding New Languages

To add a new language (e.g., Go):

1. Add input toggles:
   ```yaml
   run_go:
     type: boolean
     default: false
   ```

2. Add language-specific job:
   ```yaml
   go:
     if: inputs.run_go && hashFiles('**/go.mod') != ''
     # ... job steps
   ```

### Adding Matrix Testing

For projects needing OS/version matrices, create specialized workflows:
```yaml
# lang-ci-matrix.yml
strategy:
  matrix:
    os: [ubuntu-latest, windows-latest, macos-latest]
    rust: [stable, beta]
```

## Maintenance

### Dependabot Configuration

```yaml
# <org>/.github/.github/dependabot.yml
version: 2
updates:
  - package-ecosystem: "github-actions"
    directory: "/"
    schedule:
      interval: "weekly"
    commit-message:
      prefix: "chore(deps)"
    # Group updates to reduce PR noise
    groups:
      actions:
        patterns:
          - "*"
```

### Minimal Versions Testing

Periodically test with minimal dependency versions:
```yaml
- name: Test minimal versions (nightly)
  run: |
    cargo +nightly update -Z minimal-versions
    cargo +nightly test --workspace
```

### Monitoring

- Track workflow run times
- Monitor failure rates
- Review security alerts
- Update actions monthly

## Migration Checklist

For organizations adopting this workflow:

- [ ] Create `<org>/.github` repository
- [ ] Add `lang-ci.yml` workflow file
- [ ] Configure Dependabot
- [ ] Tag initial version (v1.0.0)
- [ ] Create major version tag (v1)
- [ ] Document in org README
- [ ] Create example shims
- [ ] Pilot with 1-2 projects
- [ ] Roll out organization-wide

## FAQ

**Q: Can projects override specific steps?**  
A: No, but they can add additional jobs in their calling workflow.

**Q: How to handle private dependencies?**  
A: Use `secrets: inherit` and configure tokens at the repository level.

**Q: What about deployment workflows?**  
A: Keep deployment separate; this is for CI only.

**Q: How to test workflow changes?**  
A: Use a test branch and reference it: `uses: org/.github/.github/workflows/lang-ci.yml@test-branch`

**Q: How to handle platform-specific code?**  
A: Use Linux as primary (must pass), Windows/macOS as secondary (continue-on-error).

**Q: Should we test all feature combinations?**  
A: Use cargo-hack selectively on critical crates to avoid exponential test times.

**Q: How often should we update pinned actions?**  
A: Weekly via Dependabot, review security advisories immediately.