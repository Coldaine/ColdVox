# Comprehensive Dependency Caching Strategy
## Beyond Vosk Models: Complete Self-Hosted Runner Optimization

**Document Created**: 2025-09-11  
**Objective**: Minimize all dependency downloads through comprehensive caching strategy  
**Expected Impact**: Reduce job execution time by 2-5 minutes per run, improve reliability

---

## Current State Analysis

### ✅ Already Cached (Phase 2 Achievements)
- **Vosk Models**: 40MB+ cached at `/home/coldaine/ActionRunnerCache/vosk-models/`
- **Cargo Dependencies**: Partial caching via `Swatinem/rust-cache@v2.8.0`

### ❌ Downloads Every Run (Optimization Opportunities)

#### **1. System Packages (Highest Impact)**
**Current Downloads per Job**:
```bash
# Base dependencies (setup-coldvox): ~50-100MB
alsa-lib-devel, xdotool, libXtst-devel, wget, unzip, @development-tools

# Text injection tests (text_injection_tests job): ~150-300MB  
xorg-x11-server-Xvfb, fluxbox, dbus-x11, at-spi2-core,
wl-clipboard, xclip, ydotool, xorg-x11-utils, wmctrl, gtk3-devel
```
**Time Cost**: 30-120 seconds per job  
**Network Cost**: 200-400MB total downloads  
**Reliability Risk**: External package repository failures

#### **2. Rust Toolchains (High Impact)**
**Current Downloads per Job**:
- Rust stable toolchain: ~100-200MB
- Rust 1.75 MSRV toolchain: ~100-200MB  
- Components (rustfmt, clippy): ~50-100MB
**Time Cost**: 60-180 seconds per job
**Network Cost**: 250-500MB per unique toolchain

#### **3. GitHub Actions (Medium Impact)**
**Current Downloads per Job**:
```yaml
actions/checkout@v4, actions-rust-lang/setup-rust-toolchain@v1,
dtolnay/rust-toolchain@v1, Swatinem/rust-cache@v2.8.0,
rustsec/audit-check@v2.0.0, actions/upload-artifact@v4
```
**Time Cost**: 10-30 seconds per job
**Network Cost**: 10-50MB per job

#### **4. Binary Libraries (Medium Impact)**
**Current Extraction per Job**:
```bash
# libvosk extraction (every job that runs setup-coldvox)
unzip -q vendor/vosk/vosk-linux-x86_64-0.3.45.zip  # ~30MB
sudo cp libvosk.so /usr/local/lib/
sudo cp vosk_api.h /usr/local/include/
```
**Time Cost**: 5-15 seconds per job
**I/O Cost**: Unnecessary file operations

---

## Industry Best Practices & Standards

### **Confirmed Industry Standards**:
1. **Tool Cache**: GitHub's `$RUNNER_TOOL_CACHE` for actions and toolchains
2. **Package Manager Caching**: Pre-installed packages or persistent package cache  
3. **Local Cache Actions**: Direct filesystem caching on self-hosted runners
4. **Docker Layer Caching**: Pre-baked container images with dependencies
5. **Persistent Storage**: Dedicated cache volumes for large binaries

### **2025 GitHub Updates**:
- New cache service (v2) mandatory by February 1st, 2025
- actions/cache v4 required for compatibility
- Self-hosted runners must be ≥ version 2.231.0
- Enhanced performance with 1-hour cache download timeout

---

## Comprehensive Caching Strategy

### **Phase A: System Package Pre-Installation (Highest ROI)**
**Strategy**: Pre-install all commonly used packages on the runner system

#### A.1 Base Package Installation
```bash
# Create persistent package installation script
# /home/coldaine/ActionRunnerCache/setup-system-packages.sh

#!/bin/bash
set -euo pipefail

echo "Installing ColdVox system dependencies..."

# Core dependencies (always needed)
sudo dnf install -y \
  alsa-lib-devel \
  xdotool \
  libXtst-devel \
  wget \
  unzip \
  @development-tools

# Text injection dependencies (for text_injection_tests)  
sudo dnf install -y \
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

# Additional development tools
sudo dnf install -y \
  git \
  curl \
  htop \
  tree \
  jq

echo "✅ System packages pre-installed"
```

#### A.2 Workflow Integration
```yaml
# Replace package installation with validation
- name: Validate System Dependencies  
  run: |
    echo "Validating pre-installed system dependencies..."
    command -v xdotool >/dev/null || { echo "ERROR: xdotool not found"; exit 1; }
    command -v ydotool >/dev/null || { echo "ERROR: ydotool not found"; exit 1; }
    pkg-config --exists gtk+-3.0 || { echo "ERROR: GTK+ 3.0 not found"; exit 1; }
    echo "✅ All system dependencies available"
```

**Expected Improvement**: -60 to -120 seconds per job, -200MB network per job

### **Phase B: Enhanced Toolchain Caching (High ROI)**
**Strategy**: Persistent Rust toolchain storage with runner tool cache

#### B.1 Toolchain Cache Directory Structure
```bash
/home/coldaine/ActionRunnerCache/rust-toolchains/
├── stable/
│   ├── bin/rustc
│   ├── bin/cargo  
│   ├── bin/rustfmt
│   └── bin/clippy
├── 1.75/
│   ├── bin/rustc
│   └── bin/cargo
└── components/
    ├── rustfmt/
    └── clippy/
```

#### B.2 Enhanced Toolchain Setup
```yaml
- name: Setup Cached Rust Toolchain
  run: |
    CACHE_DIR="/home/coldaine/ActionRunnerCache/rust-toolchains"
    TOOLCHAIN="${{ matrix.toolchain || 'stable' }}"
    
    if [ -d "$CACHE_DIR/$TOOLCHAIN" ]; then
      echo "✅ Using cached Rust $TOOLCHAIN toolchain"  
      export PATH="$CACHE_DIR/$TOOLCHAIN/bin:$PATH"
      rustc --version
    else
      echo "📥 Installing and caching Rust $TOOLCHAIN toolchain"
      # Install and copy to cache
      # ... installation logic
    fi
```

**Expected Improvement**: -60 to -180 seconds per job, -250MB network per unique toolchain

### **Phase C: Binary Library Pre-Processing (Medium ROI)**
**Strategy**: Pre-extract and install libvosk permanently

#### C.1 Persistent libvosk Installation
```bash
# One-time setup: /home/coldaine/ActionRunnerCache/setup-libvosk.sh
#!/bin/bash
set -euo pipefail

VOSK_VER=0.3.45
CACHE_DIR="/home/coldaine/ActionRunnerCache/libvosk"

if [ ! -f "/usr/local/lib/libvosk.so" ]; then
  echo "Installing libvosk $VOSK_VER permanently..."
  
  cd "$CACHE_DIR"
  unzip -q "../vendor/vosk-linux-x86_64-${VOSK_VER}.zip"
  sudo cp "vosk-linux-x86_64-${VOSK_VER}/libvosk.so" /usr/local/lib/
  sudo cp "vosk-linux-x86_64-${VOSK_VER}/vosk_api.h" /usr/local/include/
  sudo ldconfig
  
  echo "✅ libvosk installed permanently"
else
  echo "✅ libvosk already available"
fi
```

#### C.2 Workflow Simplification  
```yaml
# Replace extraction with validation
- name: Validate libvosk Installation
  run: |
    if [ ! -f "/usr/local/lib/libvosk.so" ]; then
      echo "ERROR: libvosk not found, run setup-libvosk.sh"
      exit 1
    fi
    echo "✅ libvosk available at /usr/local/lib/libvosk.so"
```

**Expected Improvement**: -5 to -15 seconds per job, reduced I/O operations

### **Phase D: GitHub Actions Tool Cache (Medium ROI)**
**Strategy**: Leverage `$RUNNER_TOOL_CACHE` for persistent action caching

#### D.1 Tool Cache Structure
```bash
# GitHub automatically manages: /opt/hostedtoolcache/
# For self-hosted: $RUNNER_TOOL_CACHE (default: /home/coldaine/actions-runner/_work/_tool)
$RUNNER_TOOL_CACHE/
├── actions-checkout/
├── rust-toolchain/  
├── security-audit/
└── cache-action/
```

#### D.2 Enhanced Action Caching
```yaml
# Actions will automatically use tool cache when available
# No workflow changes needed - performance improvement is automatic
```

**Expected Improvement**: -10 to -30 seconds per job, -10MB network per job

---

## Implementation Priority & Timeline

### **Priority 1: System Package Pre-Installation (Week 1)**
- **Impact**: Highest ROI - eliminates largest downloads
- **Effort**: Low - one-time runner setup  
- **Risk**: Low - packages are stable, well-tested

**Actions**:
1. Create and run system package installation script
2. Update workflows to validate instead of install
3. Test on development workflows first

### **Priority 2: Enhanced Rust Toolchain Caching (Week 1-2)**  
- **Impact**: High ROI - eliminates toolchain downloads
- **Effort**: Medium - requires workflow integration
- **Risk**: Medium - toolchain compatibility considerations

**Actions**:
1. Set up persistent toolchain cache directory
2. Create enhanced toolchain setup scripts
3. Update workflows with conditional caching logic
4. Test with both stable and MSRV toolchains

### **Priority 3: Binary Library Pre-Processing (Week 2)**
- **Impact**: Medium ROI - small but consistent savings
- **Effort**: Low - simple pre-extraction
- **Risk**: Low - binary compatibility is stable

**Actions**:  
1. Pre-extract libvosk to system locations
2. Update setup-coldvox action to validate instead of extract
3. Verify library compatibility across all workflows

### **Priority 4: GitHub Actions Tool Cache (Week 2)**
- **Impact**: Medium ROI - automatic improvements
- **Effort**: None - GitHub handles automatically
- **Risk**: None - standard GitHub functionality

---

## Success Metrics & Expected Improvements

### **Performance Improvements (Conservative Estimates)**
- **Job Execution Time**: -2 to -5 minutes per job (20-40% faster)
- **Network Usage**: -500MB to -1GB per job (60-80% reduction)  
- **Reliability**: Eliminate external dependency failures during package downloads
- **Cost**: Reduced GitHub Actions minutes usage

### **Specific Job Improvements**
```
build_and_check:        12 min → 8-10 min  (-20-33%)
text_injection_tests:   15 min → 10-12 min (-20-33%)  
msrv-check:            10 min → 7-8 min   (-20-30%)
Overall CI pipeline:    35 min → 25-30 min (-15-30%)
```

### **Monitoring & Validation**
- **Performance monitoring**: Track job execution times before/after
- **Cache hit rates**: Monitor successful cache usage
- **Failure analysis**: Ensure no new failure modes introduced
- **Storage monitoring**: Track cache directory disk usage

---

## Risk Assessment & Mitigation

### **Potential Risks**

| Risk | Impact | Likelihood | Mitigation |
|------|---------|------------|------------|
| **Cache corruption** | HIGH | LOW | Automated cache validation, easy rebuild |
| **Disk space exhaustion** | MEDIUM | MEDIUM | Automated cleanup, monitoring alerts |
| **Version incompatibility** | MEDIUM | LOW | Version pinning, compatibility testing |
| **Build dependency changes** | LOW | MEDIUM | Regular cache updates, fallback to downloads |

### **Fallback Strategies**
- **Cache miss handling**: Automatic fallback to download-based installation
- **Cache corruption**: Automated cache rebuild procedures  
- **Disk space**: Automatic cleanup of oldest/unused cache entries
- **Emergency rollback**: Quick reversion to current download-based workflows

---

## Long-term Maintenance Plan

### **Automated Maintenance**
```bash
# Weekly cache maintenance script
/home/coldaine/ActionRunnerCache/maintenance.sh:
- Cleanup unused cache entries (>30 days old)
- Update system packages 
- Refresh toolchain caches
- Validate cache integrity
- Generate cache usage reports
```

### **Update Procedures**
- **System packages**: Monthly updates aligned with system maintenance
- **Rust toolchains**: Update with each Rust release or as needed
- **Binary libraries**: Update with ColdVox dependency updates
- **Cache validation**: Continuous validation in workflows

---

## Implementation Checklist

### **Phase A: System Package Pre-Installation**
- [ ] Create system package installation script
- [ ] Execute one-time package installation on runner
- [ ] Update workflows to validate instead of install packages
- [ ] Test package availability across all workflow jobs
- [ ] Measure performance improvement

### **Phase B: Enhanced Toolchain Caching**  
- [ ] Set up persistent Rust toolchain cache directories
- [ ] Create enhanced toolchain setup scripts
- [ ] Update workflows with conditional caching logic
- [ ] Test stable and MSRV toolchain caching
- [ ] Validate component availability (rustfmt, clippy)

### **Phase C: Binary Library Pre-Processing**
- [ ] Pre-extract libvosk to permanent system location
- [ ] Update setup-coldvox action to validate instead of extract
- [ ] Test libvosk availability across all jobs
- [ ] Verify library linking and functionality

### **Phase D: Validation & Monitoring**
- [ ] Implement cache performance monitoring
- [ ] Create cache usage dashboard
- [ ] Set up automated cache maintenance
- [ ] Document cache management procedures
- [ ] Train team on cache troubleshooting

---

## Expected Timeline: 1-2 Weeks

**Week 1**: Focus on highest impact items (system packages, toolchain caching)  
**Week 2**: Complete binary pre-processing, monitoring, and validation

**Total Expected Improvement**: 2-5 minutes faster per job, 500MB-1GB less network usage per job, significantly improved reliability by eliminating external download dependencies.

**ROI**: Significant time savings on every CI run, improved developer productivity, reduced infrastructure load, enhanced build reliability.