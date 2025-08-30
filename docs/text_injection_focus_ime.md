# Text Injection Focus and IME/Localization Guidance

## Overview

This document outlines the focus tracking and Input Method Editor (IME) considerations for reliable text injection across different platforms and locales.

## Focus Tracking Requirements

### Current Limitations
- Focus detection is currently basic and may not handle complex window hierarchies
- AT-SPI integration is incomplete for focus tracking
- No real-time focus change monitoring

### AT-SPI Roadmap

#### Required FocusTracker Methods
```rust
impl FocusTracker {
    /// Get the currently focused accessible element
    pub async fn get_focused_element(&self) -> Result<AccessibleElement, FocusError>;

    /// Check if the focused element supports paste operations
    pub async fn supports_paste_action(&self) -> Result<bool, FocusError>;

    /// Get application identifier from focused element
    pub async fn get_app_id(&self) -> Result<String, FocusError>;
}
```

#### Implementation Priority
1. **Phase 1**: Basic focus detection (current)
2. **Phase 2**: AT-SPI element inspection
3. **Phase 3**: Real-time focus monitoring
4. **Phase 4**: Cross-process focus validation

## IME and Localization Considerations

### IME-Heavy Environments
- **East Asian Languages**: Chinese, Japanese, Korean require IME for text input
- **Complex Scripts**: Arabic, Hebrew, Devanagari may have IME dependencies
- **Mobile/Desktop Convergence**: Increasing IME usage on desktop

### Injection Strategy for IME

#### Current Auto Mode Logic
```rust
let use_paste = match config.injection_mode.as_str() {
    "paste" => true,
    "keystroke" => false,
    "auto" => {
        // Current: length-based threshold
        text.len() > config.paste_chunk_chars as usize
    }
    _ => text.len() > config.paste_chunk_chars as usize,
};
```

#### Future IME-Aware Logic
```rust
let use_paste = match config.injection_mode.as_str() {
    "paste" => true,
    "keystroke" => false,
    "auto" => {
        // Future: IME-aware decision
        text.len() > config.paste_chunk_chars as usize ||
        self.should_use_paste_for_ime(text).await
    }
    _ => text.len() > config.paste_chunk_chars as usize,
};
```

### IME Detection Methods

#### Configuration-Based
```toml
[text_injection]
# Future: Prefer paste for IME environments
prefer_paste_for_ime = true

# Current: Rely on length thresholds
paste_chunk_chars = 50
```

#### Runtime Detection
- Check for active IME processes
- Monitor keyboard layout changes
- Detect non-ASCII character patterns
- Query system IME status

## Platform-Specific Considerations

### Wayland
- **Virtual Keyboard**: Portal/wlr virtual keyboard not yet implemented
- **Clipboard + AT-SPI**: Most reliable current approach
- **Focus Tracking**: Requires AT-SPI for accurate element focus

### X11
- **xdotool Path**: Available for fallback injection
- **Window Properties**: WM_CLASS for application identification
- **IME Integration**: Varies by desktop environment

### Windows/macOS
- **Native APIs**: Platform-specific focus and IME detection
- **Accessibility APIs**: Required for reliable injection
- **Virtual Keyboard**: May be available through system APIs

## Action Items

### Immediate (Phase 2)
1. Implement AT-SPI focus element inspection
2. Add basic IME language detection
3. Improve focus validation before injection

### Medium-term (Phase 3)
1. Add real-time focus monitoring
2. Implement IME-aware injection decisions
3. Add platform-specific IME detection

### Long-term (Phase 4)
1. Virtual keyboard integration (Wayland portal)
2. Advanced IME state tracking
3. Cross-platform IME compatibility layer

## Testing Scenarios

### IME Testing Matrix
- **Locale**: en_US, zh_CN, ja_JP, ko_KR, ar_SA
- **IME State**: Active, Inactive, Switching
- **Text Types**: ASCII-only, Mixed, Unicode-only
- **Injection Methods**: Paste, Keystroke, Auto

### Focus Testing
- **Window Types**: Native, Web, Terminal, IDE
- **Focus Changes**: During injection, Between injections
- **Modal Dialogs**: System dialogs, Application modals
- **Multi-Monitor**: Focus across displays

## Configuration Recommendations

### Conservative Settings (Default)
```toml
[text_injection]
# Prefer paste for reliability
injection_mode = "paste"
# Shorter chunks to avoid IME issues
paste_chunk_chars = 20
```

### Performance-Optimized
```toml
[text_injection]
# Allow keystroke for short text
injection_mode = "auto"
# Balance between IME safety and performance
paste_chunk_chars = 50
```

## Troubleshooting

### Common IME Issues
1. **Text Not Appearing**: IME consuming keystrokes
2. **Wrong Characters**: Encoding mismatches
3. **Focus Loss**: IME switching focus during injection
4. **Composition Conflicts**: IME composition mode interference

### Mitigation Strategies
1. **Force Paste Mode**: For IME-heavy applications
2. **Focus Validation**: Ensure target has focus before injection
3. **Timing Adjustments**: Account for IME processing delays
4. **Fallback Chains**: Multiple injection attempts with different methods