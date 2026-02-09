---
doc_type: research
subsystem: text-injection
status: draft
freshness: historical
preservation: delete
last_reviewed: 2025-10-19
owners: Documentation Working Group
version: 1.0.0
---

# Plan to Trace Through Text Injection Pipeline

## Objective
To validate and refine my understanding of the text injection system by tracing the actual flow from transcription input to text injection.

## Key Entry Points to Trace

### 1. Main Entry Point: `InjectionProcessor::handle_transcription`
- File: `processor.rs`
- Function: `handle_transcription(event: TranscriptionEvent)`
- Purpose: Handle incoming transcription events from STT system

### 2. Session State Management: `InjectionSession`
- File: `session.rs`
- Key functions: `add_transcription`, `should_inject`, `take_buffer`
- Purpose: Manage buffering and timing of text injection

### 3. Strategy Selection: `StrategyManager::inject`
- File: `manager.rs`
- Function: `inject(text: &str)`
- Purpose: Select and execute injection methods with fallbacks

### 4. Injection Execution: Individual Injectors
- AT-SPI Injector: `injectors/atspi.rs`
- Clipboard Injector: `injectors/clipboard.rs`
- Ydotool Injector: `ydotool_injector.rs`

## Tracing Strategy

### Phase 1: Event Reception to Buffering
1. Start with `InjectionProcessor::handle_transcription`
2. Follow how `TranscriptionEvent::Final` is handled
3. Trace into `InjectionSession::add_transcription`
4. Understand state transitions (Idle → Buffering → WaitingForSilence → ReadyToInject)

### Phase 2: Injection Triggering
1. Follow `InjectionProcessor::check_and_inject`
2. Trace how `InjectionSession::should_inject` determines timing
3. Understand the role of silence timeouts and punctuation triggers

### Phase 3: Strategy Selection
1. Examine `StrategyManager::get_method_order_cached`
2. Understand how backend detection influences method selection
3. Trace the fallback chain logic

### Phase 4: Injection Execution
1. Follow the selected injector's `inject_text` method
2. Trace AT-SPI direct insertion path
3. Trace clipboard-based injection path
4. Understand the confirmation mechanism

### Phase 5: Confirmation and Fallback
1. Examine `confirm.rs` for injection verification
2. Understand how failures trigger fallback methods
3. Trace metrics collection and error handling

## Key Questions to Answer During Tracing

1. **Timing**: When exactly does injection happen after transcription?
2. **Context**: How does the system determine the target application?
3. **Fallbacks**: What are the exact conditions for trying each method?
4. **Confirmation**: How reliable is the injection confirmation?
5. **Error Handling**: What happens when all methods fail?
6. **Performance**: Where are the potential bottlenecks?

## Trace Points to Examine

### 1. Configuration Impact
- How does `InjectionConfig` affect the pipeline?
- What are the default behaviors?

### 2. Environment Detection
- How does `BackendDetector` influence strategy?
- What happens on Wayland vs X11?

### 3. Pre-warming Effects
- When does `PrewarmController` get called?
- What resources are pre-warmed?

### 4. Session State Transitions
- What triggers `Buffering` → `WaitingForSilence`?
- What triggers `WaitingForSilence` → `ReadyToInject`?

### 5. Method Selection Logic
- How are success rates tracked?
- How does cooldown affect method selection?

## Expected Flow Summary

Based on my current understanding:

1. STT → `TranscriptionEvent::Final` → `InjectionProcessor::handle_transcription`
2. → `InjectionSession::add_transcription` → state change to `Buffering`
3. → `InjectionProcessor::check_and_inject` → `InjectionSession::should_inject`
4. → `StrategyManager::inject` → method selection → actual injection
5. → `confirm::text_changed` → confirmation → metrics update

This plan will help me validate my understanding and identify any gaps or misconceptions in my analysis.