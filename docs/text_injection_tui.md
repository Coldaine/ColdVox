# Text Injection TUI Wiring Specification

## Overview

This document specifies the user interface elements and data sources for displaying text injection status in the ColdVox TUI dashboard.

## Display Panels

### Main Injection Status Panel

#### Header Section
```
Text Injection Status
━━━━━━━━━━━━━━━━━━━━━
```

#### Core Metrics Row
```
Backend: Wayland+AT-SPI    Mode: auto    Status: Active
```

#### Performance Metrics Row
```
Buffer: 0 chars    Last: 42 chars    Latency: 45ms    Success: 98.5%
```

#### Error/Status Indicators Row
```
Errors: 2    Rate Limited: 0    Backend Denied: 0    Paused: No
```

### Detailed Metrics Panel

#### Success/Failure Breakdown
```
Success Rate: 98.5% (247/250)
├── Paste: 95.2% (120/126)
├── Keystroke: 99.1% (109/110)
└── AT-SPI: 97.8% (88/90)
```

#### Latency Histogram
```
Latency Distribution (ms):
├── <50ms: ████████░░ 80%
├── 50-100ms: ████░░░░ 40%
├── 100-200ms: █░░░░░░░ 10%
└── >200ms: ░░░░░░░░ 0%
```

## Data Sources

### Primary Data Source
```rust
// Shared metrics instance
let metrics: Arc<Mutex<InjectionMetrics>> = Arc::new(Mutex::new(InjectionMetrics::default()));
```

### Backend Detection
```rust
// From BackendDetector
let current_backend = backend_detector.get_preferred_backend();
let available_backends = backend_detector.detect_available_backends();
```

### Configuration Values
```rust
// From InjectionConfig
let injection_mode = config.injection_mode; // "auto", "paste", "keystroke"
let pause_hotkey = config.pause_hotkey; // Optional hotkey display
```

## Field Specifications

### Backend Field
- **Source**: `BackendDetector.get_preferred_backend()`
- **Format**: "Wayland+AT-SPI", "X11+Clipboard", "Windows+SendInput", etc.
- **Update**: On backend changes or detection failures
- **Fallback**: "Unknown" if detection fails

### Mode Field
- **Source**: `InjectionConfig.injection_mode`
- **Format**: "auto", "paste", "keystroke"
- **Update**: On configuration changes
- **Default**: "auto"

### Buffer Chars Field
- **Source**: `InjectionMetrics.chars_buffered`
- **Format**: "X chars" (e.g., "0 chars", "156 chars")
- **Update**: Real-time as text is processed
- **Reset**: On successful injection or flush

### Last Flush Size Field
- **Source**: `InjectionMetrics.last_flush_size`
- **Format**: "X chars" (e.g., "42 chars")
- **Update**: After each successful injection
- **Default**: "0 chars"

### Latency Field
- **Source**: Moving average of `InjectionMetrics.latency_samples`
- **Format**: "Xms" (e.g., "45ms", "127ms")
- **Update**: Calculated from recent samples
- **Default**: "0ms"

### Success Rate Field
- **Source**: `InjectionMetrics.successes` / `InjectionMetrics.attempts`
- **Format**: "X.X%" (e.g., "98.5%")
- **Update**: After each injection attempt
- **Default**: "0.0%"

### Error Counters
- **Source**: Various `InjectionMetrics` counters
- **Format**: Integer counts
- **Update**: On error occurrence
- **Fields**:
  - `failures`: Total injection failures
  - `rate_limited`: Times rate limit was hit
  - `backend_denied`: Times backend was unavailable

## Update Cadence

### Real-time Updates (50-100ms)
- Buffer character count
- Current operation status
- Active injection progress

### Standard Updates (200-500ms)
- Success/failure rates
- Latency calculations
- Backend status
- Error counters

### Slow Updates (1-5 seconds)
- Moving averages
- Historical summaries
- Configuration changes

## Pause Toggle Integration

### Display Logic
```rust
if injection_paused {
    "Paused: Yes (Hotkey: Ctrl+Shift+P)"
} else {
    "Paused: No"
}
```

### Hotkey Display
- **Source**: `InjectionConfig.pause_hotkey`
- **Format**: Human-readable (e.g., "Ctrl+Shift+P")
- **Fallback**: Hidden if no hotkey configured

## Example Layout

```
┌─ Text Injection ──────────────────────────────┐
│ Backend: Wayland+AT-SPI    Mode: auto         │
│ Buffer: 0 chars    Last: 42 chars    45ms    │
│ Success: 98.5%    Errors: 2    Paused: No    │
└──────────────────────────────────────────────┘

┌─ Injection Methods ──────────────────────────┐
│ Paste:     ████████░░ 95.2% (120/126)        │
│ Keystroke: ████████░ 99.1% (109/110)         │
│ AT-SPI:    ███████░░ 97.8% (88/90)          │
└──────────────────────────────────────────────┘

┌─ Latency Distribution ──────────────────────┐
│ <50ms:  ████████░░ 80%                      │
│ 50-100ms: ████░░░░ 40%                      │
│ 100-200ms: █░░░░░░░ 10%                     │
│ >200ms: ░░░░░░░░ 0%                         │
└──────────────────────────────────────────────┘
```

## Error Handling

### Backend Unavailable
```
Backend: UNAVAILABLE (Wayland+AT-SPI)
Mode: auto (degraded)
```

### High Error Rate
```
Success: 45.2% ⚠️  Errors: 23
```

### Rate Limited
```
Rate Limited: 5 (last 30s)
```

## Configuration Integration

### TUI-Specific Config
```toml
[tui]
# Text injection panel settings
text_injection_panel = true
text_injection_update_ms = 300
text_injection_show_latency = true
text_injection_show_method_breakdown = true
```

### Metrics Collection
```rust
// Ensure metrics are collected
let metrics = Arc::new(Mutex::new(InjectionMetrics::new()));
strategy_manager.set_metrics(metrics.clone());
processor.set_metrics(metrics);
```

## Implementation Notes

- All data access must be thread-safe using `Arc<Mutex<>>`
- UI updates should not block injection operations
- Redact sensitive information in error displays
- Handle division by zero in percentage calculations
- Provide graceful degradation when metrics unavailable