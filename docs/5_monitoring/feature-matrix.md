---
doc_type: documentation
subsystem: ci-cd
version: 1.0
status: draft
owners: ["CI Team"]
last_reviewed: 2025-09-08
---

# Feature Compatibility Matrix

This document maps all `#[cfg(feature = "...")]` conditional compilation blocks in the codebase to their corresponding Cargo.toml feature definitions, ensuring consistency and proper CI coverage.

## Overview

Features are used to enable optional functionality, platform-specific backends, and test configurations. This matrix ensures that:

- All feature gates match Cargo.toml definitions
- Dependencies are properly enabled when features are activated
- CI workflows test appropriate feature combinations
- No orphaned feature flags exist

## Feature Mappings

### Core Application Features (crates/app/Cargo.toml)

| Feature | Cargo.toml Definition | Code Usage | CI Coverage | Notes |
|---------|----------------------|------------|-------------|-------|
| `vosk` | `coldvox-stt-vosk = { optional = true, features = ["vosk"] }` | `#[cfg(feature = "vosk")]` in runtime.rs, stt/mod.rs, main.rs | Tested in text_injection_tests with VOSK_MODEL_PATH | Requires vosk model download |
| `silero` | `coldvox-vad-silero = { features = ["silero"] }` | `#[cfg(feature = "silero")]` in vad_adapter.rs | Default features in build_and_check | ONNX runtime dependency |
| `text-injection` | Platform-specific deps in target sections | `#[cfg(feature = "text-injection")]` in lib.rs, runtime.rs | Tested in text_injection_tests | Linux: atspi/wl_clipboard/ydotool, Windows/macOS: enigo |
| `live-hardware-tests` | No deps | `#[cfg(feature = "live-hardware-tests")]` in integration tests | Not run in CI (requires hardware) | Marked for future hardware test setup |
| `sleep-observer` | No deps | `#[cfg(feature = "sleep-observer")]` in sleep_instrumentation.rs | Default features | Debug instrumentation |

### Text Injection Features (crates/coldvox-text-injection/Cargo.toml)

| Feature | Cargo.toml Definition | Code Usage | CI Coverage | Notes |
|---------|----------------------|------------|-------------|-------|
| `atspi` | `atspi = "0.28"` | `#[cfg(feature = "atspi")]` in manager.rs, injectors | Tested in text_injection_tests | Linux accessibility backend |
| `wl_clipboard` | `wl-clipboard-rs = "0.9"` | `#[cfg(feature = "wl_clipboard")]` in clipboard_injector.rs | Tested in text_injection_tests | Wayland clipboard |
| `enigo` | `enigo = "0.6"` | `#[cfg(feature = "enigo")]` in enigo_injector.rs | Tested in text_injection_tests | Cross-platform input simulation |
| `ydotool` | No deps | `#[cfg(feature = "ydotool")]` in ydotool_injector.rs | Tested in text_injection_tests | Linux uinput backend |
| `kdotool` | No deps | `#[cfg(feature = "kdotool")]` in kdotool_injector.rs | Tested in text_injection_tests | KDE-specific backend |
| `regex` | `regex = "1.10"` | `#[cfg(feature = "regex")]` in manager.rs, tests | Tested in text_injection_tests | Pattern matching for allow/block lists |
| `real-injection-tests` | No deps | `#[cfg(feature = "real-injection-tests")]` in real_injection.rs | Explicitly tested in text_injection_tests job | Requires X11/D-Bus setup |

### STT Features (crates/coldvox-stt/Cargo.toml)

| Feature | Cargo.toml Definition | Code Usage | CI Coverage | Notes |
|---------|----------------------|------------|-------------|-------|
| `vosk` | No deps | `#[cfg(feature = "vosk")]` in plugins/mod.rs | Default features | Plugin system |
| `whisper` | No deps | `#[cfg(feature = "whisper")]` in plugins/mod.rs | Not implemented yet | Future plugin |

### VAD Features (crates/coldvox-vad-silero/Cargo.toml)

| Feature | Cargo.toml Definition | Code Usage | CI Coverage | Notes |
|---------|----------------------|------------|-------------|-------|
| `silero` | `voice_activity_detector = { git = "...", optional = true }` | `#[cfg(feature = "silero")]` in lib.rs | Default features | Git dependency |

### GUI Features (crates/coldvox-gui/Cargo.toml)

| Feature | Cargo.toml Definition | Code Usage | CI Coverage | Notes |
|---------|----------------------|------------|-------------|-------|
| `qt-ui` | `cxx-qt = "0.7"`, `cxx-qt-lib = "0.7"` | `#[cfg(feature = "qt-ui")]` in main.rs, bridge.rs | Tested in gui-groundwork (optional) | Qt 6 required |

### Telemetry Features (crates/coldvox-telemetry/Cargo.toml)

| Feature | Cargo.toml Definition | Code Usage | CI Coverage | Notes |
|---------|----------------------|------------|-------------|-------|
| `text-injection` | `coldvox-text-injection = { optional = true }` | `#[cfg(feature = "text-injection")]` in pipeline_metrics.rs | Default features | Metrics integration |

## Feature Dependencies

### Default Feature Combinations

- **Default (crates/app/Cargo.toml)**: `["silero", "vosk", "text-injection"]`
  - Enables core functionality across all subsystems
  - Tested in build_and_check job

### Platform-Specific Features

- **Linux (target.'cfg(target_os = "linux")')**: `coldvox-text-injection` with `["atspi", "wl_clipboard", "ydotool"]`
- **Windows (target.'cfg(target_os = "windows")')**: `coldvox-text-injection` with `["enigo"]`
- **macOS (target.'cfg(target_os = "macos")')**: `coldvox-text-injection` with `["enigo"]`

### Test Feature Combinations

- **real-injection-tests**: Enables hardware interaction tests
  - Requires: `text-injection` + platform backends
  - Tested in text_injection_tests job with Xvfb/D-Bus

## CI Coverage Analysis

### Jobs and Feature Testing

1. **build_and_check**: Tests default features
2. **text_injection_tests**: Tests `real-injection-tests` + platform backends
3. **gui-groundwork**: Tests `qt-ui` (optional, skips if Qt missing)

### Gaps Identified

- No explicit feature matrix testing (all combinations)
- `live-hardware-tests` not run in CI
- No MSRV testing for feature compatibility
- Security audit disabled (`if: false`)

## Recommendations

1. Add feature matrix job to test all combinations
2. Enable `live-hardware-tests` in dedicated hardware runners
3. Add MSRV checks for feature compatibility
4. Re-enable security audit with proper token handling
5. Document feature deprecation/removal process

## Maintenance

- Update this matrix when adding/removing features
- Review quarterly for deprecated features
- Ensure CI updates when feature definitions change