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
      rust_features:
        description: "Space-separated Cargo features"
        type: string
        default: ""
      rust_no_default_features:
        description: "Pass --no-default-features to cargo"
        type: boolean
        default: false
      
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
      
      # Testing configuration
      test_timeout_minutes:
        description: "Test timeout in minutes"
        type: number
        default: 30
      continue_on_error:
        description: "Continue workflow even if tests fail"
        type: boolean
        default: false

permissions:
  contents: read

jobs:
  # ============================================================
  # RUST CI JOB
  # ============================================================
  rust:
    name: Rust CI
    if: inputs.run_rust && hashFiles('**/Cargo.toml') != ''
    runs-on: ubuntu-latest
    timeout-minutes: ${{ inputs.test_timeout_minutes }}
    continue-on-error: ${{ inputs.continue_on_error }}
    
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
      
      - name: Cache Cargo dependencies
        uses: Swatinem/rust-cache@23bce251a8cd2ffc3c1075eaa2367cf899916d84 # v2.7.3
      
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
      
      - name: Check formatting
        run: cargo fmt --all -- --check
      
      - name: Run Clippy
        run: |
          FEATURES_ARG=""
          if [ "${{ inputs.rust_no_default_features }}" = "true" ]; then
            FEATURES_ARG="--no-default-features"
          fi
          if [ -n "${{ inputs.rust_features }}" ]; then
            FEATURES_ARG="$FEATURES_ARG --features ${{ inputs.rust_features }}"
          fi
          cargo clippy --workspace --all-targets $FEATURES_ARG -- -D warnings
      
      - name: Build
        run: |
          FEATURES_ARG=""
          if [ "${{ inputs.rust_no_default_features }}" = "true" ]; then
            FEATURES_ARG="--no-default-features"
          fi
          if [ -n "${{ inputs.rust_features }}" ]; then
            FEATURES_ARG="$FEATURES_ARG --features ${{ inputs.rust_features }}"
          fi
          cargo build --workspace --all-targets $FEATURES_ARG
      
      - name: Run tests
        run: |
          FEATURES_ARG=""
          if [ "${{ inputs.rust_no_default_features }}" = "true" ]; then
            FEATURES_ARG="--no-default-features"
          fi
          if [ -n "${{ inputs.rust_features }}" ]; then
            FEATURES_ARG="$FEATURES_ARG --features ${{ inputs.rust_features }}"
          fi
          cargo test --workspace $FEATURES_ARG -- --nocapture
      
      - name: Generate documentation
        run: |
          FEATURES_ARG=""
          if [ "${{ inputs.rust_no_default_features }}" = "true" ]; then
            FEATURES_ARG="--no-default-features"
          fi
          if [ -n "${{ inputs.rust_features }}" ]; then
            FEATURES_ARG="$FEATURES_ARG --features ${{ inputs.rust_features }}"
          fi
          cargo doc --workspace --no-deps $FEATURES_ARG
      
      - name: Security audit
        uses: rustsec/audit-check@dd51754611baa5c0affe6c19adb60f61f165e6e4 # v2.0.0
        with:
          token: ${{ secrets.GITHUB_TOKEN }}

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

### Action Pinning

All third-party actions are pinned to specific commit SHAs with version comments:
- Prevents supply chain attacks
- Managed via Dependabot for updates
- Pattern: `uses: owner/action@SHA # vX.Y.Z`

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

- **Rust**: Swatinem/rust-cache for Cargo
- **Python**: Built-in pip caching via setup-python
- **Docker**: Layer caching for custom images

### Concurrency

Calling workflows should implement:
```yaml
concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true
```

### Auto-detection

- Language detection via `hashFiles()`
- Skip jobs when language files absent
- Override with explicit inputs

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