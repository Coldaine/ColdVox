---
id: text-injection-overview
title: Text Injection Overview
description: Overview and usage guide for the ColdVox text-injection system.
---

# coldvox-text-injection

Automated text injection system for ColdVox transcribed speech.

## What's New (workspace v2.0.1)

- FocusProvider DI: inject focus detection for deterministic and safe tests
- Combo clipboard+paste injector (`combo_clip_ydotool`) with async `ydotool` check
- Real injection testing with lightweight test applications for comprehensive validation
- Full desktop CI support with real audio devices and desktop environments available
- Allow/block list semantics: compiled regex path when `regex` is enabled; substring matching otherwise

## Purpose

This crate provides text injection capabilities that automatically type transcribed speech into applications:

- Multi-Backend Support: Multiple text injection methods for different environments
- Focus Tracking: Automatic detection of active application windows
- Smart Routing: Application-specific injection method selection
- Cross-Platform: Support for X11, Wayland, and other desktop environments

## Key Components

### Text Injection Backends
- Clipboard: Copy transcription to clipboard and paste
- AT-SPI: Accessibility API for direct text insertion (if enabled)
- Combo (Clipboard + Paste): Clipboard set plus AT-SPI paste or `ydotool` fallback
- YDotool: uinput-based paste or key events (opt-in)
- KDotool Assist: KDE/X11 window activation assistance (opt-in)
- Enigo: Cross-platform input simulation (opt-in)

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

## Backend Selection

The system automatically selects the best available backend for each application:

1. AT-SPI (preferred for accessibility compliance)
2. Clipboard + Paste (AT-SPI paste when available; `ydotool` fallback)
3. Clipboard (plain clipboard set)
4. Input Simulation (YDotool/Enigo as opt-in fallbacks)
5. KDotool Assist (window activation assistance)

## Configuration

### CLI Options

- `--allow-kdotool`: Enable KDE-specific tools
- `--allow-enigo`: Enable Enigo input simulation
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

# For X11 helpers
sudo apt install libxtst-dev wmctrl

# For clipboard functionality
sudo apt install xclip wl-clipboard

# For ydotool-based paste (optional)
sudo apt install ydotool
```

## Security Considerations

Text injection requires various system permissions:
- X11: Access to X server for input simulation
- Wayland: May require special permissions for input
- AT-SPI: Accessibility service access
- Clipboard: Read/write access to system clipboard

## Usage

Enable through the main ColdVox application:

```bash
# Basic text injection
cargo run --features text-injection

# With specific backends
cargo run --features text-injection -- --allow-ydotool --restore-clipboard
```

### Diagnostics

Run the `injection_diagnostics` example to inspect the computed fallback chain and execute a real injection:

```bash
cargo run -p coldvox-text-injection --example injection_diagnostics \
    -- --config ../../config/default.toml --text "diagnostic ping" --no-redact
```

Omit `--no-redact` to keep text content hashed in logs.

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

See `docs/domains/text-injection/testing.md` for details on live/CI testing and feature matrices.
