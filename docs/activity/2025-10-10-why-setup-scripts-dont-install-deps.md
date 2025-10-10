# Why ColdVox Setup Scripts Don't Auto-Install System Dependencies

**Date**: October 10, 2025  
**Context**: Response to CI failures due to missing system dependencies  
**Related**: `docs/activity/2025-10-10-ci-debug-refactor-text-injection.md`

---

## TL;DR

ColdVox's CI setup scripts **deliberately do not install system dependencies**. Instead, they **verify** that dependencies are already present. This is an intentional architectural decision for self-hosted runners, not an oversight.

**Philosophy**: "Pre-provision once, verify always"

---

## The Design Pattern

### What the Scripts Do

#### ❌ What They DON'T Do
```bash
# Scripts do NOT do this:
sudo dnf install -y pulseaudio at-spi2-core-devel gtk3-devel
sudo apt install -y pulseaudio libatspi2.0-dev libgtk-3-dev
```

#### ✅ What They DO Instead
```bash
# Scripts DO this (.github/actions/setup-coldvox/action.yml):
required_commands="xdotool wget unzip gcc g++ make Xvfb openbox dbus-launch wl-paste xclip ydotool xprop wmctrl pkg-config pulseaudio"

for cmd in $required_commands; do
  if ! command -v "$cmd" &> /dev/null; then
    echo "::error::Required command '$cmd' not found on runner. Please provision the runner with this dependency."
    failed=1
  fi
done

required_pkgs="alsa gtk+-3.0 at-spi-2.0 xtst"
for pkg in $required_pkgs; do
    if ! pkg-config --exists "$pkg"; then
        echo "::error::Required library '$pkg' not found by pkg-config. Please install the corresponding -devel package on the runner."
        failed=1
    fi
done

if [[ $failed -ne 0 ]]; then
  echo "::error::One or more system dependencies are missing. Please provision the runner correctly."
  exit 1
fi
```

**Key Comment in Code**:
```yaml
# Line 18 in .github/actions/setup-coldvox/action.yml:
# This replaces the sudo dnf/apt install commands
```

This comment explicitly acknowledges that installation commands were **removed and replaced** with verification checks.

---

## Why This Design?

### 1. **Self-Hosted Runner Context**

ColdVox uses **self-hosted runners**, not GitHub's cloud runners:

```yaml
# All workflows use this:
runs-on: [self-hosted, Linux, X64, fedora, nobara]
```

**Implications**:
- The runner is a persistent, known machine (`laptop-extra`)
- It's a physical HP EliteBook with specific hardware
- It's running Nobara Linux 42 (Fedora-based) with KDE Plasma
- The environment is **controlled** and **stable**

**Philosophy**: Since you control the hardware, you can pre-provision it once rather than installing dependencies on every CI run.

### 2. **Performance & Speed**

Installing packages takes time:
```bash
# Hypothetical per-run install:
sudo dnf install -y pulseaudio openbox at-spi2-core-devel gtk3-devel ... (10+ packages)
# Result: 30-60 seconds per CI run, every run

# Actual verify-only approach:
command -v pulseaudio && pkg-config --exists at-spi-2.0
# Result: <1 second, instant feedback
```

**Benefit**: Faster CI runs (seconds vs minutes for setup)

### 3. **Idempotency & Predictability**

Pre-provisioned runners guarantee:
- ✅ Same environment every time
- ✅ No network failures during package downloads
- ✅ No unexpected package version changes mid-CI
- ✅ No DNF/APT repository unavailability issues
- ✅ Deterministic test environment

Dynamic installation introduces:
- ❌ Network dependency (mirrors can be slow/down)
- ❌ Version drift (latest package != tested package)
- ❌ Race conditions (concurrent jobs installing same packages)
- ❌ Failure modes unrelated to code quality

### 4. **Security & Control**

Self-hosted runners with pre-provisioning:
- ✅ Explicit approval of what's installed
- ✅ Audit trail of system changes
- ✅ No surprise `sudo` commands in CI logs
- ✅ Controlled dependency versions
- ✅ No accidental system-wide changes

Dynamic installation risks:
- ❌ CI jobs have `sudo` access
- ❌ Malicious PRs could execute arbitrary root commands
- ❌ Dependency confusion attacks
- ❌ Accidental system damage from bad scripts

### 5. **Clear Separation of Concerns**

```
┌─────────────────────────────────────────────────────┐
│  Infrastructure Layer (Manual, One-Time)            │
│  • Install OS dependencies                          │
│  • Configure runner environment                     │
│  • Set up system services                           │
│  • Grant permissions                                │
│  Documents: docs/self-hosted-runner-complete-setup.md│
│             docs/tasks/ci-runner-readiness-proposal.md│
└─────────────────────────────────────────────────────┘
                         ▲
                         │ Pre-provisioning
                         │
┌─────────────────────────────────────────────────────┐
│  Application Layer (Automated, Every CI Run)        │
│  • Verify dependencies present                      │
│  • Setup Vosk model & library                       │
│  • Run Rust builds                                  │
│  • Execute tests                                    │
│  Scripts: .github/actions/setup-coldvox/            │
│           scripts/ci/setup-vosk-cache.sh            │
└─────────────────────────────────────────────────────┘
```

**Principle**: Infrastructure is provisioned by humans once. CI scripts operate within that provisioned environment.

---

## Historical Context

### Evidence of Intentional Design

1. **Explicit Comment Acknowledging Removal**:
   ```yaml
   # .github/actions/setup-coldvox/action.yml:18
   # This replaces the sudo dnf/apt install commands
   ```
   Someone deliberately removed installation commands and documented why.

2. **Runner Diagnostic Workflow** (`.github/workflows/runner-diagnostic.yml`):
   ```yaml
   - name: 3. Test Package Manager and Sudo Permissions
     run: |
       echo "--- Checking sudo capabilities ---"
       if sudo -n true 2>/dev/null; then
         echo "Sudo access without password confirmed."
         echo "--- Attempting to install 'xdotool' (as a test) ---"
         sudo dnf install -y --skip-unavailable xdotool || echo "xdotool install failed"
   ```
   This workflow **can** install packages (for testing), but **normal CI workflows don't**.

3. **Comprehensive Runner Setup Documentation**:
   - `docs/self-hosted-runner-complete-setup.md` (984 lines!)
   - `docs/tasks/ci-runner-readiness-proposal.md`
   - `docs/research/self-hosted-runner-current-status.md`
   
   These documents detail **exactly how to provision the runner** before CI runs.

4. **User-Facing Setup Scripts** (`scripts/setup_text_injection.sh`):
   ```bash
   # This script DOES use sudo for initial system setup
   PKG_INSTALL="sudo dnf install -y"
   $PKG_INSTALL wl-clipboard ydotool
   ```
   Scripts for **end-users** install dependencies. Scripts for **CI** verify them.

---

## The Gap That Caused Today's Failures

### What Happened

The runner was provisioned at some point (Sept 2025) with most dependencies. But over time:

1. **New dependencies were added** to the verification script:
   - `pulseaudio` (for audio tests)
   - `at-spi-2.0` (for accessibility/text injection features)

2. **Runner wasn't updated** to include these new requirements

3. **Verification correctly failed**, alerting us to the gap

### This is Actually Good Design!

The verification script **did its job**:
- ❌ It didn't silently fail or produce flaky tests
- ❌ It didn't try to install and potentially break things
- ✅ It **loudly failed** with clear error messages
- ✅ It told us **exactly what's missing**
- ✅ It prevented tests from running in a broken environment

---

## How to Fix It (The Intended Way)

### One-Time Runner Provisioning

As documented in `docs/tasks/ci-runner-readiness-proposal.md`:

```bash
# SSH into the runner (laptop-extra)
ssh coldaine@192.168.1.66

# Install missing dependencies
sudo dnf install -y pulseaudio at-spi2-core-devel

# Optional: Install recommended related packages
sudo dnf install -y pipewire-pulseaudio openbox gtk3-devel libXtst-devel alsa-lib-devel

# Verify
for c in pulseaudio openbox; do command -v $c || echo MISSING:$c; done
for p in at-spi-2.0 gtk+-3.0 xtst alsa; do pkg-config --exists $p || echo MISSING-PKG:$p; done

# Done! All future CI runs will pass the verification step
```

**Key Point**: This is done **once** on the runner machine, not in CI scripts.

---

## When Would Dynamic Installation Make Sense?

### Scenarios Where Auto-Install is Appropriate

1. **Cloud/Ephemeral Runners** (GitHub-hosted, AWS EC2, etc.):
   ```yaml
   runs-on: ubuntu-latest  # Fresh VM every time
   steps:
     - run: sudo apt-get update && sudo apt-get install -y ...
   ```
   No persistent state, so must install each run.

2. **Multi-Distribution Support**:
   ```yaml
   strategy:
     matrix:
       os: [ubuntu-20.04, ubuntu-22.04, fedora-40, arch]
   ```
   Can't pre-provision all combinations, so auto-detect and install.

3. **Optional Dependencies**:
   ```bash
   # Install if not present, but don't fail if unavailable
   command -v clang || sudo dnf install -y clang || echo "clang optional, skipping"
   ```

4. **Development Containers** (Docker, Podman):
   ```dockerfile
   FROM fedora:42
   RUN dnf install -y gcc make rust cargo ...
   ```
   Containers are ephemeral and reproducible, so installation is baked into the image.

### Why ColdVox Doesn't Fit These Scenarios

- ❌ Not using cloud runners (using self-hosted)
- ❌ Not supporting multiple OS matrix (only Nobara/Fedora)
- ❌ Dependencies are **required**, not optional
- ❌ Not using containerized CI (bare metal runner)

---

## Comparison: Other Projects

### Projects with Dynamic Installation
```yaml
# Typical open-source project on GitHub Actions:
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: sudo apt-get install -y libfoo-dev libbar-dev
      - run: cargo build && cargo test
```

**Works for them because**: Fresh VM every time, no persistent state

### Projects with Pre-Provisioning
```yaml
# Enterprise projects with self-hosted runners:
jobs:
  test:
    runs-on: [self-hosted, linux, production]
    steps:
      - uses: actions/checkout@v4
      - run: |
          # Verify pre-provisioned dependencies
          command -v gcc || (echo "ERROR: gcc not found" && exit 1)
      - run: cargo build && cargo test
```

**Works for them because**: Controlled environment, one-time setup

**ColdVox is in this category.**

---

## What This Means for Contributors

### If You Add a New System Dependency

1. **Update the verification script** (`.github/actions/setup-coldvox/action.yml`):
   ```yaml
   required_commands="xdotool ... YOUR_NEW_COMMAND"
   required_pkgs="alsa gtk+-3.0 at-spi-2.0 YOUR_NEW_PKG"
   ```

2. **Document it** in runner setup docs:
   - `docs/self-hosted-runner-complete-setup.md`
   - `docs/tasks/ci-runner-readiness-proposal.md`

3. **Notify the runner admin** (likely yourself):
   ```bash
   # Provision on the runner
   ssh coldaine@laptop-extra
   sudo dnf install -y your-new-package
   ```

4. **Test the CI** will now pass verification

### If You're Setting Up a New Runner

Follow the comprehensive guides:
1. Read: `docs/self-hosted-runner-complete-setup.md`
2. Follow: `docs/tasks/ci-runner-readiness-proposal.md`
3. Verify: Run `gh workflow run "Runner Diagnostic"`
4. Test: Push a commit and watch CI

---

## Exceptions: The Vosk Special Case

### Why Vosk is Different

Vosk dependencies (model + library) **are** installed dynamically by CI:

```bash
# scripts/ci/setup-vosk-cache.sh does download and install
wget -q -O "$MODEL_ZIP" "$MODEL_URL"
wget -q -O "$LIB_ZIP" "$LIB_URL"
```

**Why?**:
1. **Size**: ~47MB model + ~7MB library = 54MB
2. **Versioning**: Model updates independently of system packages
3. **Project-specific**: Not a system-wide dependency
4. **Caching**: Uses runner cache for reuse, not system package manager
5. **No root required**: Installs to `vendor/vosk/` in project, not `/usr/local/`

**But note**: The script still **requires** system tools to be pre-provisioned:
```bash
# Assumes these are already installed:
wget unzip sha256sum pkg-config
```

It doesn't `sudo dnf install wget`, it just uses the pre-installed `wget`.

---

## The Philosophy in One Sentence

> **"CI scripts shouldn't change your system; they should test your code on an already-configured system."**

---

## Conclusion

### Why Setup Scripts Don't Install Dependencies

1. ✅ **Speed**: Verification is instant, installation is slow
2. ✅ **Reliability**: No network/mirror failures during CI
3. ✅ **Security**: No `sudo` in untrusted CI scripts
4. ✅ **Predictability**: Same environment every run
5. ✅ **Control**: Explicit provisioning process
6. ✅ **Separation**: Infrastructure vs application concerns
7. ✅ **Fail-fast**: Loud failures when environment is incomplete

### What You Should Do

**As a Runner Admin**:
- Provision missing dependencies once: `sudo dnf install -y pulseaudio at-spi2-core-devel`
- Keep runner documentation up-to-date
- Run diagnostic workflows after OS updates

**As a Developer**:
- Update verification scripts when adding system dependencies
- Document new requirements in runner setup guides
- Test changes don't introduce new implicit dependencies

**As a Contributor**:
- Understand the pre-provisioning model
- Don't add `sudo apt install` to CI workflows
- Report missing dependencies through proper channels

### The Current Situation

**Missing on Runner**: `pulseaudio`, `at-spi-2.0-devel`  
**Fix**: One SSH + two package installs  
**Time to Fix**: <5 minutes  
**Benefit**: All future CI runs pass  

This is exactly how the system is supposed to work! 🎉

---

## References

- `.github/actions/setup-coldvox/action.yml` - Verification script
- `.github/workflows/runner-diagnostic.yml` - Runner testing workflow
- `scripts/ci/setup-vosk-cache.sh` - Vosk-specific setup (exception)
- `scripts/setup_text_injection.sh` - User-facing setup (does install)
- `docs/self-hosted-runner-complete-setup.md` - Complete runner provisioning guide
- `docs/tasks/ci-runner-readiness-proposal.md` - Dependency requirements and remediation
- `docs/activity/2025-10-10-ci-debug-refactor-text-injection.md` - Today's debugging session

---

*Written: 2025-10-10*  
*Author: GitHub Copilot*  
*Context: Explaining architectural decisions in ColdVox CI system*
