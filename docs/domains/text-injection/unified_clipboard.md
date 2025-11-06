---
doc_type: reference
subsystem: text-injection
version: 1.0.0
status: draft
owners: Text Injection Maintainers
last_reviewed: 2025-11-06
redirect: ti-unified-clipboard.md
---

# Moved: Unified Clipboard Injector

This document was renamed to include the domain short code per the Master Documentation Playbook.

New location:
- `docs/domains/text-injection/ti-unified-clipboard.md`

Please update any bookmarks or links.

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
