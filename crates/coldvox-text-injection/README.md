# coldvox-text-injection

Automated text injection system for ColdVox transcribed speech.

## Purpose

This crate provides text injection capabilities that automatically type transcribed speech into applications:

- **Multi-Backend Support**: Multiple text injection methods for different environments
- **Focus Tracking**: Automatic detection of active application windows
- **Smart Routing**: Application-specific injection method selection
- **Cross-Platform**: Support for X11, Wayland, and other desktop environments

## Key Components

### Text Injection Backends
- **Clipboard**: Copy transcription to clipboard and paste
- **AT-SPI**: Accessibility API for direct text insertion
- **XDotool**: X11-based keyboard simulation
- **YDotool**: Universal input device simulation
- **Native APIs**: Platform-specific keyboard/input APIs

### Focus Detection
- Active window detection and application identification
- Application-specific method prioritization
- Unknown application fallback strategies

### Smart Injection Management
- Latency optimization and timeout handling
- Method fallback chains for reliability
- Configurable injection strategies per application

## Features

- `default`: Core text injection functionality with safe defaults
- `all-backends`: Enable all available injection backends
- `x11`: X11-specific backends (XDotool, etc.)
- `linux-desktop`: Common Linux desktop environment support
- `atspi`: AT-SPI accessibility support
- `arboard`: Clipboard-based injection
- `enigo`: Cross-platform input simulation
- `wl_clipboard`: Wayland clipboard support
- `xdg_kdotool`: KDE-specific tooling

## Backend Selection

The system automatically selects the best available backend for each application:

1. **AT-SPI** (preferred for accessibility compliance)
2. **Native APIs** (platform-specific optimized methods)
3. **Clipboard + Paste** (universal fallback)
4. **Input Simulation** (XDotool/YDotool for compatibility)

## Configuration

### CLI Options
- `--allow-ydotool`: Enable YDotool backend
- `--allow-kdotool`: Enable KDE-specific tools
- `--allow-enigo`: Enable Enigo input simulation
- `--allow-mki`: Enable MKI input methods
- `--restore-clipboard`: Restore clipboard contents after injection
- `--inject-on-unknown-focus`: Inject even when focus detection fails

### Timing Controls
- `--max-total-latency-ms`: Maximum time allowed for injection
- `--per-method-timeout-ms`: Timeout per backend attempt
- `--cooldown-initial-ms`: Delay before first injection attempt

## System Requirements

### Linux
```bash
# For AT-SPI support
sudo apt install libatk-bridge2.0-dev

# For X11 backends  
sudo apt install libxdo-dev libxtst-dev

# For clipboard functionality
sudo apt install xclip wl-clipboard
```

### Security Considerations

Text injection requires various system permissions:
- **X11**: Access to X server for input simulation
- **Wayland**: May require special permissions for input
- **AT-SPI**: Accessibility service access
- **Clipboard**: Read/write access to system clipboard

## Usage

Enable through the main ColdVox application:

```bash
# Basic text injection
cargo run --features text-injection

# With specific backends
cargo run --features text-injection -- --allow-ydotool --restore-clipboard
```

## Dependencies

- Backend-specific libraries (optional based on features)
- Platform integration libraries for focus detection
- Async runtime support for timeout handling