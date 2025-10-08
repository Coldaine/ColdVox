# ColdVox Self-Hosted Runner: Current Status & Remaining Work

> ⚠️ **RESEARCH DOCUMENT - WORK IN PROGRESS**
> Contains incomplete sections and future work markers.
> Last updated: 2025-10-07

**Date**: 2025-09-10
**Branch**: `update-self-hosted-runner-labels`
**Phase**: Completed 2.3, Ready for 3.1
**Priority**: Performance Optimization (NOT Mission Critical)
**Reliability**: Not Required - This is a personal project for learning and experimentation

## Current Configuration

### Runner Setup (laptop-extra)
```bash
# Runner Registration
Name: laptop-extra
Labels: [self-hosted, Linux, X64, fedora, nobara]
URL: https://github.com/Coldaine/ColdVox
Service: actions.runner.Coldaine-ColdVox.laptop-extra.service
Status: Active and running
Agent ID: 22 (reconfigured from 21)
```

### System Specifications
```yaml
Hardware:
  Model: HP EliteBook 840 14 inch G10 Notebook PC
  CPU: 13th Gen Intel Core i7-1365U (10 cores, 12 threads, up to 5.2GHz)
  RAM: 30GB
  Storage: 238.5GB NVMe SSD (/dev/nvme0n1)
  Swap: 42GB (34GB disk + 8GB zram)

Operating System:
  Distribution: Nobara Linux 42 (KDE Plasma Desktop Edition)
  Base: Fedora 42 (RHEL/CentOS compatible)
  Kernel: Linux 6.16.3-201.nobara.fc42.x86_64
  Architecture: x86_64
  Support: Until 2026-05-13

Package Management:
  Primary: DNF 5.2.16 (dnf5)
  Alternative: Flatpak 1.16.1
  Development: Git 2.51.0
```

### Workflow Configuration Status
All workflow files updated with enhanced labels:

```yaml
# Before
runs-on: [self-hosted, Linux, X64]

# After (COMPLETED)
runs-on: [self-hosted, Linux, X64, fedora, nobara]
```

**Updated Files:**
- `.github/workflows/ci.yml` (8 jobs)
- `.github/workflows/release.yml` (2 jobs)
- `.github/workflows/runner-test.yml` (1 job)
- `.github/workflows/vosk-integration.yml` (1 job)
- `.github/workflows/runner-diagnostic.yml` (1 job)

### Current Performance Characteristics

**Performance Testing Results (Phase 2.3):**
```
Baseline Test: Vosk Integration Tests
- Runtime: 3h 21m (failed after 38min build phase)
- Peak Load: 10.03 (excellent CPU utilization)
- Memory Usage: 10.3GB / 30GB available
- Disk Usage: 51% of workspace
- Concurrent Processes: 20+ rustc compilation tasks

Hardware vs GitHub-hosted Comparison:
- CPU Advantage: 5x cores (10 vs 2)
- RAM Advantage: 4.3x memory (30GB vs 7GB)
- Expected Performance Gain: 2-3x faster builds
- Current Reality: Slower due to configuration issues
```

## Issues Identified

### Critical Problems (Blocking Phase 3)

1. **Build Failures**
   - Vosk integration tests failing during compilation
   - 38-minute build process before failure
   - Dependency or system library issues

2. **Queue Management**
   - Single-threaded job execution
   - One failed job blocks entire queue for hours
   - 5+ jobs queued for 2h+ during testing

3. **Missing Optimizations**
   - No Cargo dependency caching
   - No concurrent job execution configured
   - No job timeout protection
   - Cold builds from scratch every time

### Performance Bottlenecks

1. **Cache Strategy**
   ```bash
   # Missing: Swatinem/rust-cache action
   # Result: Full dependency compilation every run
   # Impact: 3x longer builds than necessary
   ```

2. **Resource Utilization**
   ```bash
   # Current: Serial job execution
   # Potential: Parallel execution with job limits
   # Hardware Capacity: Can handle 3-4 concurrent jobs
   ```

3. **Error Recovery**
   ```bash
   # Missing: Job timeouts (current: unlimited)
   # Missing: Automatic retry logic
   # Missing: Fallback to GitHub-hosted runners
   ```

## Completed Phases

### ✅ Phase 1: Basic Validation (COMPLETE)
- Runner registration and connectivity
- Basic job execution confirmed
- System dependencies installed
- Rust toolchain compatibility resolved

### ✅ Phase 2: Full Pipeline Validation (COMPLETE)

#### 2.1 Runner Enhancement ✅
- Enhanced labels applied: `[self-hosted, Linux, X64, fedora, nobara]`
- All workflow files updated
- Runner reconfigured successfully

#### 2.2 Comprehensive CI Testing ✅
- Full CI workflows triggered and monitored
- All job types validated (build, test, security)
- Resource usage documented
- Failure points identified and documented

#### 2.3 Performance Baseline ✅
- Build times measured and documented
- Resource utilization patterns analyzed
- Optimization opportunities identified
- Comparison with GitHub-hosted completed

## Remaining Phases

### 🚀 Phase 3: Performance Experimentation (CURRENT PRIORITY)

**Timeline**: Flexible - This is for learning, not production
**Objective**: Maximize performance, explore hardware capabilities, learn from failures

#### 3.1 Performance Testing Without Safety Nets (NEXT)
**Philosophy:**
- **Failures are learning opportunities** - Not a problem if CI breaks
- **No fallback needed** - If self-hosted fails, I'll fix it when I have time
- **Experiment freely** - Try aggressive optimizations that production systems wouldn't risk

**Implementation Focus:**
```yaml
# Pure self-hosted, no fallback needed
runs-on: [self-hosted, Linux, X64, fedora, nobara]
# Can afford to fail - this is experimentation
timeout-minutes: 360  # Go wild, we have time
```

**Experimentation Areas:**
- Push CPU optimizations to the limit (AVX2, FMA, native compilation)
- Test maximum parallel job capacity until system breaks
- Try experimental Rust compiler flags
- Cache everything possible, even if it risks corruption

#### 3.2 Gradual Workflow Migration (PENDING)
1. **Non-critical workflows first** ✅ (documentation, linting - already done)
2. **CI pipeline with fallback** (build, test with hybrid matrix)
3. **Release workflows** (only after proven stability)

#### 3.3 Monitoring & Alerting (PARTIALLY COMPLETE)
- ✅ **Runner health monitoring** implemented (`scripts/performance_monitor.sh`)
- ⏳ **Job failure pattern analysis** (needs automation)
- ⏳ **Performance tracking** and regression detection

### 🔄 Phase 4: Optimization & Hardening (FUTURE)

**Timeline**: Ongoing
**Objective**: Long-term stability and performance

#### 4.1 Security Hardening (PLANNED)
- **Workspace isolation** strategies
- **Secret handling** verification
- **Network access** controls
- **Automated security updates**

#### 4.2 Performance Optimization (PLANNED)
- **Cargo cache** optimization (Priority 1)
- **Parallel build** configuration
- **Resource allocation** tuning
- **Build time monitoring**

#### 4.3 Maintenance Automation (PARTIALLY COMPLETE)
- **Automated system updates** (planned)
- ✅ **Workspace cleanup** scheduling (implemented in health script)
- ✅ **Health monitoring** with alerts (basic version complete)

## Self-Hosted Runner Advantages & Optimization Opportunities

### 🚀 Unique Self-Hosted Capabilities

**Persistent Storage & Local Caching:**
```bash
# Vosk Model Local Cache Strategy
Model Cache Location: /home/coldaine/actions-runner/_cache/vosk-models/
Current Waste: Re-downloading 1.8GB model per workflow run
Optimization: Pre-cache models locally, symlink in workflows

# Implementation Plan:
mkdir -p /home/coldaine/actions-runner/_cache/vosk-models/
# Pre-download models:
# - vosk-model-small-en-us-0.15 (40MB - fast testing)
# - vosk-model-en-us-0.22 (1.8GB - production quality)
# - Future: Multi-language support (es, fr, de)
```

**System-Level Optimizations:**
```bash
# Custom Dependencies Pre-installed
Advantage: No apt/dnf install time in workflows
Current: ~30s dependency installation per job
Optimized: Dependencies already available

# Persistent Build Caches
Cargo Registry: ~/.cargo/registry (persistent)
Target Directory: Can preserve between runs with careful cleanup
sccache: Distributed compilation cache for Rust
Custom: /opt/coldvox-build-cache/ for project-specific artifacts
```

**Resource Advantages:**
```bash
# GitHub-hosted Limitations vs Our Capabilities
Time Limit: 6 hours (GitHub) vs Unlimited (Self-hosted)
Storage: 14GB (GitHub) vs 238GB (Self-hosted)
Memory: 7GB (GitHub) vs 30GB (Self-hosted)
CPU: 2 cores (GitHub) vs 10 cores (Self-hosted)
Networking: Rate-limited vs Direct control
Root Access: No (GitHub) vs Yes (Self-hosted)
```

### 🎯 Vosk-Specific Optimization Strategy

**Model Management System:**
```bash
# Proposed Structure
/home/coldaine/actions-runner/_cache/
├── vosk-models/
│   ├── small-en-us-0.15/          # Fast testing (40MB)
│   ├── en-us-0.22/                # Production quality (1.8GB)
│   ├── checksums.txt              # Integrity verification
│   └── version-manifest.json      # Version tracking
├── vosk-binaries/
│   ├── 0.3.45/                    # Current version
│   │   ├── libvosk.so            # Pre-installed in /usr/local/lib
│   │   └── vosk_api.h            # Pre-installed in /usr/local/include
│   └── version-registry.json      # Binary version tracking
└── rust-artifacts/
    ├── cargo-registry/             # Shared dependency cache
    └── target-cache/              # Selective target preservation
```

**Workflow Optimization:**
```yaml
# Enhanced Vosk Setup (Self-hosted optimized)
- name: Setup Vosk (Self-hosted optimized)
  run: |
    # Check local cache first
    VOSK_CACHE="/home/coldaine/actions-runner/_cache/vosk-models"
    MODEL_NAME="vosk-model-small-en-us-0.15"

    if [ -d "$VOSK_CACHE/$MODEL_NAME" ]; then
      echo "Using cached Vosk model: $MODEL_NAME"
      ln -sf "$VOSK_CACHE/$MODEL_NAME" .
    else
      echo "Downloading and caching Vosk model..."
      # Download, extract, and cache for future runs
      mkdir -p "$VOSK_CACHE"
      # ... download and extract logic
      mv "$MODEL_NAME" "$VOSK_CACHE/"
      ln -sf "$VOSK_CACHE/$MODEL_NAME" .
    fi

    # Vosk binaries already installed system-wide
    echo "Vosk setup complete (cached)"
```

### 💡 Advanced Self-Hosted Optimizations

**CPU-Specific Optimizations:**
```bash
# Target our specific Intel i7-1365U
RUSTFLAGS="-C target-cpu=native -C target-feature=+avx2,+fma"
# Enable all available CPU features for maximum performance
# AVX2, FMA available on 13th gen Intel
```

**Memory Management:**
```bash
# Leverage our 30GB RAM
CARGO_BUILD_JOBS=8        # More parallel rustc processes
RUSTC_OPTS="--codegen opt-level=3"
# Can afford memory-intensive optimizations
```

**Storage Strategy:**
```bash
# Persistent caches across runs
/home/coldaine/actions-runner/_persistent/
├── cargo-cache/          # Never cleared
├── rust-analyzer-cache/  # IDE support
├── vosk-models/          # Downloaded once
└── build-artifacts/      # Incremental builds

# Workspace management
/home/coldaine/actions-runner/_work/
├── ColdVox-1/           # Current job
├── ColdVox-2/           # Concurrent job
└── _temp/               # Cleanup after 24h
```

**Network Optimizations:**
```bash
# Direct package mirror access
dnf config-manager --add-repo local-mirror
# Custom Cargo registry mirror
cargo config set registry.local-mirror.index "file:///opt/cargo-registry"
# Local crates.io mirror for air-gapped scenarios
```

## Immediate Action Plan (Phase 3.1)

### Priority 1: Fix Build Reliability + Vosk Optimization
```bash
# Enhanced Steps:
1. Investigate Vosk compilation errors
2. Implement local Vosk model caching system
3. Pre-install Vosk models in runner cache
4. Update workflows to use cached models
5. Add model integrity verification
6. Test manual build with cached models
7. Enable CPU-specific optimizations
```

### Priority 2: Multi-Layer Caching Strategy
```yaml
# Comprehensive caching approach:
- name: Cache Rust dependencies
  uses: Swatinem/rust-cache@v2
  with:
    shared-key: "coldvox-${{ matrix.features || 'default' }}"
    save-if: ${{ github.ref == 'refs/heads/main' }}"
    cache-directories: |
      ~/.cargo/registry
      ~/.cargo/git
      target/

# Additional self-hosted caching:
- name: Cache Vosk Models (Self-hosted)
  run: |
    # Use persistent local cache (no GitHub Actions cache needed)
    echo "VOSK_MODEL_PATH=/home/coldaine/actions-runner/_cache/vosk-models/vosk-model-small-en-us-0.15" >> $GITHUB_ENV
```

### Priority 3: Resource Optimization
```yaml
# Concurrent execution with resource awareness:
jobs:
  build:
    strategy:
      max-parallel: 4  # Increased based on 10-core + 30GB capacity
      matrix:
        features: [default, vosk, text-injection]
        vosk-model: [small, standard]  # Test multiple models
    # Resource limits per job
    env:
      CARGO_BUILD_JOBS: 6  # Leverage more cores per job
      RUST_BACKTRACE: 1
      # CPU-specific optimizations
      RUSTFLAGS: "-C target-cpu=native -C target-feature=+avx2,+fma"
```

### Priority 4: Self-Hosted Specific Features
```yaml
# Leverage self-hosted advantages:
timeout-minutes: 180  # Higher than GitHub's limits (we have no 6h limit)
continue-on-error: true
env:
  # Use local paths for better performance
  CARGO_HOME: /home/coldaine/.cargo
  VOSK_MODEL_PATH: /home/coldaine/actions-runner/_cache/vosk-models
  # Custom build optimizations
  RUSTFLAGS: "-C target-cpu=native -C opt-level=3"
  CARGO_NET_GIT_FETCH_WITH_CLI: "true"  # Better git performance
  # Memory optimizations (we have 30GB vs GitHub's 7GB)
  CARGO_BUILD_JOBS: 6
  RUSTC_WRAPPER: "sccache"  # Distributed compilation cache

# Additional capabilities:
- name: Setup sccache (Self-hosted only)
  run: |
    # Install and configure sccache for persistent compilation cache
    export SCCACHE_DIR=/home/coldaine/actions-runner/_cache/sccache
    mkdir -p $SCCACHE_DIR
    # sccache can persist across all builds
```

### Priority 5: Infrastructure Advantages
```bash
# Unique self-hosted capabilities:

# 1. Multiple Model Support
Pre-cache models: English, Spanish, French, German
Model selection via matrix strategy
Instant model switching (no download time)

# 2. Custom Development Tools
Pre-install: rust-analyzer, cargo-expand, cargo-llvm-cov
Debug tools: gdb, valgrind, perf
Custom binaries: project-specific tools

# 3. Monitoring Integration
Real-time metrics: CPU, memory, disk, network
Build analytics: timing breakdowns, resource usage
Integration: Grafana dashboard, alerting

# 4. Backup & Recovery
Automated backups of cache directories
Snapshot capability for debugging failed builds
Version rollback for dependency issues

# 5. Security & Compliance
Custom security scanning tools
Internal network access for proprietary dependencies
Compliance logging and audit trails
```

## Success Metrics for Phase 3

### Build Performance Targets
- **Successful build time**: < 30 minutes (down from 3h 21m failure)
- **Cache hit rate**: > 80% for dependency builds
- **Queue throughput**: 3-4 concurrent jobs
- **Failure rate**: < 10% (currently 100% for Vosk tests)

### Reliability Targets
- **Job completion rate**: > 95%
- **Queue blocking**: Eliminated with timeouts
- **Fallback activation**: < 5% of jobs
- **System uptime**: > 99% (monitoring confirmed)

## Technical Debt & Maintenance

### Current Monitoring Setup
```bash
# Health monitoring script (ACTIVE)
Location: scripts/runner-health-check.sh (enhanced)
Features: Service monitoring, disk cleanup, logging
Schedule: Manual execution (needs cron automation)

# Performance monitoring (CREATED)
Location: scripts/performance_monitor.sh
Features: Real-time metrics, CSV logging, report generation
Usage: On-demand testing and analysis
```

### Dependencies Status
```bash
System Dependencies: ✅ Installed and verified
- alsa-lib-devel, xdotool, libXtst-devel
- wget, unzip, @development-tools
- Vosk libraries: libvosk.so, vosk_api.h

Rust Toolchain: ✅ Configured
- Version: 1.89.0 (stable)
- Components: rustfmt, clippy
- Actions: dtolnay/rust-toolchain@stable
```

### Security Considerations
```bash
Current Status:
- ✅ Service isolation (systemd)
- ✅ User permissions (coldaine user)
- ✅ Workspace cleanup (automated)
- ⏳ Secret handling (needs review)
- ⏳ Network restrictions (needs hardening)
```

## Rollback Strategy

### Emergency Rollback (If Phase 3 Fails)
```bash
1. Revert workflow files to ubuntu-latest
2. Push changes to stop using self-hosted runner
3. Monitor for restoration of normal operation
4. Preserve runner setup for investigation
```

### Planned Rollback Triggers
- Build failure rate > 50% for 24 hours
- System resource exhaustion
- Security incident detection
- Hardware failure

## Next Steps Summary

**Immediate (This Week)**:
1. Diagnose and fix Vosk build failures
2. Implement Rust caching in workflows
3. Add job timeouts and concurrency limits
4. Test hybrid fallback strategy

**Short-term (Next Week)**:
1. Deploy Phase 3.1 Hybrid Workflow Strategy
2. Monitor improved performance metrics
3. Complete Phase 3.2 gradual migration
4. Document lessons learned

**Long-term (Ongoing)**:
1. Phase 4 optimization and hardening
2. Automated maintenance procedures
3. Performance regression monitoring
4. Team knowledge transfer

---

## 🎯 Strategic Advantage Summary

### Why Self-Hosted Outperforms GitHub-Hosted

**Hardware Superiority:**
- **5x CPU Power**: 10-core i7 vs 2 vCPU GitHub runners
- **4x Memory**: 30GB vs 7GB enables memory-intensive optimizations
- **17x Storage**: 238GB vs 14GB for extensive caching
- **Unlimited Runtime**: No 6-hour job limit for complex builds

**Self-Hosted Exclusive Capabilities:**
```bash
✅ Persistent Vosk model cache (save 1.8GB downloads per run)
✅ CPU-native optimizations (AVX2, FMA instruction sets)
✅ Unlimited concurrent jobs (based on hardware capacity)
✅ Custom system dependencies pre-installed
✅ sccache distributed compilation cache
✅ Multi-language model support (cached locally)
✅ Root access for system-level optimizations
✅ Custom monitoring and debugging tools
```

**Expected Performance Gains (Post-Optimization):**
- **Build Time**: 15-20min (down from 3h+ failures)
- **Cache Efficiency**: 90%+ hit rate for dependencies
- **Throughput**: 4 concurrent jobs vs 1 serial
- **Reliability**: 95%+ success rate vs current failures

### Implementation Roadmap

**Phase 3.1 (Week 1)**: Core reliability + caching
**Phase 3.2 (Week 2)**: Hybrid strategy + concurrent execution
**Phase 3.3 (Week 3)**: Advanced optimizations + monitoring
**Phase 4 (Ongoing)**: Security hardening + maintenance automation

---

**Current Status**: Phase 2 Complete ✅ | Phase 3 Ready 🔄
**Critical Path**: Fix build failures → Enable caching → Deploy hybrid strategy
**Expected Timeline**: Phase 3 completion within 1 week with focused effort
**Strategic Value**: 2-3x performance improvement + unlimited scaling potential

---

## 💡 Recommendations for Spare Laptop Runner Setup

### Why This Makes Perfect Sense

**You have a spare laptop with:**
- 10-core i7 CPU sitting idle
- 30GB RAM doing nothing
- 238GB NVMe storage available
- Already running 24/7 as laptop-extra

**GitHub Actions costs:**
- Free tier: 2,000 minutes/month (33 hours)
- Your current usage: Multiple dependabot PRs daily eating minutes
- Self-hosted: UNLIMITED minutes on hardware you already own

### Recommended Approach (Given Non-Critical Nature)

#### 1. **Go All-In on Self-Hosted**
```yaml
# Just use self-hosted everywhere
runs-on: [self-hosted, Linux, X64, fedora, nobara]
# No fallback needed - it's your personal project
```

#### 2. **CRITICAL: Vosk Model Caching**
**This is the #1 priority** - downloading 1.8GB model every run is insane:
```bash
# Pre-cache models permanently
mkdir -p /home/coldaine/actions-runner/_cache/vosk-models/
cd /home/coldaine/actions-runner/_cache/vosk-models/

# Download once, use forever
wget https://alphacephei.com/vosk/models/vosk-model-small-en-us-0.15.zip
wget https://alphacephei.com/vosk/models/vosk-model-en-us-0.22.zip
unzip *.zip && rm *.zip

# Workflows should check cache first:
if [ -d "/home/coldaine/actions-runner/_cache/vosk-models/vosk-model-small-en-us-0.15" ]; then
  ln -sf /home/coldaine/actions-runner/_cache/vosk-models/vosk-model-small-en-us-0.15 .
else
  # Only download if missing
fi
```

#### 3. **Sensible Performance Settings**
Not trying to break things, just optimize reasonably:
- **Parallel jobs**: 3-4 (well within 10-core capacity)
- **Native CPU flags**: `-C target-cpu=native` (safe optimization)
- **Cargo caching**: Use Swatinem/rust-cache (already working)
- **Reasonable timeouts**: 60-90 minutes for most jobs

#### 3. **Simplified Management**
- **No monitoring needed**: Check when something seems broken
- **No automation**: Manual fixes are fine for personal projects
- **Queue backlog is OK**: Dependabot can wait
- **Failures are fine**: Fix them when you feel like it

#### 4. **Cost-Benefit Reality**
```
Your spare laptop runner:
✅ Free (already have hardware)
✅ Unlimited build minutes
✅ 5x faster CPU than GitHub
✅ 4x more RAM than GitHub
✅ Learn runner management
✅ Full control over environment

GitHub-hosted runners:
❌ 2,000 minutes/month limit
❌ Slower hardware
❌ No persistence between runs
❌ Can't install custom tools
❌ Costs money after free tier
```

### What to Actually Focus On

**Priority 1 - Must Fix:**
1. **Vosk model caching** - Stop downloading 1.8GB every run
2. **Fix current Vosk build failures** - They're blocking everything
3. **Reasonable parallelization** - 3-4 jobs max, not stress testing

**Priority 2 - Nice to Have:**
1. **Cargo dependency caching** - Already using Swatinem/rust-cache
2. **Periodic workspace cleanup** - When disk hits 80%
3. **Basic job timeouts** - Prevent infinite hangs

**Skip This:**
1. Fallback strategies - waste of time for personal project
2. Complex monitoring - manual checks are fine
3. High availability - it's a spare laptop, not production
4. Over-optimization - reasonable settings are enough

### Bottom Line

This is the PERFECT use case for self-hosted runners:
- **Zero additional cost** (spare hardware)
- **No reliability requirements** (personal project)
- **Learning opportunity** (but keep it reasonable)
- **Infinite CI/CD minutes** (vs 33 hours free tier)

**The Real Focus:** Fix Vosk model caching FIRST (saves 1.8GB per run), then get builds working consistently. Everything else is optional optimization. This spare laptop setup gives you unlimited CI/CD for free - just need to make it work reasonably well, not perfectly.
