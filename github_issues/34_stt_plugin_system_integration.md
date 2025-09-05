# [STT] Integrate plugin system for extensible speech recognition engines

**Priority:** High

## Problem Description
The current STT implementation directly uses Vosk through `SttProcessor` rather than leveraging the existing plugin system in `coldvox-stt`. This limits the ability to easily add or switch between different STT engines, making the system less extensible and harder to customize for different use cases.

## Impact
- **High**: Limits extensibility and makes it difficult to support multiple STT engines
- Prevents users from choosing their preferred STT engine
- Makes it harder to add new STT engines without modifying core code
- Reduces code reusability and maintainability

## Reproduction Steps
1. Examine `crates/app/src/stt/processor.rs`
2. Note that it directly creates a `VoskTranscriber` rather than using the plugin registry
3. Compare with the plugin system in `crates/coldvox-stt/src/plugin.rs`
4. Check the plugin manager in `crates/app/src/stt/plugin_manager.rs`

## Expected Behavior
The STT processor should:
- Use the plugin registry to discover available STT engines
- Allow configuration to specify preferred STT plugins
- Implement proper fallback mechanisms when preferred plugins are unavailable
- Support runtime switching between different STT engines

## Current Behavior
The STT processor directly instantiates a Vosk transcriber without going through the plugin system, limiting extensibility.

## Proposed Solution
1. Modify `SttProcessor` to use the plugin registry instead of directly instantiating Vosk
2. Add configuration options to specify preferred STT plugins
3. Implement proper fallback mechanisms when preferred plugins are unavailable
4. Update the plugin manager to handle STT plugin lifecycle
5. Add tests to verify plugin loading and fallback behavior

## Implementation Steps
1. Update `SttProcessor::new()` to accept a plugin registry reference
2. Modify `SttProcessor::run()` to use the plugin registry for STT engine selection
3. Add configuration options for preferred STT engines
4. Implement plugin loading and error handling
5. Update the application initialization to pass the plugin registry to STT processor
6. Add comprehensive tests for plugin functionality

## Acceptance Criteria
- [ ] STT processor uses plugin registry for engine selection
- [ ] Configuration options for preferred STT engines
- [ ] Proper fallback mechanisms when preferred plugins are unavailable
- [ ] Support for multiple STT engines without code changes
- [ ] Comprehensive test coverage for plugin functionality

## Related Files
- `crates/app/src/stt/processor.rs`
- `crates/coldvox-stt/src/plugin.rs`
- `crates/app/src/stt/plugin_manager.rs`
- `crates/coldvox-stt-vosk/src/vosk_transcriber.rs`
