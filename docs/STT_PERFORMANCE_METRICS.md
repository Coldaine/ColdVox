# STT Performance Metrics and Monitoring

This document describes the comprehensive STT performance metrics and monitoring system implemented for ColdVox.

## Overview

The STT performance metrics system provides comprehensive monitoring of speech-to-text operations, including:

- **Latency Tracking**: End-to-end, engine processing, preprocessing, and result delivery times
- **Accuracy Monitoring**: Confidence scores, success rates, and transcription quality metrics  
- **Resource Usage**: Memory consumption, buffer utilization, and processing resource tracking
- **Operational Metrics**: Request rates, error rates, and processing throughput
- **Performance Alerts**: Configurable thresholds with automated alert detection
- **Trend Analysis**: Historical performance tracking and degradation detection

## Quick Start

### Basic Integration

```rust
use coldvox_telemetry::SttMetricsBuilder;

// Create metrics manager with production defaults
let metrics_manager = SttMetricsBuilder::production().build();

// Get metrics instance for STT processor
let performance_metrics = metrics_manager.metrics();

// Set on your STT processor
stt_processor.set_performance_metrics(performance_metrics);
```

### Custom Configuration

```rust
let metrics_manager = SttMetricsBuilder::new()
    .with_max_latency(200)        // 200ms max latency
    .with_min_confidence(0.85)    // 85% min confidence
    .with_max_memory_mb(256)      // 256MB max memory  
    .with_max_error_rate(20)      // 2% max error rate
    .build();
```

### Monitoring and Alerts

```rust
// Check for performance alerts
let alerts = metrics_manager.check_alerts();
if !alerts.is_empty() {
    warn!("STT performance alerts detected: {:?}", alerts);
}

// Get performance summary
let summary = metrics_manager.get_performance_summary();
info!("STT latency: {:.1}ms, confidence: {:.1}%", 
      summary.avg_latency_ms, summary.avg_confidence * 100.0);

// Get formatted report
info!("{}", metrics_manager.format_metrics_report());
```

## Configuration Presets

The system provides several pre-configured setups for common use cases:

### Production Configuration
- Max latency: 500ms
- Min confidence: 75%
- Max error rate: 5%
- Max memory: 512MB

```rust
let manager = SttMetricsBuilder::production().build();
```

### Development Configuration  
- Max latency: 1000ms
- Min confidence: 60%
- Max error rate: 10%
- Max memory: 1GB

```rust
let manager = SttMetricsBuilder::development().build();
```

### High Performance Configuration
- Max latency: 100ms
- Min confidence: 90%
- Max error rate: 1%
- Max memory: 256MB

```rust
let manager = SttMetricsBuilder::high_performance().build();
```

## Metrics Details

### Latency Metrics

| Metric | Description | Unit |
|--------|-------------|------|
| End-to-End | Complete transcription pipeline latency | microseconds |
| Engine Processing | STT engine processing time only | microseconds |
| Preprocessing | Audio preprocessing latency | microseconds |
| Result Delivery | Time to deliver results | microseconds |
| Model Loading | Time to load STT model | milliseconds |

### Accuracy Metrics

| Metric | Description | Range |
|--------|-------------|-------|
| Average Confidence | Mean confidence score across transcriptions | 0.0-1.0 |
| Success Rate | Percentage of successful transcriptions | 0.0-1.0 |
| Partial Count | Number of partial transcription results | count |
| Final Count | Number of final transcription results | count |
| Error Count | Number of transcription errors | count |

### Resource Metrics

| Metric | Description | Unit |
|--------|-------------|------|
| Memory Usage | Current memory consumption | bytes |
| Peak Memory | Maximum memory usage observed | bytes |
| Buffer Utilization | Audio buffer usage percentage | 0-100% |
| Active Threads | Number of processing threads | count |

### Operational Metrics

| Metric | Description | Unit |
|--------|-------------|------|
| Request Rate | Transcription requests per second | requests/sec |
| Error Rate | Errors per 1000 operations | errors/1k |
| Processing FPS | Audio frames processed per second | frames/sec |
| Queue Depth | Average processing queue depth | count |

## Performance Alerts

The system can automatically detect performance issues:

### Alert Types

- **High Latency**: When processing time exceeds threshold
- **Low Confidence**: When transcription quality drops
- **High Error Rate**: When failure rate increases
- **High Memory Usage**: When memory consumption is excessive
- **Processing Stalled**: When processing stops responding

### Alert Configuration

```rust
use coldvox_telemetry::PerformanceThresholds;

let thresholds = PerformanceThresholds {
    max_latency_us: 200_000,      // 200ms
    min_confidence: 0.8,          // 80%
    max_error_rate_per_1k: 50,    // 5%
    max_memory_bytes: 512 * 1024 * 1024, // 512MB
};

let manager = SttMetricsBuilder::new()
    .with_thresholds(thresholds)
    .build();
```

## Trend Analysis

The system tracks performance trends over time:

```rust
match metrics_manager.get_latency_trend() {
    Some(LatencyTrend::Increasing) => warn!("Latency is increasing"),
    Some(LatencyTrend::Decreasing) => info!("Latency is improving"),
    Some(LatencyTrend::Stable) => info!("Latency is stable"),
    None => info!("Insufficient data for trend analysis"),
}
```

## Integration with Existing Code

The metrics system is designed for minimal integration impact:

### STT Processor Integration

1. **Create metrics manager** during application startup
2. **Set performance metrics** on STT processor instance  
3. **Metrics are automatically collected** during transcription
4. **Monitor periodically** for alerts and performance reports

### Automatic Instrumentation

The enhanced STT processor automatically records:

- Timing measurements at each processing stage
- Confidence scores from transcription results
- Memory usage estimation during processing
- Error tracking with performance impact
- Resource utilization monitoring

### Backward Compatibility

- Existing `SttMetrics` structure extended with new fields
- New performance metrics are optional (can be `None`)
- All existing functionality preserved
- No breaking changes to existing APIs

## Testing

The system includes comprehensive tests:

```bash
# Run all telemetry tests
cd crates/coldvox-telemetry
cargo test

# Run specific STT metrics tests  
cargo test stt_metrics

# Run integration tests
cargo test integration
```

## Examples

See the `/examples` directory for complete demonstrations:

- `demo.rs` - Comprehensive metrics functionality demo
- `integration_example.rs` - Simple integration example

```bash
# Run the comprehensive demo
cd crates/coldvox-telemetry
cargo run --example demo

# Run the integration example
cargo run --example integration_example
```

## Performance Impact

The metrics system is designed for minimal performance overhead:

- **Atomic operations** for thread-safe metric updates
- **Lock-free counters** for high-frequency metrics
- **Bounded memory usage** with historical data limits
- **Optional components** can be disabled if not needed
- **Zero-allocation paths** for critical performance metrics

## Roadmap

Future enhancements planned:

- Export metrics to external monitoring systems (Prometheus, etc.)
- Real-time dashboard for metrics visualization
- Advanced alerting with configurable notification channels
- Machine learning-based anomaly detection
- Integration with distributed tracing systems

## Troubleshooting

### Common Issues

1. **High memory usage**: Check buffer utilization and consider reducing buffer sizes
2. **Performance alerts**: Review thresholds and system resources
3. **Missing metrics**: Ensure performance metrics are set on STT processor
4. **Trend analysis unavailable**: Requires at least 10 data points

### Debug Information

Enable debug logging to see detailed metrics collection:

```rust
tracing::info!("{}", metrics_manager.format_metrics_report());
```

## Architecture

The metrics system consists of:

- **`stt_metrics.rs`**: Core metrics structures and functionality
- **`integration.rs`**: Easy-to-use integration helpers and builders
- **Enhanced STT processor**: Automatic instrumentation and collection
- **Thread-safe storage**: Atomic operations and lock-free updates
- **Configurable thresholds**: Customizable performance boundaries