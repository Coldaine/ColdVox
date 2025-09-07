# coldvox-text-injection

Automated text injection system for ColdVox transcribed speech.

## What’s New (workspace v2.0.1)

- FocusProvider DI: inject focus detection for deterministic and safe tests
- Combo clipboard+paste injector (`combo_clip_ydotool`) with async `ydotool` check
- Comprehensive mock injectors and utilities for fallback and latency tests
- Headless CI support using Xvfb + fluxbox + D-Bus; readiness loops (no fixed sleeps)
- Allow/block list semantics: compiled regex path when `regex` is enabled; substring matching otherwise

## Quick Setup

For automated setup of all dependencies and permissions:

```bash
# From the workspace root
./scripts/setup_text_injection.sh
```

This handles package installation, permissions, and daemon configuration for your Linux distribution.

## Manual Setup

If you prefer manual installation, see the platform-specific instructions below.

- **Multi-Backend Support**: Multiple text injection methods for different environments
- **Focus Tracking**: Automatic detection of active application windows
- **Smart Routing**: Application-specific injection method selection
- **Cross-Platform**: Support for X11, Wayland, and other desktop environments

## Text Injection Methods: AT-SPI vs Ydotool

This crate provides two fundamentally different approaches to text injection, each with distinct advantages and limitations:

### 🔍 AT-SPI (Accessibility API) Approach

**How it works:**
- Uses Linux Accessibility APIs to query the desktop environment
- Finds specific UI elements (text fields, editors) in applications
- Directly inserts text into the identified element
- Requires AT-SPI service to be running and accessible

**Key Characteristics:**
- **Precise targeting**: Can insert text into specific text fields
- **Element-aware**: Knows about different UI components
- **Application integration**: Works with accessibility-enabled applications
- **Complex setup**: Requires AT-SPI daemon and permissions

**When to use:**
- When you need to target specific text fields
- For applications with complex UI structures
- When accessibility features are important

**Example use case:** Inserting text into a specific form field in a web browser

### ⌨️ Ydotool (Keyboard Simulation) Approach

**How it works:**
- Creates virtual input devices via Linux uinput subsystem
- Simulates keyboard input (like pressing Ctrl+V) to the currently focused window
- The focused application receives input as if you physically typed it
- No element detection - just sends input to whatever has focus

**Key Characteristics:**
- **Simple and reliable**: Works with any focused application
- **No element finding**: Doesn't need to identify specific UI components
- **Universal compatibility**: Works with all applications that accept keyboard input
- **Focus-dependent**: Only works with the currently active window

**When to use:**
- When you want simple, reliable text insertion
- For applications where focus is already on the target area
- When you don't need to target specific text fields
- As a fallback when AT-SPI is unavailable

**Example use case:** Pasting transcribed speech into any text editor or chat application

### 📊 Comparison Table

| Aspect | AT-SPI | Ydotool |
|--------|--------|---------|
| **Precision** | High (targets specific elements) | Low (targets focused window) |
| **Setup Complexity** | High (AT-SPI daemon, permissions) | Medium (uinput, daemon) |
| **Application Support** | Accessibility-enabled apps only | All applications |
| **Reliability** | Variable (depends on app accessibility) | High (works with any focused app) |
| **Performance** | Slower (API queries + element finding) | Fast (direct input simulation) |
| **Dependencies** | `libatk-bridge2.0-dev`, AT-SPI service | `ydotool`, `ydotoold` daemon |
| **Focus Requirements** | Element must be focusable | Window must have keyboard focus |

### 🎯 Choosing the Right Method

**Use AT-SPI when:**
- You need to insert text into specific form fields or text areas
- Working with complex applications (IDEs, web forms, etc.)
- Accessibility features are available and working

**Use Ydotool when:**
- You want maximum compatibility across all applications
- The target application already has focus on the right area
- You need a simple, reliable fallback method
- Setup complexity should be minimized

### 🔧 Implementation Details

**AT-SPI Implementation:**
```rust
// Queries accessibility tree to find text elements
let collection = CollectionProxy::builder(zbus_conn)
    .destination("org.a11y.atspi.Registry")
    .path("/org/a11y/atspi/accessible/root")
    .build()
    .await?;

// Finds focused actionable elements
let matches = collection.get_matches(rule, SortOrder::Canonical, 1, false).await?;

// Inserts text directly into the element
action.do_action(paste_index, &[]).await?;
```

**Ydotool Implementation:**
```rust
// Sets clipboard content
clipboard_injector.inject_text(text).await?;

// Sends Ctrl+V to focused window via uinput
Command::new("ydotool")
    .env("YDOTOOL_SOCKET", "/tmp/.ydotool_socket")
    .args(["key", "ctrl+v"])
    .output()
    .await?;
```

### 🚀 Performance & Reliability

- **AT-SPI**: More complex but potentially more accurate for specific use cases
- **Ydotool**: Simpler, faster, and more universally compatible
- **Combo Approach**: Use AT-SPI first, fallback to ydotool for maximum reliability

Both methods have their place in a comprehensive text injection system, with ydotool serving as the reliable workhorse and AT-SPI providing precision when needed.
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

1. **AT-SPI** (preferred for accessibility compliance)
2. **Clipboard + Paste** (AT-SPI paste when available; `ydotool` fallback)
3. **Clipboard** (plain clipboard set)
4. **Input Simulation** (YDotool/Enigo as opt-in fallbacks)
5. **KDotool Assist** (window activation assistance)

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

## Testing

Headless tests can be run under a session bus; CI uses Xvfb + fluxbox + D-Bus:

```bash
# Run crate tests (default)
dbus-run-session -- cargo test -p coldvox-text-injection --locked

# No-default-features
dbus-run-session -- cargo test -p coldvox-text-injection --no-default-features --locked

# Regex feature
dbus-run-session -- cargo test -p coldvox-text-injection --no-default-features --features regex --locked
```

See `docs/testing.md` for details on live/CI testing and feature matrices.
