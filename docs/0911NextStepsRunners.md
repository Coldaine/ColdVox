# ColdVox Self-Hosted Runner: Practical Next Steps

**Date**: 2025-09-11  
**Context**: Post-analysis of CI failures and realistic solutions for personal development setup  
**Goal**: Fix the blockers with minimal complexity  

---

## Current Blockers (What's Actually Broken)

1. **Cache conflicts**: "Failed to CreateCacheEntry: (409) Conflict" errors
2. **Performance monitor script**: `unbound variable: cpu_usage` error  
3. **Workflow cancellations**: Jobs cancelled by "higher priority waiting request"
4. **Slow CI**: 500MB-1GB downloads per job (5-10 min waste)
5. **Flaky GUI/audio tests**: Xvfb/D-Bus setup issues causing hangs

## The 80/20 Fix Plan

### Immediate Fixes (30 minutes total)

#### 1. Fix Cache Conflicts (5 minutes)
Update `.github/workflows/ci.yml` cache keys:

```yaml
# Current problematic cache
- uses: Swatinem/rust-cache@v2

# Fix: Add runner labels to cache key
- uses: Swatinem/rust-cache@v2
  with:
    key: ${{ runner.os }}-${{ join(runner.labels, '-') }}-${{ hashFiles('**/Cargo.lock') }}
    restore-keys: |
      ${{ runner.os }}-${{ join(runner.labels, '-') }}-
      ${{ runner.os }}-
```

#### 2. Fix Performance Monitor Script (5 minutes)
Replace the buggy `scripts/performance_monitor.sh` with:

```bash
#!/bin/bash
# Fixed version - initialize all variables
set -euo pipefail

get_system_metrics() {
    # Initialize with defaults to prevent unbound variable errors
    local load_avg="0.0"
    local memory_usage="0"
    local disk_usage="0"
    local runner_cpu="0.0" 
    local runner_mem="0.0"
    
    # Get actual values
    load_avg=$(cut -d' ' -f1 /proc/loadavg 2>/dev/null || echo "0.0")
    memory_usage=$(free -m | awk '/^Mem:/ {print $3}' 2>/dev/null || echo "0")
    disk_usage=$(df /home | awk 'NR==2 {gsub(/%/, "", $5); print $5}' 2>/dev/null || echo "0")
    
    # Runner process stats
    if pgrep -f "Runner.Listener" >/dev/null; then
        local runner_pid=$(pgrep -f "Runner.Listener" | head -1)
        local stats=$(ps -p "$runner_pid" -o %cpu,%mem --no-headers 2>/dev/null || echo "0.0 0.0")
        runner_cpu=$(echo "$stats" | awk '{print $1}')
        runner_mem=$(echo "$stats" | awk '{print $2}')
    fi
    
    echo "$load_avg,$memory_usage,$disk_usage,$runner_cpu,$runner_mem"
}

# Rest of script stays the same but with proper variable initialization
```

#### 3. Fix Workflow Cancellations (5 minutes)
Update concurrency in `.github/workflows/ci.yml`:

```yaml
concurrency:
  group: ci-${{ github.ref }}-${{ github.run_number }}  # More specific
  cancel-in-progress: true
```

#### 4. Add Simple Fallback (10 minutes)
For reliability when laptop is busy/offline:

```yaml
# In job definitions, change from:
runs-on: [self-hosted, Linux, X64, fedora, nobara]

# To:
runs-on: ${{ github.actor == 'coldaine' && '[self-hosted, Linux, X64, fedora, nobara]' || 'ubuntu-latest' }}
```

#### 5. Fix GUI Test Environment (5 minutes)
Create simple `scripts/start-headless.sh`:

```bash
#!/bin/bash
set -euo pipefail

export DISPLAY=:99

# Start Xvfb with timeout
timeout 30 Xvfb :99 -screen 0 1280x1024x24 -ac &
sleep 2

# Start window manager  
timeout 30 fluxbox -display :99 &
sleep 1

# Start D-Bus session
eval $(dbus-launch --sh-syntax)
export DBUS_SESSION_BUS_ADDRESS
export DBUS_SESSION_BUS_PID

echo "Headless environment ready"
echo "DISPLAY=$DISPLAY" >> $GITHUB_ENV  
echo "DBUS_SESSION_BUS_ADDRESS=$DBUS_SESSION_BUS_ADDRESS" >> $GITHUB_ENV
echo "DBUS_SESSION_BUS_PID=$DBUS_SESSION_BUS_PID" >> $GITHUB_ENV
```

### High-Impact Optimization (1 hour setup, permanent benefit)

#### Pre-install All Dependencies Once
Run this setup script on your runner machine:

```bash
#!/bin/bash
# scripts/setup-runner-once.sh - Run this ONCE on your laptop
set -euo pipefail

echo "=== One-time runner optimization ==="

# Install all system dependencies permanently  
sudo dnf install -y --skip-unavailable \
    alsa-lib-devel pulseaudio-libs-devel pipewire-devel \
    libXtst-devel gtk3-devel qt6-qtbase-devel \
    xorg-x11-server-Xvfb fluxbox dbus-x11 at-spi2-core \
    wl-clipboard xclip ydotool xdotool wmctrl \
    @development-tools cmake pkg-config bc jq

# Install multiple Rust toolchains
rustup toolchain install stable
rustup toolchain install 1.75.0  # MSRV
rustup default stable

# Pre-install cargo tools
cargo install --locked cargo-nextest cargo-audit

# Run the existing libvosk setup
./scripts/setup-permanent-libvosk.sh

echo "✅ Runner optimized! Expected savings: 5-10 minutes per job"
```

**Expected impact**: Eliminates 500MB+ downloads per job, reduces CI time by 50%

## What NOT to Implement (Overengineered)

❌ **Multi-runner pool** - You have one laptop  
❌ **Smart dispatcher** - If laptop is busy, job waits  
❌ **Security sandboxing** - You're running your own code  
❌ **Performance monitoring dashboards** - Simple logs are fine  
❌ **Feature matrix explosion** - Test what you're working on  
❌ **Historical analytics** - Not worth the complexity  

## Testing the Fixes

After implementing:

1. **Test cache fix**: Push a commit, verify no 409 conflicts
2. **Test performance script**: Run `./scripts/performance_monitor.sh start` 
3. **Test GUI environment**: Run `./scripts/start-headless.sh` and verify no hangs
4. **Test dependency pre-install**: Time a full CI job (should be <5 minutes)

## Success Metrics

- **Cache conflicts**: 0 per week (was ~5-10)
- **Job completion rate**: >95% (was ~65%)
- **Average job time**: <5 minutes (was 15-25)
- **Manual intervention**: <1 per week (was daily)

## If You Have Extra Time Later

**Optional improvements** (only if the above isn't enough):

- Add basic job timeout protection
- Improve error messages in scripts  
- Add simple health check before jobs
- Create a "nuke and restart" script for when things go wrong

---

## Implementation Order

1. **Today**: Cache fixes + performance script fix (15 minutes)
2. **This week**: Run the one-time dependency setup (1 hour)
3. **Next week**: GUI test improvements (if still having issues)
4. **Future**: Consider fallback strategy if laptop reliability becomes an issue

**Total time investment**: ~2 hours  
**Expected benefit**: Reliable, fast CI that "just works"

The goal is a boring, reliable CI system that you never have to think about.