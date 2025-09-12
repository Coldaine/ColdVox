---
id: COLDVOX-PIL1-004-text-injection
type: PIL
level: 1
title: Cross-platform Text Injection
status: approved
owner: @team-injection
subsystem: coldvox
updated: 2025-09-11
version: 2
last_reviewed: 2025-09-11
parent: COLDVOX-VSN0-001-voice-ai-pipeline
links:
  satisfies: [COLDVOX-VSN0-001-voice-ai-pipeline]
  depends_on: [COLDVOX-PIL1-003-speech-to-text]
  verified_by: []
  related_to: [COLDVOX-ADR3-003-adaptive-injection-strategy]
---

## Summary
Enable reliable text injection across multiple platforms and windowing systems with adaptive strategies that learn from past attempts to maximize success rates.

## Description
This pillar implements cross-platform text injection with an adaptive strategy manager that dynamically selects the most appropriate backend based on the current environment, application, and historical success rates. The system supports multiple injection methods with fallback chains for maximum compatibility.

## Key Requirements
- Support for major desktop environments (Windows, macOS, Linux)
- Multiple injection backends (AT-SPI, clipboard, ydotool, enigo, etc.)
- Adaptive strategy selection based on environment and success history
- High success rate across different applications (>99%)
- Fallback mechanisms for failed injections with exponential backoff
- Platform-aware backend activation based on build-time detection

## Success Metrics
- Text injection success rate: > 99%
- Cross-platform compatibility: 100%
- Adaptive strategy effectiveness: > 95%
- Recovery from failed injections: < 100ms
- Backend selection accuracy: > 90%

## Technical Approach
The text injection system implements a strategy pattern with adaptive learning:
1. **Backend Trait**: Standardized interface for injection implementations
2. **Multiple Backends**: Platform-specific implementations
3. **Strategy Manager**: Adaptive selection based on success rates
4. **Session Management**: Track application-specific injection history
5. **Fallback Chains**: Ordered retry mechanisms with cooldowns

## Supported Platforms and Backends
- **Linux**:
  - AT-SPI (Accessibility Toolkit)
  - wl-clipboard (Wayland clipboard)
  - ydotool (uinput-based injection)
  - kdotool (X11-based injection)
- **Windows/macOS**:
  - Enigo (Cross-platform input simulation)
- **Combined Strategies**:
  - Clipboard + paste combinations

## Dependencies
- atspi crate for Linux accessibility
- wl-clipboard-rs for Wayland clipboard operations
- enigo crate for cross-platform input simulation
- regex for pattern matching
- thiserror for structured error handling

## Integration Points
- Receives transcription events from STT processing
- Integrates with platform detection at build time
- Provides telemetry for success rate tracking

## Related Architectural Decisions
- [COLDVOX-ADR3-003-adaptive-injection-strategy](../../ADR3/COLDVOX-ADR3-003-adaptive-injection-strategy.md): Adaptive text injection strategy

---
satisfies: COLDVOX-VSN0-001-voice-ai-pipeline  
depends_on: COLDVOX-PIL1-003-speech-to-text  
verified_by:  
related_to: COLDVOX-ADR3-003-adaptive-injection-strategy