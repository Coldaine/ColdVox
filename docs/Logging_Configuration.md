# ColdVox Logging Configuration Guide

## Overview

ColdVox uses the `tracing` framework for structured, async-safe logging with automatic file rotation and dual output capabilities.

## Current Setup

### Dependencies
- `tracing = "0.1"` - Modern structured logging framework
- `tracing-subscriber = "0.3"` - Log collection and formatting with env-filter
- `tracing-appender = "0.2"` - File output with rotation support

### Architecture
- **Dual Output**: Logs appear both in console and persistent files
- **Daily Rotation**: Automatic file rotation prevents disk space issues
- **Environment Control**: `RUST_LOG` variable controls verbosity
- **Async Compatible**: Safe for use in async/tokio applications

## Log Levels and Usage

### Available Levels
- `ERROR` - System errors, failures requiring attention
- `WARN` - Potential issues, recovery attempts
- `INFO` - General operation information (default)
- `DEBUG` - Detailed debugging information
- `TRACE` - Very verbose debugging (not used in ColdVox)

### Environment Configuration
```bash
# Default (info level)
cargo run

# All errors only
RUST_LOG=error cargo run

# Full debugging
RUST_LOG=debug cargo run

# Module-specific debugging
RUST_LOG=coldvox_app::audio=debug cargo run

# Multiple modules
RUST_LOG=coldvox_app::audio=debug,coldvox_app::foundation=info cargo run

# Complex filtering
RUST_LOG="info,coldvox_app::audio::capture=debug" cargo run
```

## File Organization

### Directory Structure
```
logs/
├── coldvox.log              # Current day's logs
├── coldvox.log.2024-08-23   # Previous day
├── coldvox.log.2024-08-24   # Yesterday
└── coldvox.log.2024-08-25   # Today
```

### Rotation Policy
- **Frequency**: Daily rotation at midnight
- **Retention**: Manual cleanup (files persist indefinitely)
- **Format**: `coldvox.log.YYYY-MM-DD` for archived files
- **Current**: `coldvox.log` for today's logs

## Log Coverage Analysis

### Current Coverage (42 logging statements)

#### Application Lifecycle
- Startup and shutdown events
- State transitions (Running → Stopping → Stopped)
- Component initialization and cleanup

#### Audio Processing
- Device discovery and configuration
- Stream creation and management
- Buffer overflow warnings
- Silence detection alerts

#### Error Handling
- Audio device failures and recovery
- Watchdog timeouts and triggers
- Stream errors and reconnection attempts

#### Health Monitoring
- Component health checks
- Recovery attempts and outcomes
- Performance statistics (frames captured/dropped)

## Understanding Spans

**Spans** are time-bounded contexts that group related log events together.

### Benefits
- **Correlation**: Related logs are grouped with shared context
- **Timing**: Automatic duration tracking for operations
- **Hierarchy**: Nested spans create operation call trees
- **Traceability**: Easy to follow complex operation flows

### Current Usage
Limited span usage - mainly using discrete events.

### Future Enhancements
Consider adding spans for:
- Audio recovery sessions
- Device initialization sequences
- Health check cycles

## Debugging Workflows

### Basic Troubleshooting
```bash
# Start with debug level
RUST_LOG=debug cargo run

# Check recent logs
tail -f logs/coldvox.log

# Search for specific issues
grep -i "error" logs/coldvox.log
grep -i "recovery" logs/coldvox.log
```

### Audio Issues
```bash
# Focus on audio subsystem
RUST_LOG=coldvox_app::audio=debug cargo run

# Monitor audio capture specifically
RUST_LOG=coldvox_app::audio::capture=debug cargo run
```

### Performance Analysis
```bash
# Monitor with periodic stats
grep "Audio stats:" logs/coldvox.log

# Check for buffer issues
grep -i "buffer\|overflow\|dropped" logs/coldvox.log
```

## Example Log Outputs

### Info Level (Default)
```
2024-08-24T22:00:00.000Z INFO coldvox_app: Starting ColdVox application
2024-08-24T22:00:00.100Z INFO coldvox_app: Application state: Running
2024-08-24T22:00:00.200Z INFO coldvox_app::audio::capture: Opening audio device: Default Input Device
2024-08-24T22:00:30.000Z INFO coldvox_app: Audio stats: 1500 frames captured, 0 dropped, 0 disconnects, 0 reconnects
```

### Debug Level
```
2024-08-24T22:00:00.000Z INFO coldvox_app: Starting ColdVox application
2024-08-24T22:00:00.050Z DEBUG coldvox_app::foundation::state: State transition: Initializing -> Running
2024-08-24T22:00:00.100Z INFO coldvox_app: Application state: Running
2024-08-24T22:00:00.150Z DEBUG coldvox_app::audio::device: Enumerating audio devices
2024-08-24T22:00:00.200Z INFO coldvox_app::audio::capture: Opening audio device: Default Input Device
2024-08-24T22:00:00.250Z DEBUG coldvox_app::audio::capture: Audio config: StreamConfig { channels: 1, sample_rate: SampleRate(16000), buffer_size: Default }
```

### Error Scenarios
```
2024-08-24T22:15:30.000Z ERROR coldvox_app::audio::watchdog: Watchdog timeout! No audio data for 5.2s
2024-08-24T22:15:30.100Z WARN coldvox_app: Audio watchdog triggered, attempting recovery
2024-08-24T22:15:30.200Z INFO coldvox_app::audio::capture: Attempting audio recovery
2024-08-24T22:15:32.000Z ERROR coldvox_app::audio::capture: Recovery attempt 1 failed: Device not found
2024-08-24T22:15:34.000Z INFO coldvox_app::audio::capture: Recovery attempt 2/3
2024-08-24T22:15:36.000Z INFO coldvox_app::audio::capture: Audio recovery successful
```

## Technical Implementation

### Logger Initialization
```rust
fn init_logging() -> Result<(), Box<dyn std::error::Error>> {
    // Create logs directory
    std::fs::create_dir_all("logs")?;
    
    // Set up daily rotating file appender
    let file_appender = RollingFileAppender::new(Rotation::daily(), "logs", "coldvox.log");
    let (non_blocking_file, _guard) = tracing_appender::non_blocking(file_appender);
    
    // Environment-based log level configuration
    let log_level = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    
    // Dual output: console + file
    tracing_subscriber::fmt()
        .with_writer(std::io::stdout.and(non_blocking_file))
        .with_env_filter(log_level)
        .init();
    
    // Keep guard alive for program duration
    std::mem::forget(_guard);
    Ok(())
}
```

### Log Statement Examples
```rust
// Basic event logging
tracing::info!("Application started successfully");

// Structured logging with fields
tracing::warn!(
    device_name = %device_name,
    attempt = attempt_number,
    "Device connection failed"
);

// Error with context
tracing::error!(
    error = %error,
    recovery_attempt = attempt,
    "Audio recovery failed"
);
```

## Best Practices

### Do
- Use appropriate log levels for message importance
- Include contextual information in structured fields
- Log state transitions and significant events
- Use environment variables for runtime log control

### Don't  
- Log sensitive information (credentials, personal data)
- Use println! or eprintln! instead of tracing macros
- Log in tight loops without rate limiting
- Ignore the performance impact of excessive debug logging

## Maintenance

### Log File Management
- Monitor log directory size periodically
- Implement log retention policy if needed (currently manual)
- Consider log aggregation for long-term storage

### Performance Considerations
- Debug logging has minimal performance impact due to async design
- File I/O is non-blocking to prevent audio processing delays
- Log rotation prevents individual files from becoming too large

## Future Enhancements

### Potential Improvements
1. **Span Implementation**: Add operation spans for better correlation
2. **Structured Fields**: Convert remaining string-formatted logs
3. **Log Retention**: Automatic cleanup of old log files
4. **JSON Format**: Optional JSON output for log analysis tools
5. **Correlation IDs**: Track related operations across components

### Not Planned (Over-engineering for personal project)
- Distributed tracing
- Log aggregation services (ELK, Fluentd)
- Complex log shipping
- Enterprise monitoring integration