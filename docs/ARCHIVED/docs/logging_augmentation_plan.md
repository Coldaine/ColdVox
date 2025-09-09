# Logging Augmentation Plan for ColdVox

## Overview

This document outlines the current logging gaps in the ColdVox audio pipeline and proposes a systematic approach to enhance logging for better debugging, monitoring, and troubleshooting. The plan focuses on filling critical gaps in the VAD-to-STT pipeline while maintaining performance and following existing logging patterns.

## Current Logging Status

### Existing Logging Infrastructure
- **Framework**: Uses `tracing` crate with structured logging
- **Output**: Dual output to stderr and daily-rotated file (`logs/coldvox.log`)
- **Control**: `RUST_LOG` environment variable and `--log-level` CLI flag
- **Levels**: ERROR, WARN, INFO, DEBUG, TRACE

### Current Log Coverage
- ✅ **Audio Capture**: Device detection, stream start, configuration
- ✅ **Audio Processing**: Chunker start, resampler configuration
- ✅ **STT Initialization**: Processor start, model loading
- ✅ **Hotkey System**: Backend selection, shortcut registration
- ✅ **Application Lifecycle**: Startup, shutdown, error handling
- ⚠️ **VAD Processing**: Partial coverage (startup logs missing)
- ❌ **Event Routing**: No logs for VAD event forwarding
- ❌ **Pipeline Health**: Limited visibility into component connectivity

## Identified Gaps

### 1. VAD Processor Startup & Health
**Current State**: VAD processor logs when `run()` starts, but spawn success/failure isn't confirmed
**Impact**: Cannot distinguish between spawn failure vs. silent non-processing
**Location**: `crates/app/src/audio/vad_processor.rs`, `crates/app/src/runtime.rs`

### 2. Event Fanout & Routing
**Current State**: Fanout task has zero logging
**Impact**: Cannot see if events are generated but not routed, or if routing fails silently
**Location**: `crates/app/src/runtime.rs:283-295`

### 3. VAD Event Generation
**Current State**: Logs speech start/end events, but not frame processing confirmation
**Impact**: Cannot distinguish between no audio frames vs. no speech detection
**Location**: `crates/app/src/audio/vad_processor.rs:57-128`

### 4. STT Event Reception
**Current State**: Only DEBUG logs for SpeechStart (not visible in reviewed logs)
**Impact**: Cannot see if STT receives events but doesn't process them
**Location**: `crates/app/src/stt/processor.rs:208-217`

### 5. Broadcast Channel Health
**Current State**: No logs about broadcast receiver connections
**Impact**: Silent failures if channels aren't properly connected
**Location**: Multiple files using `broadcast` channels

## Proposed Log Enhancements

### Phase 1: Critical Pipeline Visibility (High Priority)

#### 1.1 VAD Processor Lifecycle
**File**: `crates/app/src/runtime.rs`
**Location**: After `VadProcessor::spawn()` calls
**Proposed Logs**:
```rust
tracing::info!("VAD processor spawned successfully");
tracing::error!("Failed to spawn VAD processor: {}", e);
```

**File**: `crates/app/src/audio/vad_processor.rs`
**Location**: In `spawn()` function
**Proposed Logs**:
```rust
tracing::info!("VAD processor task spawned for mode: {:?}", config.mode);
```

#### 1.2 Event Fanout Monitoring
**File**: `crates/app/src/runtime.rs`
**Location**: Fanout task (lines 283-295)
**Proposed Logs**:
```rust
tracing::debug!("Fanout: Received VAD event: {:?}", ev);
tracing::debug!("Fanout: Forwarded to broadcast channel");
tracing::debug!("Fanout: Forwarded to STT channel");
tracing::warn!("Fanout: Failed to send to broadcast: {}", e);
tracing::warn!("Fanout: Failed to send to STT: {}", e);
```

#### 1.3 VAD Frame Processing Confirmation
**File**: `crates/app/src/audio/vad_processor.rs`
**Location**: `process_frame()` function
**Proposed Logs**:
```rust
if self.frames_processed % 100 == 0 {
    tracing::debug!("VAD: Received {} frames, processing active", self.frames_processed);
}
```

#### 1.4 STT Event Reception
**File**: `crates/app/src/stt/processor.rs`
**Location**: VAD event handling (lines 208-217)
**Proposed Logs**:
```rust
tracing::info!("STT: Received VAD event: {:?}", event);
```

### Phase 2: Enhanced Diagnostics (Medium Priority)

#### 2.1 Broadcast Channel Health
**File**: Multiple files
**Proposed Logs**:
```rust
tracing::debug!("Broadcast channel: {} subscribers connected", tx.receiver_count());
```

#### 2.2 Pipeline Component Status
**File**: `crates/app/src/runtime.rs`
**Proposed Logs**:
```rust
tracing::info!("Audio pipeline components initialized: capture={}, chunker={}, vad={}, stt={}",
    capture_handle.is_some(), chunker_handle.is_some(), vad_handle.is_some(), stt_handle.is_some());
```

#### 2.3 Audio Frame Flow
**File**: `crates/coldvox-audio/src/chunker.rs`
**Proposed Logs**:
```rust
if frames_sent % 1000 == 0 {
    tracing::debug!("Chunker: Sent {} frames to {} subscribers", frames_sent, audio_tx.receiver_count());
}
```

### Phase 3: Performance & Metrics (Low Priority)

#### 3.1 Processing Latency
**File**: `crates/app/src/audio/vad_processor.rs`
**Proposed Logs**:
```rust
let processing_time = start_time.elapsed();
tracing::trace!("VAD frame processing time: {:?}", processing_time);
```

#### 3.2 Memory Usage
**File**: `crates/app/src/stt/processor.rs`
**Proposed Logs**:
```rust
tracing::debug!("STT buffer utilization: {}%", buffer_utilization_pct);
```

## Implementation Guidelines

### Log Level Strategy
- **ERROR**: Component failures, data loss
- **WARN**: Recovery actions, degraded performance
- **INFO**: Component lifecycle, major state changes
- **DEBUG**: Event flow, periodic status, troubleshooting
- **TRACE**: Detailed frame-by-frame processing

### Performance Considerations
- Use conditional logging for high-frequency operations
- Avoid string formatting in hot paths
- Use structured logging with key-value pairs
- Consider log sampling for very frequent events

### Testing Strategy
- Verify logs appear at correct levels
- Test log rotation and file management
- Validate log parsing for monitoring tools
- Ensure no performance regression

## Expected Benefits

### Debugging Improvements
- **Root Cause Analysis**: Clear visibility into where pipeline breaks
- **Event Tracing**: Follow VAD events from generation to consumption
- **Component Isolation**: Identify which component is failing silently

### Monitoring Enhancements
- **Health Checks**: Automated detection of pipeline issues
- **Performance Tracking**: Identify bottlenecks and optimization opportunities
- **Alerting**: Better triggers for operational issues

### Development Efficiency
- **Faster Troubleshooting**: Reduce time to identify issues
- **Better Testing**: More comprehensive test coverage with log validation
- **Code Reviews**: Clearer understanding of component interactions

## Rollout Plan

### Phase 1 Rollout (Week 1)
1. Implement VAD processor lifecycle logs
2. Add event fanout monitoring
3. Deploy and validate in development environment

### Phase 2 Rollout (Week 2)
1. Add broadcast channel health monitoring
2. Implement pipeline component status logs
3. Update documentation with new log patterns

### Phase 3 Rollout (Week 3)
1. Add performance and metrics logging
2. Optimize log levels based on production usage
3. Create log analysis guidelines

## Success Metrics

- **Pipeline Visibility**: 100% coverage of critical event flows
- **Debugging Time**: 50% reduction in time to identify issues
- **False Positives**: <5% of logs at WARN/ERROR level
- **Performance Impact**: <1% overhead on CPU and memory

## Maintenance

- **Review Cadence**: Quarterly review of log effectiveness
- **Cleanup**: Remove or demote logs that prove unhelpful
- **Documentation**: Keep this plan updated with implemented changes
- **Training**: Ensure team understands new logging patterns

## Related Documents

- `docs/troubleshooting.md` - Updated with log-based debugging procedures
- `docs/monitoring.md` - Log analysis and alerting guidelines
- `CLAUDE.md` - Updated logging section with new patterns</result>
