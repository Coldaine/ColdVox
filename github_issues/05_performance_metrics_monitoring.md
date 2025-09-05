# [Telemetry] Implement comprehensive STT performance metrics and monitoring

**Priority:** Medium

## Problem Description
The telemetry system exists but lacks STT-specific performance metrics, making it difficult to monitor transcription performance, latency, accuracy, and resource usage. This hinders optimization efforts and troubleshooting of performance issues.

## Impact
- **Medium**: Difficult to monitor and optimize STT performance
- Lack of visibility into transcription latency and accuracy
- Hard to identify performance bottlenecks
- No historical performance data for trend analysis
- Limited debugging capabilities for performance issues

## Reproduction Steps
1. Examine `crates/app/src/telemetry/mod.rs` - check existing metrics
2. Look for STT-specific performance measurements
3. Test transcription latency monitoring
4. Check for accuracy tracking capabilities
5. Verify resource usage monitoring for STT components

## Expected Behavior
The telemetry system should provide:
- Real-time transcription latency metrics
- STT accuracy measurements and tracking
- Resource usage monitoring (CPU, memory) for STT
- Performance bottleneck identification
- Historical performance data and trends
- Alerting capabilities for performance degradation

## Current Behavior
The telemetry system lacks:
- STT-specific performance metrics
- Transcription latency monitoring
- Accuracy measurement capabilities
- Resource usage tracking for STT components
- Performance alerting and anomaly detection

## Proposed Solution
1. Add STT-specific metrics to telemetry system
2. Implement transcription latency monitoring
3. Add accuracy measurement and tracking
4. Create performance dashboards and alerts
5. Implement resource usage monitoring for STT

## Implementation Steps
1. Define STT performance metrics schema
2. Add latency measurement points in transcription pipeline
3. Implement accuracy tracking mechanisms
4. Create resource usage monitoring for STT components
5. Add performance alerting and anomaly detection
6. Implement metrics collection and storage

## Acceptance Criteria
- [ ] STT transcription latency metrics implemented
- [ ] Accuracy measurement and tracking capabilities
- [ ] Resource usage monitoring for STT components
- [ ] Performance alerting for degradation detection
- [ ] Historical performance data collection
- [ ] Performance dashboards and visualization

## Key Metrics to Implement
- **Latency Metrics**:
  - End-to-end transcription latency
  - STT engine processing time
  - Audio preprocessing latency
  - Result delivery latency

- **Accuracy Metrics**:
  - Word error rate (WER)
  - Character error rate (CER)
  - Confidence score tracking
  - Transcription success rate

- **Resource Metrics**:
  - CPU usage by STT components
  - Memory usage patterns
  - Model loading time
  - Audio buffer utilization

- **Operational Metrics**:
  - Transcription request rate
  - Error rate by component
  - Fallback mechanism usage
  - Model switching frequency

## Related Files
- `crates/app/src/telemetry/mod.rs`
- `crates/coldvox-telemetry/src/lib.rs`
- `crates/app/src/stt/processor.rs`
- `crates/coldvox-stt-vosk/src/vosk_transcriber.rs`
