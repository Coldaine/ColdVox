# Text Injection Strategy Simplification Analysis

**Date:** 2025-08-31  
**Status:** Design Decision Required

## Problem Statement

The current `StrategyManager` implementation includes sophisticated per-app adaptive behavior with success tracking, cooldowns, and dynamic method reordering. While powerful, this may be over-engineered for our primary target: KDE Plasma on Linux.

## Proposed Simplification

### Platform-Based Configuration

Instead of dynamic per-app adaptation, pass platform context at initialization:

```rust
pub struct PlatformContext {
    os: OperatingSystem,           // Linux, Windows, macOS
    desktop_environment: Option<DE>, // KDE, GNOME, etc.
    compositor: Option<Compositor>,  // KWin, Mutter, wlroots
    distro: Option<String>,         // Debian, Fedora, etc.
}

impl StrategyManager {
    pub fn new(platform: PlatformContext, config: InjectionConfig) -> Self {
        // Configure static strategy based on platform
        let method_order = Self::get_platform_strategy(&platform);
        // ...
    }
}
```

### App Type Categories (Instead of Per-App)

Replace granular per-app tracking with broad categories:

```rust
#[derive(Debug, Clone, Copy)]
pub enum AppType {
    Terminal,      // Konsole, gnome-terminal, alacritty
    WebBrowser,    // Firefox, Chrome, Edge
    IDE,           // VS Code, IntelliJ, Kate
    Office,        // LibreOffice, OnlyOffice
    Chat,          // Discord, Slack, Element
    Generic,       // Everything else
}

// Static configuration per app type
const APP_TYPE_STRATEGIES: &[(AppType, &[InjectionMethod])] = &[
    (AppType::Terminal, &[
        InjectionMethod::YdoToolPaste,
        InjectionMethod::Clipboard,
    ]),
    (AppType::WebBrowser, &[
        InjectionMethod::AtspiInsert,
        InjectionMethod::ClipboardAndPaste,
        InjectionMethod::Clipboard,
    ]),
    // ...
];
```

## Analysis: Is This Simplification Worth It?

### Option 1: Keep Current Implementation As-Is

**Pros:**
- ✅ Already implemented and tested
- ✅ Self-optimizing without manual configuration
- ✅ Handles edge cases automatically
- ✅ No need to maintain app categorization
- ✅ Works across all platforms without changes

**Cons:**
- ❌ More complex code to maintain
- ❌ ~5-10ms overhead on first injection per app
- ❌ Memory overhead for success tracking (~1KB per app)
- ❌ May converge to same patterns anyway

### Option 2: Platform-Based Static Strategy

**Pros:**
- ✅ Simpler, more predictable behavior
- ✅ Faster (no sorting/adaptation overhead)
- ✅ Easier to debug and reason about
- ✅ Clear documentation of what works where

**Cons:**
- ❌ Requires maintaining platform detection logic
- ❌ Need to manually optimize for each platform
- ❌ Can't adapt to unexpected app behavior
- ❌ Loses ability to learn from failures

### Option 3: Hybrid - Platform Base + Optional Adaptation

**Pros:**
- ✅ Best of both worlds
- ✅ Fast defaults with learning capability
- ✅ Can disable adaptation for simplicity
- ✅ Platform-optimized starting point

**Cons:**
- ❌ Still maintains complexity in codebase
- ❌ Two code paths to test and maintain

## Real-World Impact Assessment

### For KDE Plasma Specifically

Given that we're targeting KDE Plasma:

1. **App Uniformity**: Most KDE apps behave similarly (Qt + AT-SPI2)
2. **Limited Variety**: Maybe 20-30 apps total in typical use
3. **Predictable Patterns**: 
   - Terminals → Need ydotool or clipboard
   - Qt Apps → AT-SPI2 works
   - GTK Apps → AT-SPI2 works
   - Browsers → AT-SPI2 works

### Memory & Performance

**Current Implementation Overhead:**
- Memory: ~50KB for strategy manager + ~1KB per app
- CPU: ~5ms on first injection, <0.1ms cached
- **Total Impact**: Negligible for human-speed dictation

**Simplified Implementation:**
- Memory: ~10KB static configuration
- CPU: ~0.5ms constant time
- **Savings**: ~40KB memory, 4.5ms on first injection

## Recommendation

### Keep Current Implementation, But Configure It

The existing implementation is **not complex enough to justify refactoring**. Instead:

1. **Add Platform Hints** to configuration:
```rust
// In InjectionConfig
pub struct InjectionConfig {
    // Existing fields...
    
    // New platform hints
    pub platform_hint: Option<PlatformHint>,
    pub disable_adaptation: bool,  // Turn off per-app learning
    pub force_method_order: Option<Vec<InjectionMethod>>, // Override
}

pub struct PlatformHint {
    pub environment: &'static str,  // "kde-plasma", "gnome", etc.
    pub prefer_methods: Vec<InjectionMethod>,
}
```

2. **Provide Presets**:
```rust
impl InjectionConfig {
    pub fn kde_plasma_preset() -> Self {
        Self {
            disable_adaptation: false,  // Keep learning on
            platform_hint: Some(PlatformHint {
                environment: "kde-plasma",
                prefer_methods: vec![
                    InjectionMethod::AtspiInsert,
                    InjectionMethod::ClipboardAndPaste,
                ],
            }),
            ..Default::default()
        }
    }
}
```

3. **Document Platform Best Practices**:
- KDE Plasma: AT-SPI2 → Clipboard → ydotool
- GNOME: AT-SPI2 → Clipboard
- Sway/wlroots: Clipboard → wtype
- X11: xdotool → Clipboard

## Decision Points

1. **Is 50KB memory overhead significant?** → No, negligible for desktop app
2. **Is 5ms first-injection overhead significant?** → No, human dictation is slower
3. **Does per-app tracking provide value?** → Yes, terminals vs GUI apps
4. **Is the code too complex to maintain?** → No, it's well-structured and tested

## Conclusion

**Don't simplify.** The current implementation is:
- Already working
- Not causing performance issues
- Provides valuable adaptation
- Well-tested

Instead, **add configuration helpers** for specific platforms to make the system easier to use while keeping the adaptive capabilities.

### Action Items

If we proceed with keeping current implementation:
1. ✅ Add `kde_plasma_preset()` configuration helper
2. ✅ Add `disable_adaptation` flag for users who want static behavior
3. ✅ Document recommended configurations per platform
4. ✅ Consider adding app type detection as hint (not replacement) for initial ordering

If we proceed with simplification:
1. ⚠️ Implement platform detection
2. ⚠️ Create static method ordering per platform
3. ⚠️ Remove per-app success tracking
4. ⚠️ Maintain app type categorization

### Final Recommendation

**Keep the existing implementation.** It's not broken, not slow, and provides value. Add platform-specific configuration helpers to make it easier to use. The complexity is already paid for and tested - removing it provides minimal benefit while losing adaptive capabilities that handle edge cases automatically.