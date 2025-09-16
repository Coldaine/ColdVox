# STT Architecture Unification Plan

## Executive Summary

This plan outlines the consolidation of ColdVox's dual STT architectures (batch vs streaming) into a single unified processor. The current implementation has unnecessary duplication and fragile separation that creates maintenance burden without functional benefit.

## Acknowledgment of Technical Review

A detailed technical review identified several valid points that this plan incorporates:
- Emphasis on removing per-frame allocations (~31 frames/sec overhead in streaming path)
- Recognition that PluginSttProcessor already has unused streaming capability
- Clarification that runtime switching is optional (startup configuration sufficient initially)
- Focus on evolving existing code rather than creating new abstractions

## Problem Statement

### Current State Issues

1. **Artificial Duplication**: Two separate processors (`PluginSttProcessor` and `StreamingSttProcessor`) handling essentially the same task
2. **Type Redundancy**: Duplicate types (`AudioFrame` vs `StreamingAudioFrame`, `VadEvent` vs `StreamingVadEvent`)
3. **Conversion Overhead**: Spawned tasks just to convert between identical data types
4. **Testing Complexity**: E2E tests need different setups for each path
5. **Maintenance Burden**: Every STT change must be made in two places

### Root Cause Analysis

The dual architecture was introduced as a "gradual migration" but implemented as completely separate systems. The only real difference is buffering strategy:
- **Batch**: Buffer frames until SpeechEnd, then process entire segment
- **Streaming**: Process frames incrementally as they arrive

This is a 10-line behavioral difference, not a fundamental architectural difference.

## Solution Overview

Create a single `UnifiedSttProcessor` that:
- Uses one set of types and channels
- Shares all infrastructure (plugin manager, metrics, etc.)
- Switches processing behavior via runtime mode flag
- Provides clean mode transitions with documented state handling

## Detailed Implementation Plan

### Phase 1: Create Unified Types and Infrastructure

#### 1.1 Define STT Processing Mode Enum
**File:** `crates/coldvox-stt/src/types.rs`

```rust
/// STT processing mode determines how audio is buffered and processed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SttProcessingMode {
    /// Buffer all audio until speech ends, then process entire segment
    /// - Higher accuracy due to full context
    /// - Higher latency (waits for speech end)
    Batch,

    /// Process audio incrementally as it arrives
    /// - Lower latency for real-time feedback
    /// - Provides partial transcriptions during speech
    Streaming,
}

impl Default for SttProcessingMode {
    fn default() -> Self {
        Self::Batch // Conservative default
    }
}
```

#### 1.2 Unify Audio Frame Types
**File:** `crates/coldvox-stt/src/types.rs`

**Approach: Use existing types and remove conversions**

Rather than creating new unified types, we'll:
1. Remove `StreamingAudioFrame` re-export from `lib.rs`
2. Use `coldvox_audio::AudioFrame` directly everywhere
3. Eliminate the per-frame conversion task that allocates Vec<i16> (runtime.rs:453-464)
4. Leverage the existing conversion in PluginSttProcessor (lines 483-488) which already handles this

```rust
// Remove these re-exports from crates/coldvox-stt/src/lib.rs:
// pub use streaming_processor::{StreamingAudioFrame, StreamingVadEvent};

// Use AudioFrame directly in the unified processor
use coldvox_audio::AudioFrame;  // No new types needed
```

**Performance Note**: The current streaming path allocates ~50KB/sec (31 frames × 512 samples × 2 bytes) unnecessarily. Removing this conversion eliminates the allocation overhead.

#### 1.3 Use Existing VAD Events
- Remove `StreamingVadEvent` duplicate entirely
- Use `coldvox_vad::VadEvent` everywhere
- Update imports to remove streaming variant

### Phase 2: Evolve Existing STT Processor

#### 2.1 Evolve PluginSttProcessor (Not Create New)
**File:** `crates/app/src/stt/processor.rs` (evolve existing)

Rather than creating a new processor, we'll evolve the existing `PluginSttProcessor` which already has:
- The infrastructure we need (channels, metrics, run loop)
- Unused streaming capability (lines 478-519)
- Plugin manager integration
- Retry logic and error handling

**Key insight**: The existing processor already supports both modes via `config.streaming` but the streaming path bypasses it entirely.

```rust
// Current PluginSttProcessor already has this structure:
pub struct SttProcessor {  // Rename from PluginSttProcessor
    // Add mode field
    mode: SttProcessingMode,

    // Existing fields (keep as-is)
    audio_rx: broadcast::Receiver<AudioFrame>,
    vad_rx: mpsc::Receiver<VadEvent>,
    event_tx: mpsc::Sender<TranscriptionEvent>,
    plugin_manager: Arc<RwLock<SttPluginManager>>,
    // ... existing state, metrics, etc.
}

#### 2.2 Add Buffer Size Limits
Add safeguards to prevent unbounded growth:

```rust
impl SttProcessor {
    const MAX_BUFFER_SAMPLES: usize = 16000 * 30; // 30 seconds at 16kHz

    async fn handle_audio_frame(&mut self, frame: AudioFrame) {
        match self.state {
            UtteranceState::SpeechActive { ref mut audio_buffer, .. } => {
                // Add buffer size check
                if audio_buffer.len() + frame.samples.len() > Self::MAX_BUFFER_SAMPLES {
                    warn!("Audio buffer would exceed max size, processing early");
                    self.process_buffered_audio().await;
                    audio_buffer.clear();
                }

                // Use existing logic - already handles streaming mode
                if self.config.streaming {
                    // Use existing streaming code (lines 481-519)
                } else {
                    // Buffer for batch processing
                }
            }
        }
    }
}
```

#### 2.3 Remove Separate StreamingSttProcessor
**File:** `crates/coldvox-stt/src/streaming_processor.rs` (delete after migration)

This file becomes obsolete since the evolved `SttProcessor` handles both modes.

                // Shutdown signal
                _ = self.shutdown_rx.recv() => {
                    info!("STT processor shutdown requested");
                    self.cleanup().await;
                    break;
                }
            }
        }

        info!("STT processor stopped");
    }
}
```

#### 2.3 Mode-Specific Audio Handling

```rust
async fn handle_audio_frame(&mut self, frame: AudioFrame) {
    let mode = self.mode.read().await;
    let mut state = self.state.write().await;

    // Skip processing during mode transitions
    if state.is_switching {
        debug!("Dropping frame during mode switch");
        return;
    }

    // Only process during speech
    if !matches!(state.utterance_state, UtteranceState::SpeechActive { .. }) {
        return;
    }

    state.metrics.frames_in += 1;

    match *mode {
        SttProcessingMode::Batch => {
            // Buffer frames for end-of-speech processing
            let stt_frame = SttAudioFrame::from(frame);
            state.audio_buffer.extend_from_slice(&stt_frame.samples_i16);

            if let UtteranceState::SpeechActive { ref mut frames_processed, .. } = state.utterance_state {
                *frames_processed += 1;
            }
        }

        SttProcessingMode::Streaming => {
            // Process immediately
            let stt_frame = SttAudioFrame::from(frame);
            drop(state); // Release lock before async call

            let result = {
                let mut pm = self.plugin_manager.write().await;
                pm.process_audio(&stt_frame.samples_i16).await
            };

            if let Ok(Some(event)) = result {
                let _ = self.event_tx.send(event).await;
            }

            // Update metrics
            let mut state = self.state.write().await;
            if let UtteranceState::SpeechActive { ref mut frames_processed, .. } = state.utterance_state {
                *frames_processed += 1;
            }
        }
    }
}
```

### Phase 3: Simplified Mode Configuration (Startup-Only Initially)

#### 3.1 Startup Configuration Approach

**Simplified approach**: Start with startup-only mode configuration rather than complex runtime switching:

```rust
// In runtime.rs startup, derive mode from environment:
let stt_mode = match env::var("COLDVOX_STT_MODE").unwrap_or_default().as_str() {
    "streaming" => SttProcessingMode::Streaming,
    "batch" | _ => SttProcessingMode::Batch,  // Default to batch
};

// Set config.streaming from mode (not hardcoded per path)
let stt_config = TranscriptionConfig {
    streaming: matches!(stt_mode, SttProcessingMode::Streaming),
    ..Default::default()
};
```

#### 3.2 Optional Runtime Switching (Future Enhancement)

Runtime switching can be added later if needed, with these safeguards:
- Only allow switches when idle (no active speech)
- Use state polling instead of arbitrary sleeps
- Queue mode changes during active speech
- Add cooldown to prevent rapid switching

```rust
// Future enhancement - not required for initial unification
pub async fn set_stt_mode(&self, mode: SttProcessingMode) -> Result<(), String> {
    // Guard against switching during active speech
    if self.is_speech_active().await {
        return Err("Cannot switch STT mode during active speech".to_string());
    }
    // ... rest of switching logic
}
```

### Phase 4: Runtime Integration - Remove 140-Line Branching

#### 4.1 Primary Goal: Eliminate Conversion Overhead

**File:** `crates/app/src/runtime.rs` (lines 403-543)

The critical change is removing the entire branching section that causes:
- Per-frame Vec<i16> allocations (~50KB/sec in streaming)
- Duplicate fanout tasks
- Separate channel setups
- Type conversions

**Before (Current):**
```rust
if stt_arch == "batch" {
    // 40 lines of batch setup with PluginSttProcessor
    // Sets streaming: false
} else {
    // 85 lines of streaming setup with conversions
    // Spawns conversion task (lines 450-466)
    // Creates StreamingAudioFrame/StreamingVadEvent
    // Uses StreamingSttProcessor + ManagerStreamingAdapter
}
```

**After (Unified):**
```rust
// Single setup path regardless of mode
let stt_config = TranscriptionConfig {
    streaming: matches!(stt_mode, SttProcessingMode::Streaming),
    ..Default::default()
};

let processor = SttProcessor::new(  // evolved from PluginSttProcessor
    audio_tx.subscribe(),           // Direct AudioFrame, no conversions
    stt_vad_rx,                     // Direct VadEvent, no conversions
    stt_tx.clone(),
    plugin_manager.clone(),
    stt_config,
);

// Single VAD fanout (no duplicate)
let vad_fanout_handle = tokio::spawn(async move {
    let mut rx = raw_vad_rx;
    while let Some(ev) = rx.recv().await {
        let _ = vad_bcast_tx.send(ev);      // Broadcast to VAD consumers
        let _ = stt_vad_tx.send(ev).await;  // Direct to STT (no conversion)
    }
});
```

#### 4.2 Remove Files After Migration

**Files to delete:**
- `crates/coldvox-stt/src/streaming_processor.rs` (obsolete)
- `crates/app/src/stt/streaming_adapter.rs` (obsolete)

**Files to update:**
- `crates/coldvox-stt/src/lib.rs` - Remove StreamingAudioFrame/StreamingVadEvent re-exports
- `crates/app/src/stt/processor.rs` - Rename to SttProcessor, expose mode field

**Key Performance Improvement:**
This change eliminates the ~50KB/sec allocation overhead from per-frame conversions (runtime.rs:453-464) by using AudioFrame directly throughout the pipeline.

### Phase 5: UI Integration

#### 5.1 Add to TUI Dashboard

**File:** `crates/app/src/bin/tui_dashboard.rs`

Add STT mode control and display:

```rust
// Add to key handling
async fn handle_input(key: KeyCode, state: &mut DashboardState) -> bool {
    match key {
        // ... existing key handlers ...

        KeyCode::Char('m') | KeyCode::Char('M') => {
            // Toggle STT mode
            let current = state.app_handle.get_stt_mode().await;
            let new_mode = match current {
                SttProcessingMode::Batch => SttProcessingMode::Streaming,
                SttProcessingMode::Streaming => SttProcessingMode::Batch,
            };

            state.status_message = format!("Switching STT mode to {:?}...", new_mode);

            match state.app_handle.set_stt_mode(new_mode).await {
                Ok(_) => {
                    state.status_message = format!("STT mode: {:?}", new_mode);
                    state.last_status_update = std::time::Instant::now();
                }
                Err(e) => {
                    state.status_message = format!("Failed to switch STT mode: {}", e);
                    state.last_status_update = std::time::Instant::now();
                }
            }
        }

        // ... rest of handlers ...
    }
}

// Add to status display
fn draw_status_bar(f: &mut Frame, area: Rect, state: &DashboardState) {
    let current_mode = tokio::task::block_in_place(|| {
        Handle::current().block_on(state.app_handle.get_stt_mode())
    });

    let status_text = format!(
        "STT: {:?} | Press M to toggle | {} | Q: Quit",
        current_mode,
        state.status_message
    );

    let status = Paragraph::new(status_text)
        .style(Style::default().fg(Color::Yellow))
        .wrap(Wrap { trim: true });
    f.render_widget(status, area);
}
```

#### 5.2 Add Help Text

Update help display to include STT mode controls:

```rust
fn draw_help(f: &mut Frame, area: Rect) {
    let help_text = vec![
        "ColdVox Controls:",
        "",
        "S - Start/Stop pipeline",
        "A - Toggle VAD/PTT mode",
        "M - Toggle STT mode (Batch/Streaming)",
        "R - Reset pipeline",
        "Q - Quit",
        "",
        "STT Modes:",
        "  Batch: Buffer until speech ends (higher accuracy)",
        "  Streaming: Process incrementally (lower latency)",
    ];

    // ... render help_text ...
}
```

### Phase 6: Documentation

#### 6.1 Create Mode Documentation

**File:** `docs/stt-processing-modes.md`

```markdown
# STT Processing Modes

## Overview

ColdVox supports two Speech-to-Text (STT) processing modes that can be switched at runtime:

## Batch Mode (Default)

**How it works:**
- Buffers all audio frames during a speech segment
- Processes the entire audio buffer when speech ends (VAD SpeechEnd event)
- Provides complete context to the STT engine

**Advantages:**
- Higher transcription accuracy due to full utterance context
- No processing during speech (waits until end)
- More reliable for longer utterances

**Trade-offs:**
- Higher latency (must wait for speech to end)
- No incremental results during speech
- Buffers audio for long utterances

**Best for:**
- Dictation and document creation
- Scenarios where accuracy is more important than speed
- Users who speak in complete sentences

## Streaming Mode

**How it works:**
- Processes audio frames incrementally as they arrive
- Provides partial transcription results during speech
- Updates transcription in real-time

**Advantages:**
- Lower latency (immediate feedback)
- Partial results available during speech
- Better user experience for interactive applications

**Trade-offs:**
- Potentially lower accuracy (less context)
- Continuous processing during speech
- More complex result handling

**Best for:**
- Real-time transcription display
- Interactive voice applications
- Users who want immediate feedback

## Mode Switching

### Methods to Switch Modes

1. **Environment Variable** (startup only):
   ```bash
   COLDVOX_STT_MODE=batch ./coldvox
   COLDVOX_STT_MODE=streaming ./coldvox
   ```

2. **TUI Dashboard** (runtime):
   - Press 'M' to toggle between modes
   - Current mode displayed in status bar

3. **Programmatic** (API):
   ```rust
   app_handle.set_stt_mode(SttProcessingMode::Streaming).await?;
   ```

### What Happens During Mode Switch

When switching STT modes:

1. **Transition begins**: Current mode processing stops
2. **State cleanup**:
   - Any active transcription is canceled
   - Audio buffers are cleared
   - Plugin state is reset
3. **Error notification**: If transcription was active, an interruption event is sent
4. **Mode activation**: New mode becomes active
5. **Ready state**: Next utterance will use the new mode

### Important Notes

- **Mode switches are immediate but clean**
- **In-flight transcriptions are lost** during the switch
- **Recommended to switch between utterances** for best experience
- **Mode persists until changed** (not reset on restart)
- No functional impact when not switching

### Choosing the Right Mode

| Use Case | Recommended Mode | Reason |
|----------|------------------|---------|
| Document dictation | Batch | Higher accuracy for complete thoughts |
| Live captions | Streaming | Immediate feedback for viewers |
| Voice commands | Batch | Accuracy important for command recognition |
| Interactive chat | Streaming | Real-time conversation flow |
| Transcription accuracy test | Batch | Maximum context for STT engine |
| Responsiveness test | Streaming | Immediate partial results |

## Technical Details

Both modes use the same underlying infrastructure:
- Same plugin manager and STT engines
- Same audio processing pipeline
- Same VAD (Voice Activity Detection)
- Same transcription output format

The only difference is **when** and **how** audio is sent to the STT engine.
```

#### 6.2 Update Main Documentation

Update `CLAUDE.md` and `README.md` to mention unified STT architecture and mode switching capabilities.

### Phase 7: Testing Strategy

#### 7.1 Unit Tests for Mode Switching

**File:** `crates/app/src/stt/unified_processor_tests.rs`

```rust
#[tokio::test]
async fn test_mode_switch_during_idle() {
    // Setup processor in batch mode
    let processor = create_test_processor(SttProcessingMode::Batch).await;

    // Switch to streaming while idle
    processor.switch_mode(SttProcessingMode::Streaming).await;

    // Verify mode changed and state is clean
    assert_eq!(processor.get_mode().await, SttProcessingMode::Streaming);
    assert!(processor.is_idle().await);
}

#[tokio::test]
async fn test_mode_switch_during_speech() {
    let mut processor = create_test_processor(SttProcessingMode::Batch).await;

    // Start speech
    processor.handle_vad_event(VadEvent::SpeechStart {
        timestamp_ms: 1000,
        energy_db: -20.0
    }).await;

    // Send some audio frames
    for _ in 0..5 {
        processor.handle_audio_frame(create_test_frame()).await;
    }

    // Switch mode during speech
    processor.switch_mode(SttProcessingMode::Streaming).await;

    // Verify transcription was interrupted and state reset
    assert_eq!(processor.get_mode().await, SttProcessingMode::Streaming);
    assert!(processor.is_idle().await);
    // Should have received interruption event
}

#[tokio::test]
async fn test_rapid_mode_switches() {
    let mut processor = create_test_processor(SttProcessingMode::Batch).await;

    // Rapidly switch modes
    for _ in 0..10 {
        processor.switch_mode(SttProcessingMode::Streaming).await;
        processor.switch_mode(SttProcessingMode::Batch).await;
    }

    // Should end up in stable state
    assert_eq!(processor.get_mode().await, SttProcessingMode::Batch);
    assert!(processor.is_idle().await);
}
```

#### 7.2 Integration Tests

**File:** `crates/app/tests/stt_mode_integration.rs`

```rust
#[tokio::test]
async fn test_batch_vs_streaming_equivalence() {
    // Test that both modes produce equivalent results for same input
    let test_audio = load_test_audio("test_utterance.wav");

    let batch_result = process_with_mode(test_audio.clone(), SttProcessingMode::Batch).await;
    let streaming_result = process_with_mode(test_audio, SttProcessingMode::Streaming).await;

    // Results should be equivalent (allowing for minor differences)
    assert_transcripts_equivalent(&batch_result.text, &streaming_result.text);
}

#[tokio::test]
async fn test_mode_persistence_across_restarts() {
    // Set mode to streaming
    let app = start_test_app().await;
    app.set_stt_mode(SttProcessingMode::Streaming).await.unwrap();

    // Simulate restart
    app.shutdown().await;
    let app2 = start_test_app().await;

    // Mode should persist
    assert_eq!(app2.get_stt_mode().await, SttProcessingMode::Streaming);
}
```

#### 7.3 End-to-End Tests

Update existing E2E tests to cover both modes:

```rust
#[tokio::test]
async fn end_to_end_unified_stt_pipeline() {
    for mode in [SttProcessingMode::Batch, SttProcessingMode::Streaming] {
        // Test with each mode
        let mut app = create_test_app_with_mode(mode).await;

        // Run standard E2E test
        test_complete_pipeline(&mut app).await;

        app.shutdown().await;
    }
}
```

### Phase 8: Migration and Cleanup

#### 8.1 Remove Old Code (Behind Feature Flag Initially)

Create feature flag for transition period:

```toml
# Cargo.toml
[features]
default = ["silero", "vosk", "text-injection", "unified-stt"]
unified-stt = []
legacy-dual-stt = []
```

#### 8.2 Deprecation Path

1. **Week 1-2**: Deploy unified processor alongside old code (feature flag)
2. **Week 3-4**: Default to unified processor, keep legacy as fallback
3. **Week 5-6**: Remove legacy code entirely

#### 8.3 Files to Remove (After Migration)

- `crates/app/src/stt/processor.rs` (PluginSttProcessor)
- `crates/coldvox-stt/src/streaming_processor.rs` (StreamingSttProcessor)
- `crates/app/src/stt/streaming_adapter.rs` (ManagerStreamingAdapter)
- Type definitions: `StreamingAudioFrame`, `StreamingVadEvent`
- Conversion tasks and branching logic in `runtime.rs`

#### 8.4 Update Examples and Documentation

- Update all examples to use unified processor
- Update API documentation
- Create migration guide for external users

## Benefits Summary

### Code Quality
- **Reduced STT-related duplication**
- **Single test suite** covers both processing modes
- **Unified type system** removes conversion tasks
- **Clear separation of concerns** between buffering strategy and infrastructure

### Maintainability
- **Single implementation** to maintain and debug
- **Shared infrastructure** reduces testing surface
- **Clear mode transition protocol** prevents state corruption
- **Documented behavior** for mode switching

### User Experience
- **Runtime mode switching** without application restart
- **Visual feedback** in TUI for current mode
- **Clean transitions** with documented side effects
- **Performance characteristics** clearly documented per mode

### Architecture
- **Removes conversion tasks** (simplified data flow)
- **Reduces channel proliferation** (cleaner data flow)
- **Shared plugin manager** (unified resource management)
- **Mode-specific behavior** possible within single codebase

## Risk Mitigation

### Development Risks
1. **Feature flag protection** during migration period
2. **Extensive testing** of both modes and transitions
3. **Gradual rollout** with fallback to legacy code
4. **Clear rollback plan** using git history

### Operational Risks
1. **Conservative defaults** (batch mode, existing behavior)
2. **Runtime monitoring** of mode switches and errors
3. **Documentation** of exactly what changes during transitions
4. **Backward compatibility** for environment variables and APIs

### Quality Risks
1. **Equivalent functionality testing** between old and new processors
2. **Performance regression testing** for both modes
3. **Memory leak testing** during mode transitions
4. **Error handling testing** for edge cases

## Implementation Approach

### Phase Breakdown
- **Phase 1-2**: Core unification - type system and processor
- **Phase 3-4**: Runtime integration - mode switching and API
- **Phase 5-6**: UI and documentation - user interface and guides
- **Phase 7**: Testing - comprehensive test coverage
- **Phase 8**: Migration and cleanup - remove old code

### Dependencies
- No external dependencies
- No breaking API changes required
- Can be developed incrementally alongside existing code

## Success Criteria

### Functional
- [ ] Both processing modes work identically to current implementation
- [ ] Mode switching works cleanly with documented behavior
- [ ] Equivalent functionality in either mode
- [ ] All existing tests pass with unified processor

### Code Quality
- [ ] Single STT processor implementation
- [ ] No duplicate types or conversion tasks
- [ ] Clean mode transition protocol
- [ ] Comprehensive test coverage

### Documentation
- [ ] Clear mode selection guidance
- [ ] Documented transition behavior
- [ ] Updated examples and tutorials
- [ ] Migration guide for any external users

This plan transforms the fragile dual-architecture system into a clean, maintainable, and user-friendly unified processor while preserving all existing functionality and adding runtime configurability.