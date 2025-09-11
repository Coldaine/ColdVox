# Self-Hosted GitHub Actions Runner Migration Strategy

## Executive Summary

**Objective**: Complete migration of ColdVox GitHub Actions workflows from `ubuntu-latest` to self-hosted Fedora runner on `laptop-extra`.

**Current Status**: ✅ **Phase 1 Complete** - Basic runner functionality validated
- Runner connectivity and job execution confirmed
- Rust toolchain compatibility issue identified and resolved
- System dependencies mostly available
- Test workflow executing successfully on local machine

**Next Phase**: Full CI pipeline validation and production migration with risk mitigation.

---

## Current Assessment

### ✅ Validated Capabilities
- **Runner Registration**: `laptop-extra` with labels `[self-hosted, Linux, X64]`
- **Job Execution**: Confirmed local execution via runner logs analysis
- **Basic Dependencies**: Core packages (alsa-lib-devel, xdotool, libXtst-devel, etc.) installed
- **Toolchain Setup**: `actions-rust-lang/setup-rust-toolchain@v1` resolves Cargo.lock format issues
- **Workspace Operations**: File system access and workspace management functional

### ❌ Untested Critical Areas
- **Full CI Pipeline**: Complex builds, multi-step workflows, parallel jobs
- **Text Injection Backends**: AT-SPI, clipboard, ydotool integration on Fedora
- **Performance Characteristics**: Build times, resource usage vs GitHub-hosted
- **Error Recovery**: Network failures, disk space, process cleanup
- **Security Isolation**: Secret handling, workspace cleanup, network access

### 🔧 Current Configuration Gaps
- **Runner Labels**: Missing `fedora`/`nobara` specific labels for targeted workflows
- **Fallback Strategy**: No automatic failover to GitHub-hosted runners
- **Monitoring**: No health checks or alerting for runner availability
- **Maintenance**: No systematic update or cleanup procedures

---

## Risk Analysis & Mitigation

| Risk | Impact | Likelihood | Mitigation Strategy |
|------|---------|-----------|-------------------|
| **Runner Hardware Failure** | HIGH - Blocks all CI/CD | MEDIUM | Implement hybrid matrix strategy with GitHub-hosted fallback |
| **Network Connectivity Loss** | HIGH - Job timeouts/failures | MEDIUM | Configure aggressive retry policies, network redundancy |
| **Fedora Package Incompatibility** | MEDIUM - Build failures | HIGH | Maintain cross-platform package management in workflows |
| **Performance Regression** | MEDIUM - Slower builds | HIGH | Establish baseline metrics, optimize resource allocation |
| **Security Compromise** | CRITICAL - Code/secrets exposure | LOW | Implement workspace isolation, secret handling protocols |
| **Disk Space Exhaustion** | MEDIUM - Job failures | MEDIUM | Automated cleanup policies, monitoring with alerts |

---

## Migration Strategy: Phase-by-Phase Approach

### Phase 2: Full Pipeline Validation (Current Priority)
**Timeline**: 1-2 days  
**Objective**: Validate complete CI/CD functionality

#### 2.1 Runner Enhancement
```bash
# Add Fedora-specific labels for better targeting
./config.sh --labels self-hosted,Linux,X64,fedora,nobara
```

#### 2.2 Comprehensive CI Testing
- **Trigger full CI workflow** on test branch
- **Validate all job types**: build, test, security audit, text injection tests
- **Monitor resource usage**: CPU, memory, disk, network
- **Document failure points** and resolution strategies

#### 2.3 Performance Baseline
- **Measure build times** vs GitHub-hosted equivalents
- **Analyze resource utilization** patterns
- **Identify optimization opportunities**

### Phase 3: Production Migration with Safety Rails
**Timeline**: 3-5 days  
**Objective**: Gradual production rollout with fallback capabilities

#### 3.1 Hybrid Workflow Strategy
Implement matrix strategy for critical workflows:
```yaml
strategy:
  matrix:
    runner:
      - [self-hosted, Linux, X64, fedora]  # Preferred
      - ubuntu-latest                      # Fallback
    include:
      - runner: [self-hosted, Linux, X64, fedora]
        experimental: false
      - runner: ubuntu-latest
        experimental: true
  fail-fast: false
```

#### 3.2 Gradual Workflow Migration
1. **Non-critical workflows first** (documentation, linting)
2. **CI pipeline with fallback** (build, test with hybrid matrix)
3. **Release workflows** (only after proven stability)

#### 3.3 Monitoring & Alerting
- **Runner health monitoring** (uptime, resource usage)
- **Job failure pattern analysis**
- **Performance tracking** and regression detection

### Phase 4: Optimization & Hardening
**Timeline**: Ongoing  
**Objective**: Long-term stability and performance

#### 4.1 Security Hardening
- **Workspace isolation** strategies
- **Secret handling** verification
- **Network access** controls
- **Automated security updates**

#### 4.2 Performance Optimization
- **Cargo cache** optimization
- **Parallel build** configuration
- **Resource allocation** tuning

#### 4.3 Maintenance Automation
- **Automated system updates**
- **Workspace cleanup** scheduling
- **Health monitoring** with alerts

---

## Technical Implementation Details

### Runner Label Configuration
```bash
cd /home/coldaine/actions-runner
sudo ./svc.sh stop
./config.sh remove --token <TOKEN>
./config.sh --url https://github.com/Coldaine/ColdVox \
           --token <TOKEN> \
           --name laptop-extra \
           --labels self-hosted,Linux,X64,fedora,nobara
sudo ./svc.sh install
sudo ./svc.sh start
```

### Workflow Updates Required

#### Primary CI Workflow (ci.yml)
- ✅ **Rust toolchain setup added** to critical jobs
- ✅ **System dependency management** updated for cross-platform
- ⏳ **Add fallback matrix strategy** for reliability

#### Recommended Workflow Template
```yaml
jobs:
  build:
    strategy:
      matrix:
        runner: 
          - [self-hosted, Linux, X64, fedora]
          - ubuntu-latest
      fail-fast: false
    runs-on: ${{ matrix.runner }}
    steps:
      - uses: actions/checkout@v4
      
      # Rust toolchain - critical for self-hosted
      - name: Setup Rust toolchain
        if: contains(matrix.runner, 'self-hosted')
        uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          toolchain: stable
          components: rustfmt, clippy
          override: true
      
      # Cross-platform dependency management
      - name: Setup ColdVox
        uses: ./.github/actions/setup-coldvox
        with:
          skip-toolchain: ${{ contains(matrix.runner, 'self-hosted') && 'true' || 'false' }}
```

### Monitoring Strategy

#### Health Check Script
```bash
#!/bin/bash
# ~/.local/bin/runner-health-check.sh
RUNNER_PID=$(pgrep -f "Runner.Listener")
if [ -z "$RUNNER_PID" ]; then
    echo "CRITICAL: Runner not running"
    # Restart runner service
    sudo systemctl restart actions.runner.*
    exit 2
fi

# Check disk space
DISK_USAGE=$(df /home/coldaine/actions-runner/_work | awk 'NR==2 {print $5}' | sed 's/%//')
if [ "$DISK_USAGE" -gt 80 ]; then
    echo "WARNING: Disk usage high: ${DISK_USAGE}%"
    # Cleanup old workspaces
    find /home/coldaine/actions-runner/_work -type d -name "ColdVox" -mtime +7 -exec rm -rf {} \;
fi

echo "OK: Runner healthy, disk usage: ${DISK_USAGE}%"
```

---

## Success Criteria

### Phase 2 Completion
- [ ] Full CI pipeline executes successfully on self-hosted runner
- [ ] All system dependencies resolve correctly
- [ ] Performance baseline established (build times documented)
- [ ] Error handling validated (network failures, timeouts)

### Phase 3 Completion  
- [ ] Production workflows migrated with fallback strategy
- [ ] No increase in workflow failure rate
- [ ] Performance meets or exceeds GitHub-hosted baseline
- [ ] Monitoring and alerting operational

### Phase 4 Completion
- [ ] Security hardening implemented and verified
- [ ] Automated maintenance procedures established
- [ ] Documentation updated for team knowledge sharing
- [ ] Long-term sustainability plan documented

---

## Rollback Strategy

### Immediate Rollback (Emergency)
1. **Revert workflow files** to use `ubuntu-latest`
2. **Push changes** to stop using self-hosted runner
3. **Monitor** for restoration of normal operation

### Planned Rollback
1. **Update workflows** to remove self-hosted runner from matrix
2. **Gracefully drain** existing jobs
3. **Document lessons learned** and issues encountered
4. **Preserve runner setup** for future retry

---

## Next Actions (Immediate)

### This Week
1. **Add runner labels** for better targeting
2. **Trigger comprehensive CI test** on current branch
3. **Document performance baseline** and failure points
4. **Implement basic monitoring** script

### Next Week  
1. **Implement hybrid fallback strategy** in critical workflows
2. **Begin gradual production migration** starting with non-critical workflows
3. **Establish monitoring and alerting** procedures

---

## Decision Points

### Go/No-Go Criteria for Production Migration
- ✅ Full CI pipeline passes consistently (3/3 runs)
- ✅ Performance equal or better than GitHub-hosted
- ✅ All critical dependencies functional
- ✅ Fallback strategy tested and verified

### Long-term Viability Assessment  
- **Cost-benefit analysis** (maintenance effort vs infrastructure savings)
- **Performance sustainability** under varying loads
- **Security posture** meeting organizational requirements
- **Team capability** for ongoing maintenance

---

*Document Status: Draft v1.0*  
*Branch: `fedora-runner-test`*  
*Author: Claude Code Migration Assistant*  
*Date: 2025-09-09*