# ColdVox Dependency Documentation

**Last Updated:** 2025-01-08  
**Rust Version:** 1.90.0 (should be documented in `rust-toolchain.toml` - see recommendations)

This document comprehensively reviews all dependencies in the ColdVox project, with special focus on pinned dependencies and their justifications.

---

## Table of Contents

1. [Pinned Dependencies](#pinned-dependencies)
2. [Rust Crate Dependencies](#rust-crate-dependencies)
3. [System Dependencies](#system-dependencies)
4. [GitHub Actions Dependencies](#github-actions-dependencies)
5. [Dependency Management Strategy](#dependency-management-strategy)
6. [Maintenance Recommendations](#maintenance-recommendations)

---

## Pinned Dependencies

### Git Dependencies (Critical: Pinned to Specific Commit)

#### 1. `voice_activity_detector`

**Location:** `crates/coldvox-vad-silero/Cargo.toml`

**Current Pin:**
```toml
voice_activity_detector = { 
    git = "https://github.com/nkeenan38/voice_activity_detector", 
    rev = "234b7484860125014f06ad85da842da81b02e51a", 
    optional = true 
}
```

**Version:** 0.2.1  
**Pinned Commit:** 234b7484860125014f06ad85da842da81b02e51a

**Justification:**
- **Reason for Git Dependency:** This crate is not published on crates.io, so we must use the git repository directly
- **Reason for Commit Pin:** Ensures reproducible builds and prevents unexpected breakage from upstream changes
- **Why This Specific Commit:** This is a known stable commit that works with our Silero V5 ONNX-based VAD implementation
- **Critical Dependency:** Core functionality - VAD (Voice Activity Detection) is central to ColdVox's real-time audio processing pipeline

**Risks:**
- ⚠️ Not actively maintained upstream (last commit check recommended)
- ⚠️ Pinning prevents automatic security updates
- ⚠️ ONNX runtime dependency chain (ort, ort-sys) may have compatibility issues with newer systems

**Review Status:** ⚠️ **NEEDS REVIEW**
- Last verified: Unknown
- Recommended action: 
  1. Check if upstream has newer commits with bug fixes or improvements
  2. Test if updating to latest commit breaks functionality
  3. Consider vendoring or forking if upstream is abandoned
  4. Document why this specific commit was chosen (if not already known)

**Dependencies of This Package:**
- `futures`
- `ndarray` (numerical array operations)
- `ort` (ONNX Runtime bindings)
- `ort-sys` (ONNX Runtime C++ bindings)

---

## Rust Crate Dependencies

### Core Dependencies (Used Across Multiple Crates)

#### Async Runtime & Concurrency

##### `tokio` (version = "1" or "1.35")
- **Purpose:** Async runtime for all async operations
- **Version Strategy:** Major version pin (`"1"`) allows minor/patch updates
- **Usage:** Foundation for all async operations, audio capture threading, STT processing
- **Justification:** Industry-standard async runtime, stable API, good ecosystem support
- **Review:** ✅ **APPROVED** - Using latest 1.x is recommended, semver allows safe updates

##### `parking_lot` (version = "0.12")
- **Purpose:** Fast, lightweight synchronization primitives (Mutex, RwLock)
- **Usage:** Shared state management across audio pipeline, VAD, and STT processors
- **Justification:** Better performance than std::sync primitives, no poisoning
- **Review:** ✅ **APPROVED** - Stable, well-maintained

##### `crossbeam-channel` (version = "0.5")
- **Purpose:** Multi-producer multi-consumer channels
- **Usage:** Inter-thread communication in audio pipeline
- **Justification:** More flexible than tokio channels for certain use cases
- **Review:** ✅ **APPROVED** - Stable API

##### `async-trait` (version = "0.1")
- **Purpose:** Enables async functions in trait definitions
- **Usage:** STT plugin system, VAD engine trait
- **Justification:** Required for async trait methods until native support in Rust
- **Review:** ✅ **APPROVED** - Will eventually be replaced by native async traits

#### Serialization & Configuration

##### `serde` (version = "1.0")
- **Purpose:** Serialization/deserialization framework
- **Usage:** Configuration, plugin definitions, telemetry data structures
- **Justification:** De facto standard for Rust serialization
- **Review:** ✅ **APPROVED** - Stable 1.x API

##### `serde_json` (version = "1.0")
- **Purpose:** JSON serialization via serde
- **Usage:** Plugin configuration files, structured logging output
- **Review:** ✅ **APPROVED**

##### `toml` (version = "0.8")
- **Purpose:** TOML file parsing
- **Usage:** Main configuration files (`config/default.toml`)
- **Review:** ✅ **APPROVED** - Latest stable version

##### `config` (version = "0.14")
- **Purpose:** Layered configuration management
- **Usage:** Centralized configuration system with environment variable overrides
- **Justification:** Supports multiple config sources, environment variables, type-safe defaults
- **Review:** ✅ **APPROVED** - Recently added (PR #123), stable API

#### Error Handling & Logging

##### `thiserror` (version = "2.0")
- **Purpose:** Derive macro for custom error types
- **Usage:** All error types across the workspace
- **Justification:** Best practice for library error types, generates good error messages
- **Review:** ✅ **APPROVED** - Version 2.0 is latest major release

##### `anyhow` (version = "1.0")
- **Purpose:** Flexible error handling for application code
- **Usage:** Application-level error propagation, examples
- **Justification:** Good for applications where exact error types don't matter
- **Review:** ✅ **APPROVED** - Stable API

##### `tracing` (version = "0.1")
- **Purpose:** Structured logging and diagnostics
- **Usage:** All logging throughout the application
- **Justification:** More powerful than `log` crate, async-aware, supports spans
- **Review:** ✅ **APPROVED** - De facto standard for structured logging

##### `tracing-subscriber` (version = "0.3")
- **Purpose:** Utilities for consuming tracing data
- **Usage:** Log formatting, filtering, output to file/stderr
- **Review:** ✅ **APPROVED**

##### `tracing-appender` (version = "0.2")
- **Purpose:** File rotation for tracing output
- **Usage:** Daily-rotated log files in `logs/coldvox.log`
- **Review:** ✅ **APPROVED**

#### CLI & TUI

##### `clap` (version = "4.0")
- **Purpose:** Command-line argument parsing
- **Usage:** Main application CLI, mic_probe, tui_dashboard
- **Justification:** Feature-rich, derive macro API, good error messages
- **Review:** ✅ **APPROVED** - Version 4.x is current, stable API

##### `ratatui` (version = "0.28")
- **Purpose:** Terminal UI framework
- **Usage:** TUI dashboard (`tui_dashboard` binary)
- **Justification:** Modern, actively maintained fork of `tui-rs`
- **Review:** ✅ **APPROVED** - Latest stable version

##### `crossterm` (version = "0.28")
- **Purpose:** Cross-platform terminal manipulation
- **Usage:** Terminal input/output for TUI dashboard
- **Justification:** Required by ratatui, cross-platform
- **Review:** ✅ **APPROVED**

### Audio Processing Dependencies

#### `cpal` (version = "0.16.0")
- **Purpose:** Cross-platform audio I/O
- **Usage:** Core audio capture from microphone devices
- **Justification:** Cross-platform, low-latency, supports various backends (ALSA, PulseAudio, etc.)
- **Pinning:** Specific patch version (0.16.0)
- **Review:** ⚠️ **NEEDS UPDATE CHECK** - Verify if 0.16.x has important bug fixes
- **System Dependencies:** ALSA libraries on Linux

#### `rtrb` (version = "0.3")
- **Purpose:** Realtime-safe ring buffer (SPSC)
- **Usage:** Lock-free audio sample buffer between capture thread and processing
- **Justification:** Lock-free, wait-free operations suitable for real-time audio
- **Review:** ✅ **APPROVED** - Specialized for audio use case

#### `rubato` (version = "0.16")
- **Purpose:** Audio resampling
- **Usage:** Converts audio from device sample rate to 16 kHz (required by VAD/STT)
- **Justification:** High-quality resampling, configurable quality levels
- **Review:** ✅ **APPROVED** - Version 0.16 is recent

#### `dasp` (version = "0.11")
- **Purpose:** Digital audio signal processing primitives
- **Usage:** Audio frame processing, sample format conversion
- **Justification:** Comprehensive DSP toolkit for audio manipulation
- **Review:** ✅ **APPROVED**

#### `hound` (version = "3.5")
- **Purpose:** WAV file reading/writing
- **Usage:** Recording audio samples, loading test fixtures, STT testing
- **Review:** ✅ **APPROVED** - Simple, reliable WAV implementation

### STT (Speech-to-Text) Dependencies

#### `vosk` (version = "0.3")
- **Purpose:** Vosk speech recognition bindings
- **Usage:** Offline speech-to-text transcription
- **Justification:** Offline, no API costs, decent accuracy for small models
- **System Dependencies:** libvosk.so (handled by build.rs with vendored fallback)
- **Review:** ⚠️ **MONITOR** - Check for 0.4.x updates
- **Build-time:** Requires libvosk installed or vendored in `vendor/vosk/lib/`

#### `vosk-sys` (transitive)
- **Purpose:** Low-level FFI bindings to libvosk
- **Note:** Transitive dependency of `vosk` crate

#### `zip` (version = "0.6")
- **Purpose:** ZIP archive handling
- **Usage:** Model extraction and management in STT subsystem
- **Review:** ✅ **APPROVED**

#### `uuid` (version = "1.0")
- **Purpose:** UUID generation
- **Usage:** STT session/request tracking
- **Review:** ✅ **APPROVED**

### Text Injection Dependencies

#### `atspi` (version = "0.28", optional)
- **Purpose:** AT-SPI (Assistive Technology Service Provider Interface) accessibility API
- **Usage:** Text injection via Linux accessibility subsystem
- **Justification:** Most reliable method for text injection on modern Linux desktops
- **Platform:** Linux only
- **System Dependencies:** AT-SPI 2.0 development libraries
- **Review:** ✅ **APPROVED** - Latest stable version

#### `wl-clipboard-rs` (version = "0.9", optional)
- **Purpose:** Wayland clipboard manipulation
- **Usage:** Clipboard-based text injection on Wayland
- **Platform:** Linux Wayland sessions
- **Review:** ⚠️ **CHECK FOR UPDATES** - Verify if 1.0 exists

#### `enigo` (version = "0.6", optional)
- **Purpose:** Cross-platform input simulation
- **Usage:** Fallback text injection method, Windows/macOS primary method
- **Justification:** Cross-platform API for keyboard simulation
- **Review:** ⚠️ **OUTDATED** - Version 0.6 is old, check for breaking changes in newer versions
- **Risk:** May have security or compatibility issues on newer systems

#### Platform-Specific (Build-Time Detection)

**`kdotool`** (feature flag, no crate dependency)
- **Purpose:** X11 text injection via external `kdotool` binary
- **Usage:** KDE Plasma desktop environment
- **Detection:** Build script checks for KDE environment variables

**`ydotool`** (feature flag, no crate dependency)
- **Purpose:** Wayland text injection via external `ydotool` binary
- **Usage:** Wayland sessions where AT-SPI is unavailable
- **Detection:** Runtime binary availability check

### D-Bus & IPC Dependencies

#### `zbus` (version = "5.11.0")
- **Purpose:** D-Bus client/server implementation
- **Usage:** KDE KGlobalAccel integration for global hotkeys
- **Justification:** Pure Rust D-Bus implementation, async-native
- **Pinning:** Specific patch version (5.11.0)
- **Review:** ⚠️ **CONSIDER RELAXING PIN** - Check if 5.11.x patch updates are compatible
- **Note:** Version 5.x is latest major version

### GUI Dependencies (Optional, Future Use)

#### `cxx` (version = "1")
- **Purpose:** C++ interop for Qt bindings
- **Usage:** Foundation for GUI crate (currently placeholder)
- **Review:** ✅ **APPROVED** - Required for CXX-Qt

#### `cxx-qt` (version = "0.7", optional)
#### `cxx-qt-lib` (version = "0.7", optional)
#### `cxx-qt-build` (version = "0.7", build dependency)
- **Purpose:** Qt 6 bindings via CXX bridge
- **Usage:** Future GUI frontend (gated behind `qt-ui` feature)
- **Justification:** Modern approach to Qt bindings for Rust
- **Status:** Placeholder - not actively used yet
- **Review:** ⚠️ **FUTURE** - Will need updates when GUI development starts
- **Note:** No git dependency needed (unlike older plan document suggested), using crates.io version

### Utility Dependencies

#### `chrono` (version = "0.4")
- **Purpose:** Date and time handling
- **Usage:** Timestamps, log rotation, telemetry
- **Review:** ✅ **APPROVED** - Stable 0.4.x series

#### `dirs` (version = "5.0")
- **Purpose:** Standard directory locations (config, data, cache)
- **Usage:** Finding user config directories for STT models
- **Review:** ✅ **APPROVED**

#### `once_cell` (version = "1.19")
- **Purpose:** Lazy static initialization
- **Usage:** Global state initialization
- **Review:** ✅ **APPROVED** - Will eventually be superseded by std::cell::LazyCell

#### `fastrand` (version = "2.0")
- **Purpose:** Fast random number generation
- **Usage:** Non-cryptographic randomness (IDs, jitter, etc.)
- **Review:** ✅ **APPROVED**

#### `csv` (version = "1.3")
- **Purpose:** CSV reading/writing
- **Usage:** Data export, telemetry logging
- **Review:** ✅ **APPROVED**

#### `futures` (version = "0.3")
- **Purpose:** Async primitives and utilities
- **Usage:** Async stream processing, future combinators
- **Review:** ✅ **APPROVED**

#### `regex` (version = "1.10", optional)
- **Purpose:** Regular expressions
- **Usage:** Text injection pattern matching
- **Review:** ✅ **APPROVED**

### Development Dependencies

#### `tempfile` (version = "3.22")
- **Purpose:** Temporary file/directory creation
- **Usage:** Test fixtures, temporary storage in tests
- **Review:** ✅ **APPROVED**

#### `mockall` (version = "0.12")
- **Purpose:** Mock object generation for testing
- **Usage:** Unit tests for trait-based abstractions
- **Review:** ✅ **APPROVED**

#### `tokio-test` (version = "0.4")
- **Purpose:** Testing utilities for async code
- **Usage:** Async test helpers
- **Review:** ✅ **APPROVED**

#### `proptest` (version = "1.4")
- **Purpose:** Property-based testing
- **Usage:** Fuzz testing audio pipeline edge cases
- **Review:** ✅ **APPROVED**

#### `rand` (version = "0.8")
- **Purpose:** Random number generation (test fixtures)
- **Usage:** Generating test data
- **Review:** ✅ **APPROVED**

#### `ctrlc` (version = "3.5")
- **Purpose:** Cross-platform Ctrl+C handler
- **Usage:** Graceful shutdown in examples and tests
- **Review:** ✅ **APPROVED**

#### `serial_test` (version = "3.0")
- **Purpose:** Serialize test execution
- **Usage:** Hardware integration tests that can't run in parallel
- **Review:** ✅ **APPROVED**

#### `arboard` (version = "3.2")
- **Purpose:** Clipboard access (dev dependency)
- **Usage:** Testing clipboard-based text injection
- **Review:** ✅ **APPROVED**

### Build Dependencies

#### `cc` (version = "1.2")
- **Purpose:** Invoke C compiler
- **Usage:** Building GTK test applications in text-injection crate
- **Review:** ✅ **APPROVED**

#### `pkg-config` (version = "0.3")
- **Purpose:** Query system package information
- **Usage:** Finding GTK+ 3.0, AT-SPI 2.0 headers
- **Review:** ✅ **APPROVED**

---

## System Dependencies

### Build-Time Requirements

#### Essential
- **pkg-config** - Required to locate system libraries
- **gcc/g++** - C/C++ compiler for native dependencies
- **make** - Build automation (for some transitive dependencies)

#### Optional (Feature-Dependent)
- **GTK+ 3.0 development libraries** (`gtk+-3.0`)
  - Only needed with `real-injection-tests` feature
  - Used to build GTK test applications
  
- **AT-SPI 2.0 development libraries** (`at-spi-2.0`)
  - Only needed with `atspi` text injection feature
  - Provides accessibility API headers

### Runtime Requirements

#### Essential
- **ALSA libraries** (Linux)
  - Audio capture via CPAL
  - Usually installed by default on Linux systems

#### Optional (Feature-Dependent)
- **libvosk.so**
  - Required only with `vosk` feature (enabled by default)
  - Can be system-installed or vendored in `vendor/vosk/lib/`
  - Build script (`crates/coldvox-stt-vosk/build.rs`) handles location detection

- **PulseAudio** (Linux)
  - Recommended audio server on Linux
  - Used by integration tests for audio routing

- **D-Bus**
  - Required for KDE KGlobalAccel integration
  - Standard on all modern Linux desktop environments

- **xdotool** (Linux X11)
  - Optional external binary for X11 text injection
  - Runtime availability checked, not required for build

- **ydotool** (Linux Wayland)
  - Optional external binary for Wayland text injection
  - Runtime availability checked, not required for build

- **wl-paste / xclip**
  - Optional clipboard utilities
  - Used by clipboard-based text injection methods

- **Xvfb, openbox** (Testing only)
  - Headless X11 server for CI text injection tests
  - Not needed for normal operation

---

## GitHub Actions Dependencies

All GitHub Actions are pinned to specific commit SHAs for security and reproducibility.

### Checkout Actions

#### `actions/checkout@v5` (Multiple Pins)
- **Latest Pin:** `08c6903cd8c0fde910a37f88322edcfb5dd907a8` (v5.0.0)
- **Older Pin:** `11bd71901bbe5b1630ceea73d27597364c9af683` (v4.2.2)
- **Justification:** Repository checkout at specific commit ensures workflow doesn't break from action updates
- **Usage:** Every workflow needs to check out the code
- **Review:** ⚠️ **INCONSISTENT** - Some workflows use v4, others use v5. Should standardize on v5.

### Rust Toolchain Actions

#### `dtolnay/rust-toolchain` (Multiple Pins)
- **Stable Pin:** `dtolnay/rust-toolchain@stable` (tracks latest stable)
- **Commit Pin:** `e97e2d8cc328f1b50210efc529dca0028893a2d9` (v1)
- **Justification:** Installs Rust toolchain, pinning prevents supply chain attacks
- **Usage:** All CI jobs need Rust installed
- **Review:** ⚠️ **INCONSISTENT** - Mix of `@stable` and commit pins. Recommend commit pins everywhere.

#### `actions-rust-lang/setup-rust-toolchain@v1`
- **Usage:** Alternative Rust toolchain action (used in some workflows)
- **Review:** ⚠️ **INCONSISTENT** - Should standardize on one action (`dtolnay/rust-toolchain` recommended)

### Caching Actions

#### `Swatinem/rust-cache@v2` (Multiple Pins)
- **Latest Pin:** `98c8021b550208e191a6a3145459bfc9fb29c4c0` (v2.8.0)
- **Older Pin:** `@v2` (tag, not commit)
- **Justification:** Caches Cargo build artifacts, speeds up CI significantly
- **Usage:** Most build jobs
- **Review:** ⚠️ **INCONSISTENT** - Some jobs use commit pin, others use tag. Standardize on commit pins.

### Artifact Actions

#### `actions/upload-artifact@v4`
- **Pin:** Tag only (no commit SHA)
- **Justification:** Upload build artifacts for release or debugging
- **Review:** ⚠️ **SHOULD PIN TO COMMIT** - For security, pin to specific commit

---

## Dependency Management Strategy

### Current Approach

1. **Rust Crates:**
   - Most dependencies use semantic versioning ranges (e.g., `"1.0"`, `"0.3"`)
   - Some have specific patch pins (e.g., `zbus = "5.11.0"`, `cpal = "0.16.0"`)
   - One critical git dependency pinned to commit (voice_activity_detector)

2. **GitHub Actions:**
   - Mix of tag-based and commit-SHA pins
   - Inconsistent across workflows

3. **System Dependencies:**
   - Detected at build time via pkg-config
   - Optional features gracefully degrade if not available
   - No version constraints specified

### Problems Identified

1. **No Rust Toolchain Pin:**
   - No `rust-toolchain.toml` file in repository
   - CI docs claim MSRV 1.75, but builds use 1.90
   - Different developers may use different Rust versions

2. **Git Dependency Management:**
   - `voice_activity_detector` pinned but no update process documented
   - No verification of upstream commit status
   - Risk of using outdated or insecure code

3. **Inconsistent GitHub Actions Pins:**
   - Some workflows use commit SHAs, others use tags
   - Mix of v4 and v5 for `actions/checkout`
   - Using two different Rust toolchain actions

4. **Outdated Dependencies:**
   - `enigo = "0.6"` - Potentially outdated, security concern
   - `wl-clipboard-rs = "0.9"` - Check for 1.0 release
   - Several specific patch pins without documented reason

5. **Patch Version Pins Without Justification:**
   - `zbus = "5.11.0"` - Why not `"5.11"` to allow patch updates?
   - `cpal = "0.16.0"` - Why not `"0.16"` to allow patch updates?

---

## Maintenance Recommendations

### Immediate Actions (Priority: HIGH)

#### 1. Create Rust Toolchain Pin

**Action:** Create `rust-toolchain.toml` at repository root:

```toml
[toolchain]
channel = "1.90.0"
components = ["rustfmt", "clippy"]
profile = "minimal"
```

**Justification:**
- Ensures all developers and CI use the same Rust version
- Prevents "works on my machine" issues
- Current CI uses 1.90.0, should make this explicit
- MSRV documentation claims 1.75, but this is outdated

**Alternative:** If you want to track stable:
```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy"]
profile = "minimal"
```

**Update Required:**
- Remove MSRV 1.75 references from documentation
- Update `.github/workflows/ci.yml` to remove MSRV matrix if not needed

#### 2. Review and Update `voice_activity_detector` Pin

**Actions:**
1. Check upstream repository for newer commits:
   ```bash
   git ls-remote https://github.com/nkeenan38/voice_activity_detector HEAD
   ```

2. If newer commits exist:
   - Review changelog/commits
   - Test with newer commit
   - Update pin if stable
   
3. If upstream is abandoned:
   - Consider forking to Coldaine organization
   - Or vendor the dependency entirely
   - Or investigate alternative VAD implementations

**Current Commit:** 234b7484860125014f06ad85da842da81b02e51a  
**Last Verified:** Unknown - ADD DATE AFTER VERIFICATION

#### 3. Standardize GitHub Actions Pins

**Action:** Update all workflows to use consistent, commit-pinned actions:

```yaml
# Recommended standard
- uses: actions/checkout@08c6903cd8c0fde910a37f88322edcfb5dd907a8 # v5.0.0
- uses: dtolnay/rust-toolchain@e97e2d8cc328f1b50210efc529dca0028893a2d9 # stable
- uses: Swatinem/rust-cache@98c8021b550208e191a6a3145459bfc9fb29c4c0 # v2.8.0
```

**Files to Update:**
- `.github/workflows/ci.yml`
- `.github/workflows/vosk-integration.yml`
- `.github/workflows/release.yml`
- `.github/workflows/ci-minimal.yml`
- Any other workflow files

**Remove:**
- `actions-rust-lang/setup-rust-toolchain@v1` - replace with `dtolnay/rust-toolchain`

#### 4. Document Patch Version Pins

**Action:** For each specific patch version pin, add inline comment explaining why:

```toml
# Example
zbus = { version = "5.11.0" }  # Pinned: Issue #XXX requires this exact version
# OR
zbus = { version = "5.11" }  # Changed from 5.11.0 - allow patch updates
```

**Crates to Document:**
- `zbus = "5.11.0"`
- `cpal = "0.16.0"`

Or relax pins to allow patch updates if no specific reason exists.

### Medium Priority Actions

#### 5. Review and Update Outdated Dependencies

**`enigo = "0.6"`**
- **Current:** 0.6 (released 2+ years ago)
- **Action:** Check latest version, review breaking changes
- **Risk Level:** HIGH - Text injection is security-sensitive
- **Test Carefully:** Input simulation API may have changed

**`wl-clipboard-rs = "0.9"`**
- **Current:** 0.9
- **Action:** Check if 1.0 or 0.10+ exists
- **Risk Level:** LOW - Wrapper around external tool

**`atspi = "0.28"`**
- **Action:** Check for 0.29+ updates
- **Risk Level:** MEDIUM - Core text injection method

#### 6. Add Dependency Update Automation

**Options:**

**A. Dependabot (GitHub Native)**
```yaml
# .github/dependabot.yml
version: 2
updates:
  - package-ecosystem: "cargo"
    directory: "/"
    schedule:
      interval: "weekly"
    open-pull-requests-limit: 10
    
  - package-ecosystem: "github-actions"
    directory: "/"
    schedule:
      interval: "weekly"
```

**B. Renovate Bot**
- More configurable than Dependabot
- Can auto-merge minor/patch updates
- Better grouping of related updates

**Recommendation:** Start with Dependabot, evaluate Renovate if more control needed.

#### 7. Set Up Dependency Audit

**Add `cargo-audit` to CI:**

```yaml
# In .github/workflows/ci.yml or separate workflow
- name: Security Audit
  run: |
    cargo install cargo-audit
    cargo audit
```

**Purpose:** Detect known vulnerabilities in dependencies

#### 8. Document System Dependency Versions

**Action:** Create matrix of tested system library versions:

| Library | Minimum Version | Tested Versions | Notes |
|---------|----------------|-----------------|-------|
| ALSA | 1.0.x | 1.2.8 | Standard in most distros |
| libvosk | 0.3.x | 0.3.45 | Vendored in CI |
| GTK+ 3.0 | 3.0 | 3.24.x | Only for test builds |
| AT-SPI 2.0 | 2.0 | 2.50.x | Linux accessibility |

### Low Priority / Future Improvements

#### 9. Consider Dependency Alternatives

**Evaluate These in Future:**

- **`enigo` alternatives:** 
  - `rdev` - More actively maintained
  - `autopilot-rs` - Cross-platform automation
  - Custom implementation per platform

- **`voice_activity_detector` alternatives:**
  - WebRTC VAD (simpler, no ONNX)
  - Silero-specific Rust implementations
  - Fork and maintain ourselves

#### 10. Vendoring Strategy for Critical Dependencies

**Consider vendoring:**
- `voice_activity_detector` (already a git dep)
- `libvosk` (already partially vendored)
- Any other critical dependencies without crates.io releases

**Advantages:**
- Full control over updates
- No risk of upstream disappearing
- Can apply patches immediately

**Disadvantages:**
- More maintenance burden
- Need to track upstream for security updates

#### 11. Feature Flag Audit

**Action:** Review all optional dependencies:
- Are they actually optional? (some may be required by default features)
- Document which features enable which dependencies
- Consider splitting into more granular features

#### 12. Workspace Dependency Deduplication

**Action:** Move common dependencies to workspace `[workspace.dependencies]`:

```toml
# In root Cargo.toml
[workspace.dependencies]
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
tracing = "0.1"
# etc.

# In crate Cargo.toml
[dependencies]
tokio = { workspace = true }
serde = { workspace = true }
```

**Benefits:**
- Single source of truth for versions
- Easier to update across workspace
- Prevents version conflicts

---

## Update Checklist

Use this checklist when updating dependencies:

### Before Update
- [ ] Check changelog for breaking changes
- [ ] Review security advisories
- [ ] Check MSRV requirements of new version
- [ ] Identify if update is major/minor/patch

### Testing Update
- [ ] Update `Cargo.toml`
- [ ] Run `cargo update -p <package-name>`
- [ ] Run `cargo check --all-features`
- [ ] Run `cargo test --all-features`
- [ ] Run `cargo clippy --all-features`
- [ ] Test hardware-dependent features manually (audio, text injection)
- [ ] Run examples to verify no regressions

### After Update
- [ ] Update this documentation if pinning rationale changes
- [ ] Update `Cargo.lock` (committed to repo)
- [ ] Note version change in CHANGELOG.md
- [ ] Create PR with test results

---

## Dependency Review Schedule

**Quarterly Review:** (Every 3 months)
- Review all dependencies for updates
- Check `voice_activity_detector` upstream for new commits
- Run `cargo audit` for security advisories
- Update GitHub Actions to latest commits
- Review this documentation for accuracy

**Annual Review:** (Once per year)
- Major version updates evaluation
- Alternative dependency evaluation
- Vendoring strategy review
- MSRV policy review

**As-Needed:**
- Security advisory responses (immediate)
- Critical bug fixes (within 1 week)
- Breaking changes from upstream (coordinate with releases)

---

## Conclusion

ColdVox has a relatively clean dependency footprint with mostly sensible choices. The main areas needing attention are:

1. **Critical:** Establish Rust toolchain pinning
2. **Critical:** Review and document git dependency update policy
3. **High:** Standardize GitHub Actions pinning
4. **High:** Update potentially outdated dependencies (`enigo`, etc.)
5. **Medium:** Set up automated dependency monitoring

Most dependencies use appropriate version constraints (semantic versioning with reasonable ranges). The workspace structure is clean with good separation of concerns.

**Total Dependency Count:** ~537 packages (including transitive dependencies)  
**Direct Dependencies:** ~50 packages across all workspace crates  
**Git Dependencies:** 1 (voice_activity_detector)  
**Pinned to Exact Patch Version:** 2-3 (zbus, cpal, and dependency-specific pins)

---

**Maintenance Contact:** ColdVox Contributors  
**Last Audit:** 2025-01-08  
**Next Scheduled Review:** 2025-04-08 (3 months)
