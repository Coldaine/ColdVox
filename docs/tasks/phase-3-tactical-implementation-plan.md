# Phase 3 Tactical Implementation Plan
## Self-Hosted Runner Production Migration with Safety Rails

**Document Created**: 2025-09-11  
**Last Updated**: 2025-09-11 - Added Stage 1 Comprehensive Dependency Caching
**Phase**: Phase 3 - Production Migration with Safety Rails  
**Prerequisites**: ✅ Phase 2 Complete (Vosk model caching, full CI validation)

---

## Executive Summary

**Objective**: Implement production-ready hybrid workflows with automatic fallback capabilities while maintaining zero regression in reliability and performance.

**Key Strategy**: Progressive rollout with safety rails, comprehensive monitoring, and automatic failover to GitHub-hosted runners when issues are detected.

---

## Current State Analysis

### ✅ Phase 2 Achievements (Baseline Established)
- **Critical Performance**: Vosk model setup optimized from 3h+ timeouts → 11s (165x+ improvement)
- **Reliability**: Eliminated 100% Vosk test failure rate through local caching
- **Infrastructure**: Runner operational with `[self-hosted, Linux, X64, fedora, nobara]` labels
- **Monitoring**: Performance monitoring scripts and health checks implemented
- **Caching**: 40MB+ models cached at `/home/coldaine/ActionRunnerCache/vosk-models/`

### ⚠️ Current Issues Requiring Investigation
- Recent workflow failures detected (all CI and Vosk Integration runs failing)
- Need root cause analysis before Phase 3 implementation
- Performance monitoring script has variable binding issues

---

## Phase 3 Implementation Strategy

### Stage 1: Comprehensive Dependency Caching (NEW - Priority 1)
**Timeline**: 3-5 days
**Objective**: Eliminate 500MB-1GB downloads per job through comprehensive caching
**Expected Impact**: 2-5 minutes faster per job, 60-80% network reduction

#### 1.1 System Package Pre-Installation (Day 1-2)
- **Pre-install all dnf packages**: alsa-lib-devel, gtk3-devel, xorg-x11-server-Xvfb, etc.
- **Eliminate 200-400MB downloads**: One-time runner setup
- **Update workflows**: Change from installation to validation
- **Test on development workflows**: Ensure package availability

#### 1.2 Enhanced Rust Toolchain Caching (Day 2-3)
- **Create persistent toolchain cache**: `/home/coldaine/ActionRunnerCache/rust-toolchains/`
- **Cache stable and MSRV 1.75**: Eliminate 250-500MB downloads
- **Include components**: rustfmt, clippy pre-cached
- **Update workflow integration**: Conditional cache usage

#### 1.3 Binary Library Pre-Processing (Day 3-4)
- **Pre-extract libvosk permanently**: Install to `/usr/local/lib/`
- **Eliminate extraction overhead**: Save 5-15 seconds per job
- **Simplify setup-coldvox action**: Validate instead of extract
- **Test library availability**: Verify across all workflows

#### 1.4 Performance Validation (Day 4-5)
- **Measure improvements**: Document before/after metrics
- **Validate cache hit rates**: Ensure caching works correctly
- **Monitor disk usage**: Implement cache cleanup policies
- **Create maintenance scripts**: Automated cache management

### Stage 2: Stabilization and Investigation (Priority 2)
**Timeline**: 1-2 days
**Objective**: Resolve any remaining workflow issues with enhanced performance

#### 2.1 Current Issue Resolution
- **Investigate CI failures**: With faster feedback loops from caching
- **Fix performance monitoring script**: Resolve unbound variable errors
- **Validate all caches**: Ensure Vosk models and new caches functional
- **Confirm workflow success**: Verify improved execution times

#### 2.2 Enhanced Monitoring Preparation
- **Update monitoring for cache metrics**: Track cache hit rates
- **Create performance dashboard**: Show time savings from caching
- **Establish new baseline**: Document post-caching performance

### Stage 3: Hybrid Matrix Implementation (Priority 3)  
**Timeline**: 2-3 days
**Objective**: Implement fallback strategy with safety rails

#### 3.1 Workflow Template Creation
```yaml
# .github/workflows/hybrid-ci-template.yml (new template)
strategy:
  matrix:
    runner-config:
      - runner: [self-hosted, Linux, X64, fedora]
        experimental: false
        cache-strategy: "local"
      - runner: ubuntu-latest  
        experimental: true
        cache-strategy: "download"
  fail-fast: false
  
jobs:
  build:
    runs-on: ${{ matrix.runner-config.runner }}
    continue-on-error: ${{ matrix.runner-config.experimental }}
    
    steps:
      - name: Conditional Vosk Model Setup
        if: contains(matrix.runner-config.runner, 'self-hosted')
        run: |
          # Use local cache
          ln -sf /home/coldaine/ActionRunnerCache/vosk-models/vosk-model-small-en-us-0.15 models/
      
      - name: Conditional Vosk Model Download
        if: contains(matrix.runner-config.runner, 'ubuntu-latest')
        # Download logic for GitHub-hosted runners
```

#### 3.2 Progressive Workflow Migration
1. **Documentation workflows** (lowest risk)
2. **Linting and formatting** (low risk)
3. **Unit tests** (medium risk) 
4. **Integration tests** (high risk)
5. **Release workflows** (highest risk - Phase 4)

### Stage 4: Production Migration (Priority 4)
**Timeline**: 3-5 days
**Objective**: Migrate critical workflows with continuous monitoring

#### 4.1 Gradual Rollout Plan
- **Day 1**: Update documentation workflows
- **Day 2**: Update ci.yml with hybrid strategy  
- **Day 3**: Update vosk-integration.yml with hybrid strategy
- **Day 4-5**: Monitor, tune, and validate stability

#### 4.2 Monitoring and Alerting Implementation
- **Real-time failure detection**: Immediate alerts for 100% self-hosted failures
- **Performance regression detection**: Alert if build times exceed baseline + 50%
- **Automatic escalation**: Email/notification system for critical failures
- **Dashboard creation**: Real-time visibility into runner health and job success rates

---

## Risk Mitigation Strategies

### High-Priority Risks

| Risk | Mitigation Strategy | Detection Method | Rollback Trigger |
|------|-------------------|------------------|------------------|
| **Self-hosted runner failure** | Hybrid matrix with ubuntu-latest fallback | Health check every 60s | 3 consecutive health check failures |
| **Performance regression** | Monitor build times, alert if >50% baseline increase | Performance monitoring script | Build time >2x baseline for 3 runs |
| **Workflow reliability** | fail-fast: false, continue-on-error for experimental | Job success rate monitoring | <80% success rate over 24h |
| **Disk space exhaustion** | Automated cleanup, disk usage monitoring | Health check disk usage alerts | >85% disk usage |

### Rollback Procedures

#### Immediate Rollback (Emergency)
```bash
# Emergency rollback script
git checkout main
git revert <hybrid-workflow-commits>
git push origin main
# All workflows automatically use ubuntu-latest
```

#### Gradual Rollback (Planned)
1. Update matrix strategy to prefer ubuntu-latest
2. Monitor for stability restoration  
3. Remove self-hosted from matrix entirely
4. Document lessons learned

---

## Success Criteria and Metrics

### Phase 3 Completion Criteria
- [ ] **Hybrid workflows operational**: CI and Vosk Integration running with both runners
- [ ] **Zero reliability regression**: Success rate ≥ baseline (pre-Phase 3)
- [ ] **Performance maintained**: Self-hosted builds ≤ baseline + 25%
- [ ] **Monitoring operational**: Real-time alerts and dashboards functional
- [ ] **Fallback validated**: Automatic failover to GitHub-hosted tested and working

### Key Performance Indicators (KPIs)
- **Build Success Rate**: Target ≥95% (current baseline TBD after stabilization)
- **Self-hosted Preference**: Target ≥70% of jobs run on self-hosted when available
- **Build Time Performance**: Self-hosted ≤ GitHub-hosted + 25%
- **Vosk Model Setup**: Maintain <15s setup time vs 3h+ baseline improvement
- **Runner Uptime**: Target ≥99% availability during business hours

---

## Implementation Checklist

### Stage 1: Comprehensive Dependency Caching (NEW - PRIORITY 1)
- [ ] Create system package installation script
- [ ] Pre-install all dnf packages on runner  
- [ ] Set up Rust toolchain cache directories
- [ ] Cache stable and MSRV 1.75 toolchains
- [ ] Pre-extract and install libvosk permanently
- [ ] Update workflows to validate instead of install
- [ ] Measure performance improvements (target: 2-5 min faster)
- [ ] Document cache usage and hit rates

### Stage 2: Stabilization
- [ ] Investigate and resolve current workflow failures
- [ ] Fix performance_monitor.sh variable binding issues
- [ ] Validate all caches (Vosk models, packages, toolchains)
- [ ] Establish new baseline metrics with caching
- [ ] Test runner health check functionality

### Stage 3-4: Hybrid Implementation & Production
- [ ] Create hybrid workflow templates
- [ ] Update documentation workflows with hybrid strategy
- [ ] Update ci.yml with hybrid matrix and fallback logic
- [ ] Update vosk-integration.yml with hybrid matrix
- [ ] Implement enhanced monitoring and alerting
- [ ] Create real-time dashboard for runner status

### Post-Implementation (Validation)
- [ ] Monitor hybrid workflows for 48h minimum
- [ ] Validate fallback mechanism through controlled failure test
- [ ] Document performance improvements and any regressions
- [ ] Update team documentation with new workflow behavior
- [ ] Plan Phase 4 security hardening and release workflow migration

---

## Technical Implementation Details

### Monitoring Script Enhancements
```bash
# Enhanced monitoring with proper error handling
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
```

### Alert Integration
```yaml
# .github/workflows/runner-health-alert.yml
name: Runner Health Alert
on:
  schedule:
    - cron: '*/5 * * * *'  # Every 5 minutes
  workflow_dispatch:

jobs:
  health-check:
    runs-on: [self-hosted, Linux, X64, fedora]
    steps:
      - name: Check Runner Health
        run: |
          if ! ./scripts/performance_monitor.sh health; then
            # Send alert via GitHub Issues API or external service
            echo "ALERT: Self-hosted runner health check failed"
            exit 1
          fi
```

---

## Next Actions

### Immediate Priority - Comprehensive Caching (This Week)
1. **Day 1-2**: System package pre-installation (eliminate 200-400MB downloads)
2. **Day 2-3**: Rust toolchain caching (eliminate 250-500MB downloads)
3. **Day 3-4**: Binary library pre-processing (libvosk permanent installation)
4. **Day 4-5**: Performance validation and metrics documentation

**Expected Results**: 2-5 minutes faster per job, 60-80% network reduction

### Following Week - Hybrid Strategy Implementation
1. **Stabilization**: Resolve any remaining workflow issues with new performance baseline
2. **Template Creation**: Implement hybrid workflow templates with fallback
3. **Progressive Rollout**: Start with low-risk workflows, progress to critical
4. **Monitoring Setup**: Enhanced monitoring with cache metrics and alerts
5. **Validation**: Comprehensive testing of fallback mechanisms

---

## Phase 4: Advanced Runner Optimization (Future)

### Parallel Job Execution Implementation
**Timeline**: 2-3 days  
**Objective**: Maximize hardware utilization through concurrent job execution
**Expected Impact**: 3-4x throughput improvement, reduced queue times

#### Hardware Capacity Analysis
- **CPU**: 10-core i7-1365U with 12 threads (hyperthreading)
- **Memory**: 30GB RAM
- **Storage**: 238GB NVMe SSD
- **Theoretical Capacity**: 4-5 concurrent jobs optimal, 6+ possible for lightweight jobs

#### Implementation Options

##### Option A: Single Runner with Concurrent Jobs (Recommended)
**Approach**: Configure existing runner for parallel execution
```bash
# Add to runner environment
echo "ACTIONS_RUNNER_CONCURRENT_JOBS=4" >> /home/coldaine/actions-runner/.env
systemctl restart actions-runner
```

**Pros**:
- Simple configuration
- Uses existing runner setup
- Automatic job distribution by GitHub

**Cons**:
- Less granular resource control
- All jobs share same environment

##### Option B: Multiple Runner Instances (Advanced)
**Approach**: Register 3-4 separate runner instances with resource allocation
```bash
# Runner 1: Primary builder (6 cores, 16GB RAM)
./config.sh --name coldaine-builder --labels self-hosted,Linux,X64,fedora,nobara,heavy-build

# Runner 2: Test runner (2 cores, 8GB RAM) 
./config.sh --name coldaine-tester --labels self-hosted,Linux,X64,fedora,nobara,light-test

# Runner 3: Linting/docs (2 cores, 4GB RAM)
./config.sh --name coldaine-lint --labels self-hosted,Linux,X64,fedora,nobara,fast-lint
```

**Pros**:
- Fine-grained resource allocation
- Specialized runner configurations
- Better isolation between job types

**Cons**:
- More complex setup and maintenance
- Requires workflow label updates

##### Option C: Hybrid Resource-Aware Scheduling (Optimal)
**Approach**: Combine single runner with resource-aware job classification

```yaml
# Heavy build jobs (Rust compilation, integration tests)
runs-on: [self-hosted, Linux, X64, fedora, nobara, heavy-build]
env:
  CARGO_BUILD_JOBS: 6  # Use 6 cores for compilation

# Light jobs (formatting, linting, documentation)  
runs-on: [self-hosted, Linux, X64, fedora, nobara, light-job]
env:
  CARGO_BUILD_JOBS: 1  # Use 1 core for lightweight tasks
```

#### Resource Allocation Strategy

| Job Type | CPU Cores | RAM | Concurrent Limit | Examples |
|----------|-----------|-----|------------------|----------|
| **Heavy Build** | 6 cores | 12GB | 1 concurrent | Rust compilation, integration tests |
| **Medium Test** | 3 cores | 6GB | 2 concurrent | Unit tests, STT tests |
| **Light Tasks** | 1 core | 2GB | 4 concurrent | Linting, formatting, docs |

#### Performance Monitoring Enhancements
```bash
# Enhanced monitoring for parallel execution
./scripts/performance_monitor.sh --parallel-mode --job-tracking
```

**Metrics to Track**:
- Per-job resource usage (CPU, memory, I/O)
- Queue time vs execution time
- Resource contention detection
- Cache hit rates across concurrent jobs

#### Implementation Phases

**Phase 4.1: Basic Parallel Setup (Day 1)**
- Configure `ACTIONS_RUNNER_CONCURRENT_JOBS=3`
- Test with low-risk workflows (documentation, linting)
- Monitor resource usage and adjust

**Phase 4.2: Resource Classification (Day 2)**
- Add resource-aware labels to workflows
- Implement job-specific resource limits
- Validate no resource contention

**Phase 4.3: Optimization & Tuning (Day 3)**  
- Fine-tune concurrent job limits based on monitoring
- Optimize cache sharing across concurrent jobs
- Document optimal configuration

#### Expected Performance Improvements

**Current State**:
- Single job execution: ~8-12 minutes per CI run
- Queue serialization: Jobs wait for completion

**With Parallel Execution**:
- 3-4 concurrent light jobs: ~3-4 minutes total
- Heavy + light job mixing: ~6-8 minutes total  
- **Overall throughput**: 3-4x improvement

#### Risk Mitigation

| Risk | Mitigation | Detection |
|------|------------|-----------|
| **Resource exhaustion** | Conservative initial limits, monitoring | Memory/CPU alerts |
| **Cache conflicts** | Job-specific cache keys, file locking | Build failures |
| **I/O contention** | SSD monitoring, staggered heavy jobs | Disk usage spikes |

#### Success Criteria
- [ ] 3+ concurrent jobs executing successfully  
- [ ] No resource-related build failures
- [ ] 2-3x throughput improvement measured
- [ ] Cache hit rates maintained across parallel jobs
- [ ] System stability under concurrent load

**Document Status**: Living document - will be updated as implementation progresses