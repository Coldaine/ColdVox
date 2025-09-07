ColdVox Text Injection — Testing Guide

Overview
- This guide covers how to run tests for the `coldvox-text-injection` crate across feature combinations, how compile-time mocking is approached, and how to run optional integration tests.

 Scope Reminder: The active prototype targets Linux Nobara on KDE Plasma. Any environment-specific notes assume this desktop unless otherwise stated.

Test Modes
- Default: `cargo test -p coldvox-text-injection`
- No-default-features: `cargo test -p coldvox-text-injection --no-default-features`
- Regex feature: `cargo test -p coldvox-text-injection --features regex`

Notes
- Tests are headless and avoid relying on actual desktop services. The `StrategyManager` falls back to safe paths in headless environments.
- Clipboard and ydotool integration paths are exercised via logic-level tests; heavy system interactions should be covered by compile-time–swappable facades (planned).

Compile-time Mocking (pattern)
- Preferred approach is an alias module pattern:
  - `pub mod command_api { #[cfg(not(test))] pub use real_impl::CommandApi as CommandApi; #[cfg(test)] pub use mock_impl::CommandApi as CommandApi; }`
  - Backends (`ydotool_injector`, `kdotool_injector`, `window_manager`, `combo_clip_ydotool`) depend on `command_api::CommandApi` to run commands, allowing tests to simulate success/non-zero exit/timeout deterministically.

Optional Integration
- Real backends (AT-SPI, wl-clipboard, ydotool) are not required for unit tests. Integration tests that touch those systems should be `#[ignore]` by default and documented with preconditions.

Environment Tips
- Wayland: set `WAYLAND_DISPLAY=wayland-0` if manually exercising clipboard code.
- ydotool: requires daemon/socket, uinput permissions; not required for unit tests.
