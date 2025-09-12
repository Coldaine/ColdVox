---
id: COLDVOX-ADR3-005-stt-plugin-architecture
type: ADR
level: 3
title: Speech-to-Text Plugin Architecture
status: accepted
owner: @team-stt
updated: 2025-09-11
parent: COLDVOX-DOM2-004-stt-engine
links:
  satisfies: [COLDVOX-DOM2-004-stt-engine]
  depends_on: []
  supersedes: []
  related_to: [COLDVOX-SPEC5-004-stt-engine-interface]
---

## Context
ColdVox needs to support multiple Speech-to-Text engines (Vosk, Whisper, cloud APIs) while maintaining a consistent interface and enabling runtime selection. A plugin architecture allows for extensibility without tightly coupling the core application to specific STT implementations.

## Decision
Implement a trait-based plugin architecture in `crates/coldvox-stt` that defines a common interface for all STT engines. Plugins are feature-gated and can be compiled in selectively. The main application uses direct Vosk integration for performance, but the plugin system provides a migration path for future engines.

## Status
Accepted

## Consequences
### Positive
- Enables support for multiple STT engines with a consistent interface
- Allows runtime selection and fallback between engines
- Provides clear separation between core pipeline and STT implementations
- Enables feature-gated compilation to reduce binary size
- Supports both offline (Vosk) and online (Whisper, cloud) engines

### Negative
- Plugin system adds complexity to the architecture
- Trait-based interface may limit some engine-specific optimizations
- Feature gating requires careful dependency management
- Current main app doesn't use the plugin system (direct Vosk integration)

## Implementation
The plugin architecture defines:
- `SttPlugin` trait with methods for initialization, transcription, and shutdown
- `PluginInfo` struct for metadata about each plugin
- `SttPluginError` enum for standardized error handling
- Plugin registry for discovering and loading available plugins
- Feature gates for conditional compilation (`vosk-plugin`, `whisper-plugin`)

Plugins are implemented in `crates/coldvox-stt/src/plugins/`:
- `mock_plugin.rs` - Testing plugin
- `noop_plugin.rs` - No-operation plugin
- `vosk_plugin.rs` - Vosk STT plugin (feature-gated)
- `whisper_plugin.rs` - Whisper STT plugin (feature-gated)

## Alternatives Considered
1. Direct integration of each STT engine - Would tightly couple the application to specific implementations
2. Compile-time engine selection only - Would require separate binaries for different engines
3. Runtime dynamic loading (DLLs) - Would add platform complexity and security considerations

## Related Documents
- `crates/coldvox-stt/src/plugin.rs`
- `crates/coldvox-stt/src/plugins/`
- `crates/app/src/stt/vosk.rs` (current direct integration)
- `COLDVOX-SPEC5-004-stt-engine-interface.md`

---
satisfies: COLDVOX-DOM2-004-stt-engine  
depends_on:   
supersedes:   
related_to: COLDVOX-SPEC5-004-stt-engine-interface