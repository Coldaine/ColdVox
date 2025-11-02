# Unified Clipboard Injector

This document describes the unified clipboard injector implementation that consolidates
functionality from the previous `ClipboardInjector`, `ClipboardPasteInjector`, and `ComboClipboardYdotool` implementations.

## Overview

The `UnifiedClipboardInjector` provides a single, configurable implementation for clipboard-based text injection
that supports both strict and best-effort modes. It consolidates the best features from the three
previous implementations while eliminating code duplication.

## Features

### Platform Support
- **Wayland**: Native support via `wl-clipboard-rs` with fallback to `wl-copy`/`wl-paste`
- **X11**: Support via `xclip` command
- **Cross-platform**: AT-SPI paste when available, with Enigo and ydotool fallbacks

### Injection Modes
- **BestEffort**: Attempts paste but continues even if paste fails (default behavior)
- **Strict**: Requires successful paste and returns error if paste fails

### Clipboard Management
- Automatic backup and restore of clipboard content
- Configurable restore delay
- Optional Klipper history cleanup (feature-gated)

### Error Handling
- Comprehensive timeout handling for all operations
- Graceful fallbacks between native and command-line tools
- Detailed error context and logging

## Usage

```rust
use coldvox_text_injection::injectors::unified_clipboard::{UnifiedClipboardInjector, ClipboardInjectionMode};

// Create injector with best-effort mode (default)
let injector = UnifiedClipboardInjector::new(config);

// Or create with strict mode
let strict_injector = UnifiedClipboardInjector::new_with_mode(config, ClipboardInjectionMode::Strict);

// Use the injector
injector.inject_text("Hello, world!", None).await?;
```

## Migration from Previous Implementations

The unified implementation replaces:
- `ClipboardInjector` - Core clipboard functionality
- `ClipboardPasteInjector` - Strict mode behavior
- `ComboClipboardYdotool` - Best-effort mode with ydotool integration

All existing code using these implementations has been updated to use the new
`UnifiedClipboardInjector` while maintaining the same API and behavior.

## Benefits

1. **Reduced Code Duplication**: Eliminated ~1000 lines of duplicate clipboard handling code
2. **Improved Maintainability**: Single implementation to maintain instead of three
3. **Enhanced Flexibility**: Configurable strict/best-effort modes
4. **Better Error Handling**: Unified timeout and fallback mechanisms
5. **Consistent Behavior**: Same API and behavior across all use cases
