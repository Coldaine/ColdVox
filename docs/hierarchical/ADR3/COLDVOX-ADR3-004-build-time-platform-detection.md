---
id: COLDVOX-ADR3-004-build-time-platform-detection
type: ADR
level: 3
title: Build-Time Platform and Desktop Environment Detection
status: accepted
owner: @team-core
updated: 2025-09-11
parent: COLDVOX-DOM2-001-audio-capture
links:
  satisfies: [COLDVOX-DOM2-001-audio-capture]
  depends_on: []
  supersedes: []
  related_to: [COLDVOX-DOM2-005-text-injection]
---

## Context
ColdVox needs to support multiple platforms (Windows, macOS, Linux) and desktop environments (KDE, GNOME, X11, Wayland) with different capabilities and backend requirements. At compile time, we need to detect the target platform and desktop environment to enable appropriate features and backends.

## Decision
Use Rust's `cfg!` macros and Cargo's build script (`build.rs`) to detect platform and desktop environment at compile time, emitting custom `cfg` attributes that can be used throughout the codebase to conditionally compile platform-specific features.

## Status
Accepted

## Consequences
### Positive
- Enables platform-specific optimizations and backend selection
- Reduces runtime overhead by determining capabilities at compile time
- Allows conditional compilation of heavy dependencies only when needed
- Provides clear feature flags for different platform capabilities
- Enables proper backend selection for text injection (AT-SPI, clipboard, ydotool, etc.)

### Negative
- Build script complexity increases
- Platform detection logic must be maintained and tested
- May not detect all edge cases in build environments
- Requires careful documentation of emitted cfg attributes

## Implementation
The build script (`crates/app/build.rs`) detects:
- Target OS using `cfg!(target_os = "...")`
- Desktop environment on Linux by checking environment variables:
  - `KDE_FULL_SESSION` or `PLASMA_SESSION` for KDE → enables `kde_globalaccel`
  - `WAYLAND_DISPLAY` or `XDG_SESSION_TYPE=wayland` for Wayland → enables `wayland_session`
  - `DISPLAY` or `XDG_SESSION_TYPE=x11` for X11 → enables `x11_session`

These custom cfg attributes are then used throughout the codebase to conditionally compile platform-specific features.

## Alternatives Considered
1. Runtime detection only - Would require all backends to be compiled in and increase runtime overhead
2. Feature flags only - Would require manual configuration by users and increase complexity
3. Separate binaries per platform - Would increase distribution complexity and maintenance overhead

## Related Documents
- `crates/app/build.rs`
- `crates/coldvox-text-injection/src/manager.rs`
- `CLAUDE.md` (Platform Detection section)

---
satisfies: COLDVOX-DOM2-001-audio-capture  
depends_on:   
supersedes:   
related_to: COLDVOX-DOM2-005-text-injection