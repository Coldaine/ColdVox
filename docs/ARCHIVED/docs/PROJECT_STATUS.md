# ColdVox Project Status

## Current Phase: STT Integration Enhancement

Most workspace refactoring is complete. The active focus is improving the STT layer (Vosk defaults, streaming behavior, and preparing for additional backends) and polishing text‑injection ergonomics.

Note: The README status badge is manually maintained. When updating this phase, also update the badge in `README.md` to keep them in sync.

Platform scope (prototype): Linux only, specifically Nobara (KDE Plasma). Cross‑platform goals remain, but validation and instructions currently assume this environment.

### Completed Work

#### Phase 1: Critical Bug Fixes ✅
- Fixed audio capture watchdog stability
- Resolved frame processing issues
- Improved error handling and recovery

#### Phase 2: Ring Buffer Implementation ✅
- Implemented lock-free SPSC ring buffer
- Improved backpressure handling
- Added comprehensive metrics and monitoring

#### Phase 3: Audio Pipeline Refactoring ✅
- Centralized resampling in AudioChunker
- Separated capture from processing concerns
- Implemented proper stereo-to-mono conversion

#### Phase 4: Multi-Crate Workspace ✅
- Split monolithic codebase into focused crates
- Established clear module boundaries
- Improved build times and testability

### Current Work

#### Phase 6: STT Integration Enhancement 🔄
- Default Vosk integration with model autodetect in runtime
- Streaming transcription pipeline in place (partial/final events)
- CI E2E WAV test documented and runnable with vendored libvosk
- Prepare for multi‑backend support (Whisper stub plugin present)

### Upcoming Work

#### Phase 7: GUI Development
- Design cross-platform GUI interface
- Implement configuration management
- Add real-time visualization

## Architecture Overview

```
coldvox/
├── crates/
│   ├── app/                    # Main application
│   ├── coldvox-foundation/     # Core types and utilities
│   ├── coldvox-audio/          # Audio capture and processing
│   ├── coldvox-vad/            # Voice Activity Detection
│   ├── coldvox-vad-silero/     # Silero VAD implementation
│   ├── coldvox-stt/            # Speech-to-text framework
│   ├── coldvox-stt-vosk/       # Vosk STT implementation
│   ├── coldvox-text-injection/ # Text injection backends
│   ├── coldvox-telemetry/      # Metrics and monitoring
│   └── coldvox-gui/            # GUI components
└── examples/                   # Example applications
```

## Known Issues / Remaining Items

- Text injection: AT‑SPI app identification fallback and regex caching optimizations are still TODOs
- Some platform-specific injection backends need broader testing across environments
- Whisper plugin is a stub (no functional transcription yet); multi‑backend selection remains future work
- Documentation updates continue as crates evolve

## Recently Completed

#### Phase 5: Text Injection Unification ✅
- Unified TextInjector to async‑first design (`crates/coldvox-text-injection`)
- Consolidated injection strategies and fallback chaining
- Platform-aware backend selection wiring from the main app

#### Phase 4: Multi-Crate Workspace ✅
- Split monolithic codebase into focused crates
- Established clear module boundaries
- Improved build times and testability

## Contributing

The project is in active development. Please check the issue tracker for ways to contribute.
