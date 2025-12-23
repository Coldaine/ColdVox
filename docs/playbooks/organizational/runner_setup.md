---
doc_type: playbook
subsystem: general
version: 1.0.0
status: draft
owners: Documentation Working Group
last_reviewed: 2025-10-19
---

# ColdVox Self-Hosted Runner Complete Setup Documentation

**Document Created**: 2025-09-11
**Purpose**: Exhaustive documentation of all configurations, settings, and workflow files
**Status**: Current as of commit 959a04a

---

## System Information

### Hardware Specifications
- **Model**: HP EliteBook 840 14 inch G10 Notebook PC
- **CPU**: 13th Gen Intel Core i7-1365U (10 cores, 12 threads, up to 5.2GHz)
- **RAM**: 30GB
- **Storage**: 238.5GB NVMe SSD (/dev/nvme0n1)
- **Swap**: 42GB (34GB disk + 8GB zram)

### Operating System
- **Distribution**: Nobara Linux 42 (KDE Plasma Desktop Edition)
- **Based on**: Fedora 42 (RHEL/CentOS compatible)
- **Kernel**: Linux 6.16.3-201.nobara.fc42.x86_64
- **Architecture**: x86_64
- **Package Manager**: DNF 5.2.16 (dnf5)

### Network Configuration
- **Local IP**: 192.168.1.66
- **Hostname**: laptop-extra
- **Username**: coldaine

---

## GitHub Actions Runner Configuration

### Runner Installation Location
```
/home/coldaine/actions-runner/
```

### Model Management

The CI system now uses the application's built-in model autodetection and auto-extraction capabilities. Runners no longer require pre-provisioned, cached models.

**Requirements:**
- A `vosk-model-*.zip` file must be present in the project's `vendor/` directory.
- The `setup_vosk.rs` script (run during CI) will copy this zip file to the project root, where the application will find and extract it on first use.

**Workflow:**
1. The `setup-vosk-model` job in `ci.yml` copies the model zip from `vendor/` to the workspace root.
2. When tests are run, the `coldvox-stt-vosk` crate automatically finds the zip, extracts it to the `models/` directory, and uses the extracted model.
3. Subsequent runs will find the extracted model and skip the extraction step.

This approach removes the dependency on a fixed-path runner cache and makes the CI setup more portable.

### Runner Labels
```
[self-hosted, Linux, X64, fedora, nobara]
```

### Runner Service Status
```bash
# Service files and processes
coldaine  230516  /bin/bash /home/coldaine/Desktop/actions-runner-logs.sh
coldaine  230517  journalctl -u actions-runner -f --no-pager
coldaine  298971  /bin/bash /home/coldaine/actions-runner/runsvc.sh
```

### Runner Environment Configuration
**File**: `/home/coldaine/actions-runner/.env`
```
LANG=en_US.utf8
```

### Cache Directory Structure
```
/home/coldaine/ActionRunnerCache/
├── vosk-models/
│   └── vosk-model-small-en-us-0.15/
├── libvosk-setup/
│   └── vosk-linux-x86_64-0.3.45/
└── (planned: rust-toolchains/, system-packages/)
```

---

## System Library Configuration

### libvosk Installation
**Location**: `/usr/local/lib/libvosk.so` (25,986,496 bytes)
**Header**: `/usr/local/include/vosk_api.h`

**Dynamic Linker Configuration**:
**File**: `/etc/ld.so.conf.d/vosk.conf`
```
/usr/local/lib
```

**Verification**:
```bash
$ ldconfig -p | grep vosk
libvosk.so (libc6,x86-64) => /usr/local/lib/libvosk.so
```

### System Dependencies Installed
**Via DNF**:
```bash
alsa-lib-devel xdotool libXtst-devel wget unzip @development-tools
xorg-x11-server-Xvfb fluxbox dbus-x11 at-spi2-core wl-clipboard
xclip ydotool xorg-x11-utils wmctrl gtk3-devel
```

### Build Tooling

#### sccache (Rust Build Cache)

sccache caches Rust compilation artifacts, reducing incremental build times by 30-60%.

**Installation** (run once on runner):
```bash
cd /path/to/ColdVox
just setup-sccache
```

**Verification**:
```bash
sccache --version
sccache --show-stats
```

**Location**: `~/.cargo/bin/sccache`
**Cache**: `~/.cache/sccache`

The CI workflow automatically detects and uses sccache if available. No additional configuration needed after installation.

#### just (Command Runner)

The project uses `just` as a command runner for development tasks.

**Installation**:
```bash
cargo install just --locked
```

**One-time setup** (installs all dev tools including sccache):
```bash
just setup
```

---

## Complete Workflow Files

### 1. Main CI Workflow

**File**: `.github/workflows/ci.yml`

```yaml
---
name: CI

on:
  pull_request:
    branches: [main]
    types: [opened, synchronize, reopened]
  workflow_dispatch:

permissions:
  contents: read
  pull-requests: read
  checks: write

concurrency:
  group: ci-${{ github.ref }}
  cancel-in-progress: true

env:
  RUST_BACKTRACE: 1
  CARGO_TERM_COLOR: always
  CARGO_INCREMENTAL: 0
  RUST_TEST_TIME_UNIT: 10000
  RUST_TEST_TIME_INTEGRATION: 30000

jobs:
  validate-workflows:
    name: Validate Workflow Definitions
    runs-on: [self-hosted, Linux, X64, fedora, nobara]
    continue-on-error: true # Optional validation
    env:
      GH_TOKEN: ${{ github.token }}
    steps:
      - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683 # v4.2.2
      - name: Validate with gh
        shell: bash
        run: |
          set -euo pipefail
          if ! command -v gh >/dev/null 2>&1; then
            echo "gh CLI not found on runner, skipping workflow validation"
            exit 0
          fi

          shopt -s nullglob
          files=(.github/workflows/*.yml .github/workflows/*.yaml)
          if [[ ${#files[@]} -eq 0 ]]; then
            echo "No workflow files found"
            exit 0
          fi

          echo "Validating ${{ github.sha }} against ${#files[@]} workflow files..."
          failed=0
          for wf in "${files[@]}"; do
            echo "-- $wf"
            if ! gh workflow view "$wf" --ref "$GITHUB_SHA" --yaml >/dev/null 2>&1; then
              echo "ERROR: Failed to render $wf via gh" >&2
              failed=1
            fi
          done

          if [[ $failed -ne 0 ]]; then
            echo "One or more workflows failed server-side validation via gh." >&2
            exit 1
          fi
          echo "All workflows render via gh."

  setup-vosk-model:
    name: Setup Vosk Model
    runs-on: [self-hosted, Linux, X64, fedora, nobara]
    outputs:
      download-outcome: success
    steps:
      - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683 # v4.2.2

      - name: Setup Vosk Model
        run: |
          set -euo pipefail
          # This script will copy the model zip from vendor/ to the root
          ./scripts/setup_vosk.rs

  # Static checks, formatting, linting, type-check, build, and docs
  build_and_check:
    name: Format, Lint, Typecheck, Build & Docs
    runs-on: [self-hosted, Linux, X64, fedora, nobara]
    needs: [setup-vosk-model]
    steps:
      - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683 # v4.2.2

      - name: Set up Rust toolchain
        uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          toolchain: stable
          components: rustfmt, clippy
          override: true

      - name: Setup ColdVox
        uses: ./.github/actions/setup-coldvox
        with:
          skip-toolchain: 'true'

      - name: Check formatting
        run: cargo fmt --all -- --check

      - name: Run clippy
        run: cargo clippy --all-targets --locked

      - name: Type check
        run: cargo check --workspace --all-targets --locked

      - name: Build
        run: cargo build --workspace --locked

      - name: Build documentation
        run: cargo doc --workspace --no-deps --locked

      - name: Run all tests (unit, integration, and E2E)
        if: needs.setup-vosk-model.outputs.download-outcome == 'success'
        run: |
          cargo test --workspace --locked

      - name: Upload test artifacts on failure
        if: failure()
        uses: actions/upload-artifact@v4
        with:
          name: test-artifacts-build-check
          path: |
            target/debug/deps/
            target/debug/build/
          retention-days: 7

  # MSRV validation
  msrv-check:
    name: MSRV Check (Rust 1.75)
    runs-on: [self-hosted, Linux, X64, fedora, nobara]
    steps:
      - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683 # v4.2.2

      - uses: dtolnay/rust-toolchain@e97e2d8cc328f1b50210efc529dca0028893a2d9 # v1
        with:
          toolchain: 1.75

      - uses: Swatinem/rust-cache@98c8021b550208e191a6a3145459bfc9fb29c4c0 # v2.8.0

      - name: Setup ColdVox
        uses: ./.github/actions/setup-coldvox
        with:
          skip-toolchain: 'true'

      - name: MSRV type check
        run: cargo check --workspace --all-targets --locked

      - name: MSRV build
        run: cargo build --workspace --locked

  # GUI groundwork check: explicitly pass if Qt 6 isn't installed
  gui-groundwork:
    name: GUI Groundwork (Qt optional)
    runs-on: [self-hosted, Linux, X64, fedora, nobara]
    steps:
      - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683 # v4.2.2

      - uses: dtolnay/rust-toolchain@e97e2d8cc328f1b50210efc529dca0028893a2d9 # v1
        with:
          toolchain: stable

      - uses: Swatinem/rust-cache@98c8021b550208e191a6a3145459bfc9fb29c4c0 # v2.8.0

      - name: Detect Qt 6
        id: detect-qt
        shell: bash
        run: |
          set -euo pipefail
          qt_found=false
          if command -v qmake6 >/dev/null 2>&1; then
            qt_found=true
          elif command -v qmake-qt6 >/dev/null 2>&1; then
            qt_found=true
          elif pkg-config --exists Qt6Core >/dev/null 2>&1; then
            qt_found=true
          fi
          echo "qt6=$qt_found" >> "$GITHUB_OUTPUT"
          if [[ "$qt_found" == "true" ]]; then
            echo "Qt 6 detected on runner."
          else
            echo "Qt 6 not detected; will skip qt-ui build and explicitly pass."
          fi

      - name: Build coldvox-gui with qt-ui feature
        if: steps.detect-qt.outputs.qt6 == 'true'
        run: |
          cargo check -p coldvox-gui --features qt-ui --locked

      - name: Qt not found — skip build
        if: steps.detect-qt.outputs.qt6 != 'true'
        run: |
          echo "WARNING: Qt 6 not detected. Skipping qt-ui build." >&2
          echo "Install Qt 6 to enable GUI testing."

  text_injection_tests:
    name: Text Injection Tests
    runs-on: [self-hosted, Linux, X64, fedora, nobara]
    needs: [setup-vosk-model]
    timeout-minutes: 30
    env:
      DISPLAY: :99
      RUST_LOG: debug
    steps:
      - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683 # v4.2.2
      - uses: dtolnay/rust-toolchain@e97e2d8cc328f1b50210efc529dca0028893a2d9 # v1
        with:
          toolchain: stable
      - uses: Swatinem/rust-cache@98c8021b550208e191a6a3145459bfc9fb29c4c0 # v2.8.0

      - name: Setup ColdVox
        uses: ./.github/actions/setup-coldvox

      # Install dependencies for text injection backends (AT-SPI, clipboard, ydotool)
      - name: Install additional system dependencies
        run: |
          # Detect package manager and install appropriate packages
          if command -v dnf >/dev/null 2>&1; then
            # Fedora/RHEL/Nobara
            sudo dnf install -y --skip-unavailable \
              xorg-x11-server-Xvfb \
              fluxbox \
              dbus-x11 \
              at-spi2-core \
              wl-clipboard \
              xclip \
              ydotool \
              xorg-x11-utils \
              wmctrl \
              gtk3-devel
          elif command -v apt-get >/dev/null 2>&1; then
            # Ubuntu/Debian
            sudo apt-get update
            sudo apt-get install -y \
              xvfb \
              fluxbox \
              dbus-x11 \
              at-spi2-core \
              wl-clipboard \
              xclip \
              ydotool \
              x11-utils \
              wmctrl \
              libgtk-3-dev
          else
            echo "ERROR: Unsupported package manager" >&2
            exit 1
          fi

      - name: Start and verify headless environment
        run: |
          set -euo pipefail
          # Start Xvfb
          Xvfb :99 -screen 0 1024x768x24 &
          for i in {1..30}; do
            if xdpyinfo -display ":99" >/dev/null 2>&1; then echo "Xvfb ready"; break; fi
            sleep 0.5; [[ $i -eq 30 ]] && exit 1
          done
          # Start window manager
          fluxbox -display :99 &
          for i in {1..30}; do
            if wmctrl -m >/dev/null 2>&1; then echo "Window manager ready"; break; fi
            sleep 0.5; [[ $i -eq 30 ]] && exit 1
          done
          # Setup D-Bus session
          eval "$(dbus-launch --sh-syntax)"
          echo "DBUS_SESSION_BUS_ADDRESS=$DBUS_SESSION_BUS_ADDRESS" >> $GITHUB_ENV
          echo "DBUS_SESSION_BUS_PID=$DBUS_SESSION_BUS_PID" >> $GITHUB_ENV
          # Verify D-Bus and clipboard tools
          if ! pgrep -x "dbus-daemon" >/dev/null; then echo "D-Bus daemon not running" >&2; exit 1; fi
          echo "D-Bus is running."
          echo "DBUS_SESSION_BUS_ADDRESS: $DBUS_SESSION_BUS_ADDRESS"
          echo "DBUS_SESSION_BUS_PID: $DBUS_SESSION_BUS_PID"
          if ! command -v xclip >/dev/null; then echo "xclip not found"; exit 1; fi
          if ! command -v wl-paste >/dev/null; then echo "wl-clipboard not found"; exit 1; fi
          echo "Clipboard utilities are available."

      - name: Validate test prerequisites
        run: |
          echo "=== Test Environment Validation ==="
          echo "Display: $DISPLAY"
          echo "Available text injection backends:"
          command -v xdotool >/dev/null && echo "  - xdotool: $(xdotool --version 2>/dev/null || echo 'available')"
          command -v ydotool >/dev/null && echo "  - ydotool: available"
          command -v enigo >/dev/null && echo "  - enigo: available (Rust crate)"
          echo "GTK development libraries:"
          pkg-config --exists gtk+-3.0 && echo "  - GTK+ 3.0: available" || echo "  - GTK+ 3.0: not found"
          echo "System audio:"
          command -v alsa-info >/dev/null && echo "  - ALSA: available" || echo "  - ALSA: not found"
          echo "=== Validation Complete ==="

      - name: Test with real-injection-tests feature
        run: |
          dbus-run-session -- bash -lc '
            # Set per-test timeout to prevent hanging
            export RUST_TEST_TIME_UNIT="10000"  # 10 second timeout per test
            export RUST_TEST_TIME_INTEGRATION="30000"  # 30 second for integration tests
            cargo test -p coldvox-text-injection \
              --features real-injection-tests \
              -- --nocapture --test-threads=1 --timeout 600
          '

      - name: Build pipeline (default features)
        run: |
          dbus-run-session -- bash -c '
            set -euo pipefail
            echo "Testing default features..."
            cargo test -p coldvox-text-injection --locked

            echo "Testing without default features..."
            cargo test -p coldvox-text-injection --no-default-features --locked

            echo "Testing regex feature only..."
            cargo test -p coldvox-text-injection --no-default-features --features regex --locked
          '

      # Build main app to ensure integration compiles
      - name: Build main application
        run: cargo build --locked -p coldvox-app

      - name: Run E2E pipeline test
        if: needs.setup-vosk-model.outputs.download-outcome == 'success'
        env:
          VOSK_MODEL_PATH: ${{ needs.setup-vosk-model.outputs.model-path }}
        run: |
          cargo test -p coldvox-app --locked test_end_to_end_wav_pipeline -- --nocapture

      - name: Upload test artifacts on failure
        if: failure()
        uses: actions/upload-artifact@v4
        with:
          name: test-artifacts-text-injection
          path: |
            target/debug/deps/
            target/debug/build/
            models/
          retention-days: 7

      - name: Cleanup background processes
        if: always()
        run: |
          set -euo pipefail
          echo "Cleaning up background processes..."
          # Kill Xvfb
          pkill -f "Xvfb.*:99" || true
          # Kill fluxbox
          pkill -f "fluxbox.*:99" || true
          # Kill dbus-daemon if it was started by this session
          if [[ -n "${DBUS_SESSION_BUS_PID:-}" ]]; then
            kill "$DBUS_SESSION_BUS_PID" 2>/dev/null || true
          fi
          echo "Cleanup completed."

  # Security audit
  security:
    name: Security Audit
    if: github.ref == 'refs/heads/main' # Only run on main
    runs-on: [self-hosted, Linux, X64, fedora, nobara]
    steps:
      - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683 # v4.2.2

      - name: Set up Rust toolchain
        uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          toolchain: stable

      - uses: rustsec/audit-check@v2.0.0 # pin to release tag
        with:
          token: ${{ secrets.GITHUB_TOKEN }}
      - name: Run security audit
        run: cargo audit

  # Success marker job
  ci-success:
    name: CI Success
    if: always()
    needs: [validate-workflows, setup-vosk-model, build_and_check, msrv-check, gui-groundwork, text_injection_tests]
    runs-on: [self-hosted, Linux, X64, fedora, nobara]
    steps:
      - name: Check if all jobs succeeded
        run: |
          set -euo pipefail
          failed=0
          for res in \
            "${{ needs.validate-workflows.result }}" \
            "${{ needs.setup-vosk-model.result }}" \
            "${{ needs.build_and_check.result }}" \
            "${{ needs.msrv-check.result }}" \
            "${{ needs.gui-groundwork.result }}" \
            "${{ needs.text_injection_tests.result }}"; do
            if [[ "$res" == "failure" ]]; then
              failed=1
            fi
          done
          if [[ $failed -eq 1 ]]; then
            echo "One or more CI jobs failed"
            exit 1
          fi
          echo "All CI jobs succeeded (ignoring skipped)"
```

### 2. Vosk Integration Tests Workflow

**File**: `.github/workflows/vosk-integration.yml`

```yaml
name: Vosk STT Integration Tests

on:
  pull_request:
    branches: [main]
    paths:
      - 'crates/coldvox-stt-vosk/**'
      - 'crates/app/src/stt/**'
      - '.github/workflows/vosk-integration.yml'
  workflow_dispatch:
    inputs:
      model_type:
        description: 'Vosk model to use for testing'
        required: false
        default: 'small'
        type: choice
        options:
          - small
          - large

env:
  RUST_BACKTRACE: 1
  CARGO_TERM_COLOR: always
  CARGO_INCREMENTAL: 0

jobs:
  vosk-integration:
    name: Vosk STT Integration
    runs-on: [self-hosted, Linux, X64, fedora, nobara]
    timeout-minutes: 15
    steps:
      - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683 # v4.2.2

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy

      - name: Setup ColdVox
        uses: ./.github/actions/setup-coldvox

      - name: Setup Vosk Model from Cache
        run: |
          set -euo pipefail

          # Use pre-cached models from permanent cache location
          CACHE_DIR="/home/coldaine/ActionRunnerCache/vosk-models"
          MODEL_DIR="models"

          mkdir -p $MODEL_DIR

          # Link the small model for tests (remove existing if present)
          if [ -d "$CACHE_DIR/vosk-model-small-en-us-0.15" ]; then
            rm -rf "$MODEL_DIR/vosk-model-small-en-us-0.15"
            ln -sf "$CACHE_DIR/vosk-model-small-en-us-0.15" "$MODEL_DIR/"
            echo "✅ Linked cached vosk-model-small-en-us-0.15"
          else
            echo "❌ Error: Vosk model not found in cache at $CACHE_DIR"
            exit 1
          fi

          # Link the production model if available
          if [ -d "$CACHE_DIR/vosk-model-en-us-0.22" ]; then
            rm -rf "$MODEL_DIR/vosk-model-en-us-0.22"
            ln -sf "$CACHE_DIR/vosk-model-en-us-0.22" "$MODEL_DIR/"
            echo "✅ Linked cached vosk-model-en-us-0.22"
          fi

          ls -la $MODEL_DIR/

      - name: Install cargo-nextest
        run: cargo install cargo-nextest --locked

      - name: Build with Vosk
        run: |
          # Build both crates that use Vosk feature
          cargo build --locked -p coldvox-stt-vosk --features vosk
          cargo build --locked -p coldvox-app --features vosk

      - name: Run Vosk tests
        env:
          VOSK_MODEL_PATH: models/vosk-model-small-en-us-0.15
        run: |
          cargo test --locked -p coldvox-stt-vosk --features vosk -- --nocapture

      - name: Run end-to-end WAV pipeline test
        env:
          VOSK_MODEL_PATH: models/vosk-model-small-en-us-0.15
        run: |
          cargo test --locked -p coldvox-app --features vosk test_end_to_end_wav_pipeline --nocapture

      - name: Test Vosk examples
        env:
          VOSK_MODEL_PATH: models/vosk-model-small-en-us-0.15
        run: |
          cargo run --locked --example vosk_test --features vosk,examples -- --test-duration 5

      - name: Upload test artifacts on failure
        if: failure()
        uses: actions/upload-artifact@v4
        with:
          name: vosk-integration-artifacts
          path: |
            target/debug/deps/
            target/debug/build/
            models/
          retention-days: 3

      - name: Performance summary
        run: |
          echo "=== Vosk Integration Test Summary ==="
          echo "Model setup: ✅ Using cached models"
          echo "Build time: Fast (using Rust cache)"
          echo "Test execution: Complete"
```

### 3. Runner Diagnostic Workflow

**File**: `.github/workflows/runner-diagnostic.yml`

```yaml
# .github/workflows/runner-diagnostic.yml
name: Runner Diagnostic

on:
  workflow_dispatch:

jobs:
  diagnose:
    name: Run Diagnostic Checks
    runs-on: [self-hosted, Linux, X64, fedora, nobara] # Ensure this matches my runner's labels

    steps:
      - name: 1. Print Runner Environment Details
        run: |
          echo "--- Runner Identity ---"
          echo "User: $(whoami)"
          echo "Home: $HOME"
          echo "pwd: $(pwd)"
          echo ""
          echo "--- OS Information ---"
          cat /etc/os-release || true
          echo ""
          echo "--- PATH ---"
          echo "$PATH"

      - name: 2. Test Network and DNS from Runner
        run: |
          echo "--- Testing DNS resolution for codeload.github.com ---"
          if command -v dig >/dev/null 2>&1; then
            dig codeload.github.com || echo "dig failed"
          else
            echo "dig not installed"
          fi
          echo ""
          echo "Attempting getent hosts lookup:"
          getent hosts codeload.github.com || echo "getent failed"
          echo ""
          echo "--- Testing HTTPS connection to codeload.github.com ---"
          curl -v --connect-timeout 10 --retry 3 --retry-delay 2 https://codeload.github.com || echo "curl failed"

      - name: 3. Test Package Manager and Sudo Permissions
        run: |
          echo "--- Checking sudo capabilities ---"
          if sudo -n true 2>/dev/null; then
            echo "Sudo access without password confirmed."
            echo "--- Attempting to install 'xdotool' (as a test) ---"
            sudo dnf install -y --skip-unavailable xdotool || echo "xdotool install failed"
            echo "--- Verifying xdotool installation ---"
            command -v xdotool && xdotool --version || echo "xdotool not available"
          else
            echo "ERROR: Sudo requires a password or is not configured for this user."
            exit 1
```

### 4. Setup ColdVox Action

**File**: `.github/actions/setup-coldvox/action.yml`

```yaml
name: Setup ColdVox Dependencies
description: Install system deps, libvosk, and Rust toolchain
inputs:
  skip-toolchain:
    description: Skip Rust toolchain setup (for jobs with custom toolchain)
    required: false
    default: 'false'
runs:
  using: composite
  steps:
    - name: Install system dependencies
      shell: bash
      run: |
        # Detect package manager and install appropriate packages
        if command -v dnf >/dev/null 2>&1; then
          # Fedora/RHEL/Nobara
          sudo dnf install -y --skip-unavailable \
            alsa-lib-devel \
            xdotool \
            libXtst-devel \
            wget \
            unzip \
            @development-tools
        elif command -v apt-get >/dev/null 2>&1; then
          # Ubuntu/Debian
          sudo apt-get update
          sudo apt-get install -y \
            libasound2-dev \
            libxdo-dev \
            libxtst-dev \
            wget \
            unzip \
            build-essential
        else
          echo "ERROR: Unsupported package manager" >&2
          exit 1
        fi

    - name: Validate libvosk installation
      shell: bash
      run: |
        echo "Validating pre-installed libvosk..."
        if [ ! -f "/usr/local/lib/libvosk.so" ]; then
          echo "ERROR: libvosk.so not found, run setup-permanent-libvosk.sh on runner"
          exit 1
        fi
        if [ ! -f "/usr/local/include/vosk_api.h" ]; then
          echo "ERROR: vosk_api.h not found, run setup-permanent-libvosk.sh on runner"
          exit 1
        fi
        # Ensure libvosk is in dynamic linker cache
        if ! ldconfig -p | grep -q vosk; then
          echo "WARNING: libvosk not in linker cache, refreshing..."
          sudo ldconfig
        fi
        echo "✅ libvosk available at /usr/local/lib/libvosk.so"
        echo "✅ libvosk cached in dynamic linker"

    - name: Setup Rust toolchain
      if: inputs.skip-toolchain != 'true'
      uses: dtolnay/rust-toolchain@stable
      with:
        components: rustfmt, clippy

    - name: Cache Cargo
      uses: Swatinem/rust-cache@v2
```

---

## Configuration Scripts

### Permanent libvosk Installation Script

**File**: `scripts/setup-permanent-libvosk.sh`

```bash
#!/bin/bash
# Permanent libvosk installation for self-hosted runner
# This should be run ONCE on the runner to eliminate per-job extraction

set -euo pipefail

echo "=== Setting up permanent libvosk installation ==="

VOSK_VER="0.3.45"
VENDOR_DIR="/home/coldaine/Projects/ColdVox/vendor/vosk"
CACHE_DIR="/home/coldaine/ActionRunnerCache"

# Ensure we have the vendor file
if [ ! -f "$VENDOR_DIR/vosk-linux-x86_64-${VOSK_VER}.zip" ]; then
    echo "ERROR: Vendor file not found: $VENDOR_DIR/vosk-linux-x86_64-${VOSK_VER}.zip"
    exit 1
fi

# Create working directory
mkdir -p "$CACHE_DIR/libvosk-setup"
cd "$CACHE_DIR/libvosk-setup"

# Extract if not already done
if [ ! -d "vosk-linux-x86_64-${VOSK_VER}" ]; then
    echo "Extracting libvosk..."
    unzip -q "$VENDOR_DIR/vosk-linux-x86_64-${VOSK_VER}.zip"
fi

# Install permanently
echo "Installing libvosk system-wide..."
sudo cp -v "vosk-linux-x86_64-${VOSK_VER}/libvosk.so" /usr/local/lib/
sudo cp -v "vosk-linux-x86_64-${VOSK_VER}/vosk_api.h" /usr/local/include/

# Update dynamic linker cache
echo "Updating dynamic linker cache..."
sudo ldconfig

# Verify installation
echo "Verifying installation..."
if ldconfig -p | grep -q vosk; then
    echo "✅ libvosk successfully installed and cached"
    ldconfig -p | grep vosk
else
    echo "❌ libvosk not found in linker cache"
    exit 1
fi

# Test linking
echo "Testing library linking..."
if ldd /usr/local/lib/libvosk.so >/dev/null 2>&1; then
    echo "✅ libvosk dependencies resolved"
else
    echo "❌ libvosk dependency issues"
    ldd /usr/local/lib/libvosk.so
    exit 1
fi

# Create permanent ldconfig configuration
echo "Creating permanent ldconfig entry..."
echo "/usr/local/lib" | sudo tee /etc/ld.so.conf.d/vosk.conf
sudo ldconfig

echo "✅ Permanent libvosk installation complete!"
echo ""
echo "🚀 Now workflows should use validation instead of extraction:"
echo "    - Remove zip extraction from setup-coldvox action"
echo "    - Replace with simple validation check"
echo "    - Expected time savings: 5-15 seconds per job"
```

### Performance Monitor Script

**File**: `scripts/performance_monitor.sh`

```bash
#!/bin/bash
# GitHub Actions Self-Hosted Runner Performance Monitor
# Monitors system resources during workflow execution

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
LOG_DIR="$PROJECT_ROOT/logs/performance"

# Create log directory
mkdir -p "$LOG_DIR"

# Configuration
SAMPLE_INTERVAL=5  # seconds
MAX_RUNTIME=3600   # 1 hour max

# Helper function to get system metrics
get_system_metrics() {
    local load_avg memory_usage disk_usage runner_cpu runner_mem

    # Fixed variable binding with proper defaults
    load_avg=$(cut -d' ' -f1 /proc/loadavg || echo "0.0")
    memory_usage=$(free -m | awk '/^Mem:/ {printf "%.1f", $3}' || echo "0.0")
    disk_usage=$(df /home/coldaine/actions-runner/_work 2>/dev/null | awk 'NR==2 {print $5}' | sed 's/%//' || echo "0")

    # Runner process metrics with error handling
    local runner_pid
    runner_pid=$(pgrep -f "Runner.Listener" || echo "")
    if [[ -n "$runner_pid" ]]; then
        local runner_stats
        runner_stats=$(ps -p "$runner_pid" -o %cpu,%mem --no-headers 2>/dev/null || echo "0.0 0.0")
        runner_cpu=$(echo "$runner_stats" | awk '{print $1}' || echo "0.0")
        runner_mem=$(echo "$runner_stats" | awk '{print $2}' || echo "0.0")
    else
        runner_cpu="0.0"
        runner_mem="0.0"
    fi

    echo "$load_avg,$memory_usage,$disk_usage,$runner_cpu,$runner_mem"
}

# Main monitoring function
monitor_performance() {
    local start_time=$(date +%s)
    local log_file="$LOG_DIR/performance_$(date +%Y%m%d_%H%M%S).log"

    echo "[$(date '+%Y-%m-%d %H:%M:%S')] Starting workflow performance monitoring..."
    echo "Monitor log: $log_file"
    echo "Sample interval: ${SAMPLE_INTERVAL}s, Max runtime: ${MAX_RUNTIME}s"

    # CSV header
    echo "timestamp,load_avg,memory_mb,disk_pct,runner_cpu,runner_mem" > "$log_file"

    while true; do
        local current_time=$(date +%s)
        local elapsed=$((current_time - start_time))

        # Check max runtime
        if [[ $elapsed -gt $MAX_RUNTIME ]]; then
            echo "Max runtime reached, stopping monitor"
            break
        fi

        # Get metrics and log
        local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
        local metrics
        metrics=$(get_system_metrics)
        echo "$timestamp,$metrics" >> "$log_file"

        sleep $SAMPLE_INTERVAL
    done
}

# Main script logic
case "${1:-}" in
    monitor)
        monitor_performance
        ;;
    health)
        # Simple health check
        load_avg=$(cut -d' ' -f1 /proc/loadavg)
        if (( $(echo "$load_avg > 10" | bc -l) )); then
            echo "HIGH LOAD: $load_avg"
            exit 1
        fi
        echo "System healthy, load: $load_avg"
        ;;
    *)
        echo "Usage: $0 {monitor|health}"
        exit 1
        ;;
esac
```

---

## Current Issues and Status

### Known Working Components
- ✅ Vosk model caching (8-11 seconds setup vs 3+ hour timeouts)
- ✅ System package installation with `--skip-unavailable` flags
- ✅ libvosk permanent installation and ldconfig configuration
- ✅ Runner labels and basic workflow execution

### Current Failure Points
- ❌ Vosk integration tests failing with `libvosk.so: cannot open shared object file`
- ❌ Some CI jobs queuing for extended periods (18+ minutes)
- ❌ Intermittent network timeouts during GitHub Action downloads

### Recent Changes (Commits)
1. **959a04a**: Implemented permanent libvosk installation
2. **4062a15**: Fixed package name from 'app' to 'coldvox-app' in workflows
3. **1f1af7f**: Added `--skip-unavailable` flags to dnf commands

### Planned Improvements (Phase 3)
1. System package pre-installation (eliminate 200-400MB downloads)
2. Enhanced Rust toolchain caching (eliminate 250-500MB downloads)
3. Parallel job execution configuration (3-4x throughput improvement)

---

**End of Document**
