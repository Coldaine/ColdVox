# coldvox-telemetry

Telemetry and metrics infrastructure for ColdVox performance monitoring.

## Purpose

This crate provides comprehensive performance monitoring and metrics collection for the ColdVox voice processing pipeline:

- **Pipeline Metrics**: Frame processing rates, latency tracking, and throughput monitoring
- **Audio Metrics**: Capture rates, buffer utilization, and dropout detection
- **Performance Counters**: CPU usage, memory consumption, and processing times
- **Health Monitoring**: System health checks and error rate tracking

## Key Components

### PipelineMetrics
- Tracks audio capture and processing frame rates
- Monitors VAD and STT processing performance
- Provides real-time statistics for debugging

### HealthMonitor
- Periodic system health checks
- Automatic recovery triggers
- Performance degradation detection

## API Overview

```rust
use coldvox_telemetry::{PipelineMetrics, HealthMonitor};

// Initialize metrics collection
let metrics = PipelineMetrics::new();
let health_monitor = HealthMonitor::new(check_interval).start();

// Update metrics
metrics.capture_fps.store(fps_value, Ordering::Relaxed);
```

## Features

- `default`: Standard telemetry functionality

## Usage

This crate is primarily used internally by other ColdVox components to collect and report performance metrics. The telemetry data can be accessed through the main application's status reporting and debugging interfaces.

## Dependencies

- `parking_lot`: Efficient synchronization for metrics storage
