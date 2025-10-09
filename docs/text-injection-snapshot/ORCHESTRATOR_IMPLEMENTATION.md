# Strategy Orchestrator Implementation

## Overview

The Strategy Orchestrator is a new component in the text injection system that acts as the "brain" for managing text injection across different desktop environments. It provides environment-aware strategy selection, fast-fail execution with strict budgets, and integrates with the pre-warming system.

## Key Features

### 1. Environment Detection

The orchestrator automatically detects the current desktop environment:

- **KDE/Wayland** and **KDE/X11**
- **Hyprland** (wlroots-based Wayland)
- **GNOME/Wayland** and **GNOME/X11**
- **Other Wayland** and **Other X11** desktops
- **Windows** and **macOS**
- **Unknown** environments

### 2. Environment-Specific Strategy Selection

Based on the detected environment, the orchestrator selects the optimal injection strategy:

- **KDE/Wayland**: AT-SPI Insert → Clipboard Paste
- **Hyprland**: AT-SPI Insert → Clipboard Paste
- **Windows**: Clipboard Paste
- **Other environments**: AT-SPI Insert → Clipboard Paste

### 3. Fast-Fail Loop with Strict Budgets

The orchestrator implements a fast-fail injection loop with strict time budgets:

- **Total budget**: Configurable (default: 1000ms)
- **Stage budget**: ≤50ms per injection attempt
- **Confirmation budget**: ≤75ms for injection confirmation

If any stage exceeds its budget, the orchestrator immediately fails and tries the next strategy.

### 4. Pre-Warming Integration

The orchestrator integrates with the pre-warming system to:

- Trigger pre-warming when the session enters the Buffering state
- Use pre-warmed AT-SPI connections and clipboard data
- Maintain cached data for faster injection

### 5. Error Handling and Fallback

The orchestrator provides robust error handling:

- Automatic fallback to the next strategy on failure
- Exponential backoff for repeated failures
- Detailed error logging and metrics

## Implementation Details

### Core Components

1. **StrategyOrchestrator**: Main orchestrator struct
2. **DesktopEnvironment**: Enum representing detected environments
3. **AtspiContext**: Context for AT-SPI injection operations

### Key Methods

- `new()`: Create a new orchestrator instance
- `detect_environment()`: Detect the current desktop environment
- `get_strategy_order()`: Get the optimal strategy order for the current environment
- `inject_text()`: Inject text using the optimal strategy
- `is_available()`: Check if the orchestrator is available

### Integration Points

- **Pre-warming System**: Uses pre-warmed AT-SPI connections and clipboard data
- **Session Management**: Monitors session state to trigger pre-warming
- **Injectors**: Delegates to appropriate injectors based on strategy
- **Confirmation System**: Confirms successful injections

## Usage

```rust
use coldvox_text_injection::{InjectionConfig, StrategyOrchestrator};

// Create an orchestrator
let config = InjectionConfig::default();
let orchestrator = StrategyOrchestrator::new(config).await;

// Get the detected environment
let env = orchestrator.desktop_environment();
println!("Detected environment: {}", env);

// Inject text
orchestrator.inject_text("Hello, World!").await?;
```

## Testing

A test example is provided in `examples/test_orchestrator.rs` that demonstrates:

- Environment detection
- Strategy selection
- Basic text injection
- Error handling

## Future Enhancements

1. **Dynamic Strategy Adaptation**: Learn from success/failure patterns to adapt strategy order
2. **More Granular Budgets**: Per-strategy budget configuration
3. **Enhanced Environment Detection**: Support for more desktop environments
4. **Performance Metrics**: Detailed performance monitoring and reporting
5. **Configuration UI**: User interface for configuring orchestrator settings