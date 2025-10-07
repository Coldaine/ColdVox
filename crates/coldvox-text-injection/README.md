# coldvox-text-injection

Automated text injection system for ColdVox transcribed speech.

## What's New

- **Composite ClipboardPaste strategy**: Unified clipboard + paste (AT-SPI first, ydotool fallback)
  - Replaced old `combo_clip_ydotool` with cleaner ClipboardPasteInjector
  - Automatic clipboard save/restore with configurable delay
- **FocusProvider DI**: Inject focus detection for deterministic and safe tests
- **Real injection testing**: Lightweight test applications for comprehensive validation
- **Full desktop CI support**: Real audio devices and desktop environments available
- **Allow/block lists**: Compiled regex when `regex` enabled; substring matching otherwise

## Purpose

This crate provides text injection capabilities that automatically type transcribed speech into applications:

- **Multi-Backend Support**: Multiple text injection methods for different environments
- **Focus Tracking**: Automatic detection of active application windows
- **Smart Routing**: Application-specific injection method selection
- **Cross-Platform**: Support for X11, Wayland, and other desktop environments

## Key Components

### Text Injection Backends
- **AT-SPI Insert**: Direct text insertion via accessibility API (preferred method)
- **ClipboardPaste** (composite strategy):
  - Sets clipboard content using wl-clipboard
  - Triggers paste via AT-SPI action (tries first) OR ydotool fallback (Ctrl+V simulation)
  - Automatically saves and restores user's clipboard after configurable delay (`clipboard_restore_delay_ms`, default 500ms)
  - **Critical**: This is ONE unified strategy, not separate "clipboard" and "paste" methods
  - **Requires**: Either AT-SPI paste support OR ydotool installed to actually trigger the paste
- **YDotool**: Direct uinput-based key simulation (opt-in, useful when AT-SPI unavailable)
- **KDotool Assist**: KDE/X11 window activation assistance (opt-in)
- **Enigo**: Cross-platform input simulation library (opt-in)

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
- `atspi`: Linux AT-SPI accessibility backend
- `wl_clipboard`: Clipboard-based injection via wl-clipboard-rs
- `enigo`: Cross-platform input simulation
- `ydotool`: Linux uinput automation
- `kdotool` / `xdg_kdotool`: KDE/X11 window activation assistance (alias supported)
- `regex`: Compiled allow/block list patterns (regex)
- `all-backends`: Enable all available backends
- `linux-desktop`: Enable recommended Linux desktop backends

## Backend Selection Strategy

The system tries backends in this order (skips unavailable methods):

1. **AT-SPI Insert** - Direct text insertion via accessibility API (most reliable when supported)
2. **ClipboardPaste** - Composite strategy: set clipboard → paste via AT-SPI or ydotool
   - Only registered if AT-SPI paste actions OR ydotool available
   - Fails if neither paste mechanism works
3. **YDotool** - Direct uinput key simulation (opt-in, requires ydotool daemon)
4. **KDotool Assist** - Window activation help (opt-in, X11 only)
5. **Enigo** - Cross-platform input simulation (opt-in)

**Note**: There is NO "clipboard-only" backend. Setting clipboard without triggering paste is useless for automation.

## Configuration

### CLI Options

- `--allow-kdotool`: Enable KDE-specific tools
- `--allow-enigo`: Enable Enigo input simulation
- `--restore-clipboard`: Restore clipboard contents after injection
	- Note: By default clipboard restoration is enabled for the clipboard-based injectors and controlled by `clipboard_restore_delay_ms` (default ~500ms). You can tune or disable behavior via configuration.
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

# For X11 helpers
sudo apt install libxtst-dev wmctrl

# For clipboard functionality
sudo apt install xclip wl-clipboard

# For ydotool-based paste (optional)
sudo apt install ydotool
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
cargo run --features text-injection -- --restore-clipboard
```

## Dependencies

- Backend-specific libraries (optional based on features)
- Platform integration libraries for focus detection
- Async runtime support for timeout handling

## Testing

All tests use real desktop applications and injection backends with full desktop environments available in all environments:

```bash
# Run crate tests with real injection validation
cargo test -p coldvox-text-injection --locked

# No-default-features with real hardware
cargo test -p coldvox-text-injection --no-default-features --locked

# Regex feature with real injection testing
cargo test -p coldvox-text-injection --no-default-features --features regex --locked
```

See `docs/testing.md` for details on live/CI testing and feature matrices.
