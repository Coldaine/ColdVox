# Phase 3 Tactical Implementation Plan
## Self-Hosted Runner Production Migration with Safety Rails

**Document Created**: 2025-09-11  
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

### Stage 1: Stabilization and Investigation (Priority 1)
**Timeline**: 1-2 days
**Objective**: Resolve current workflow failures and ensure stable baseline

#### 1.1 Current Issue Resolution
- **Investigate recent CI failures**: Analyze failed runs from past 24 hours
- **Fix performance monitoring script**: Resolve unbound variable errors
- **Validate Vosk model cache integrity**: Ensure cached models are functional
- **Test single workflow success**: Confirm at least one successful CI run

#### 1.2 Enhanced Monitoring Preparation
- **Fix performance_monitor.sh script**: Address variable binding issues
- **Create alerting mechanisms**: Real-time failure detection
- **Establish baseline metrics**: Document current successful run characteristics

### Stage 2: Hybrid Matrix Implementation (Priority 2)  
**Timeline**: 2-3 days
**Objective**: Implement fallback strategy with safety rails

#### 2.1 Workflow Template Creation
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

#### 2.2 Progressive Workflow Migration
1. **Documentation workflows** (lowest risk)
2. **Linting and formatting** (low risk)
3. **Unit tests** (medium risk) 
4. **Integration tests** (high risk)
5. **Release workflows** (highest risk - Phase 4)

### Stage 3: Production Migration (Priority 3)
**Timeline**: 3-5 days
**Objective**: Migrate critical workflows with continuous monitoring

#### 3.1 Gradual Rollout Plan
- **Day 1**: Update documentation workflows
- **Day 2**: Update ci.yml with hybrid strategy  
- **Day 3**: Update vosk-integration.yml with hybrid strategy
- **Day 4-5**: Monitor, tune, and validate stability

#### 3.2 Monitoring and Alerting Implementation
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

### Pre-Implementation (Stage 1)
- [ ] Investigate and resolve current workflow failures
- [ ] Fix performance_monitor.sh variable binding issues
- [ ] Validate Vosk model cache integrity and accessibility
- [ ] Establish baseline metrics from successful runs
- [ ] Test runner health check functionality

### Implementation (Stage 2-3)
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

### Immediate (This Week)
1. **Stabilization Phase**: Investigate and resolve current workflow failures
2. **Script Fix**: Fix performance_monitor.sh variable binding issues  
3. **Validation**: Ensure at least one successful CI run before proceeding
4. **Planning**: Finalize hybrid workflow templates and test strategy

### Phase 3 Implementation (Next Week)  
1. **Template Creation**: Implement hybrid workflow templates
2. **Progressive Rollout**: Start with documentation workflows
3. **Monitoring Setup**: Implement enhanced monitoring and alerting
4. **Validation**: Comprehensive testing of fallback mechanisms

**Document Status**: Living document - will be updated as implementation progresses