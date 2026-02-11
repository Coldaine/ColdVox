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

# Conceptual Summary of ColdVox Text Injection System

## System Overview

The ColdVox text injection system is a sophisticated, multi-layered component designed to reliably inject speech-to-text transcriptions into active Linux desktop applications. It employs multiple injection strategies with intelligent fallback mechanisms to ensure compatibility across diverse desktop environments (Wayland/X11) and applications.

## Core Design Principles

### 1. Adaptive Strategy Selection
The system dynamically selects the most appropriate injection method based on:
- Current desktop environment (Wayland vs X11)
- Target application characteristics
- Historical success rates per application-method combination
- Current system capabilities and permissions

### 2. Robust Fallback Chain
Multiple injection methods are tried in sequence until one succeeds:
1. **AT-SPI Direct Insertion** (Preferred) - Uses accessibility API for direct text insertion
2. **Kdotool Assistance** (Optional) - Window activation/focus assistance for KDE/X11
3. **Enigo Text** (Optional) - Cross-platform input simulation
4. **Clipboard Paste** (Fallback) - Seed/restore clipboard with paste action
5. **NoOp** (Last Resort) - Always succeeds but does nothing

### 3. State-Based Session Management
Transcriptions are buffered and injected based on a state machine:
- **Idle**: No active session, waiting for first transcription
- **Buffering**: Actively receiving transcriptions
- **WaitingForSilence**: No new transcriptions, waiting for timeout
- **ReadyToInject**: Silence timeout reached, ready to inject

### 4. Performance Optimization
- **Pre-warming**: Resources are prepared in advance to minimize latency
- **TTL Caching**: Pre-warmed data is cached with 3-second TTL
- **Cooldown Management**: Failed methods enter exponential backoff
- **Metrics Collection**: Comprehensive performance tracking

## Key Components

### 1. InjectionProcessor (`processor.rs`)
The main entry point that handles transcription events and coordinates the injection process. It manages the session state and triggers injection when appropriate.

### 2. InjectionSession (`session.rs`)
Manages the state machine for buffering transcriptions and determining injection timing. It handles text normalization, punctuation detection, and buffer size limits.

### 3. StrategyManager (`manager.rs`)
The "brain" of the system that selects and executes injection methods. It maintains success/failure statistics, manages cooldowns, and handles the fallback chain.

### 4. TextInjector Implementations
Multiple injector implementations provide different injection strategies:
- **AtspiInjector** (`injectors/atspi.rs`) - AT-SPI accessibility API
- **ClipboardInjector** (`injectors/clipboard.rs`) - Clipboard seed/restore
- **YdotoolInjector** (`ydotool_injector.rs`) - uinput automation
- **EnigoInjector** (`enigo_injector.rs`) - Input simulation

### 5. Supporting Infrastructure
- **BackendDetector** (`backend.rs`) - Detects available system capabilities
- **FocusTracker** (`focus.rs`) - Determines target application focus
- **Confirmation Module** (`confirm.rs`) - Verifies successful injection
- **PrewarmController** (`prewarm.rs`) - Pre-warms resources for reduced latency

## Flow Summary

1. **Event Reception**: STT system emits `TranscriptionEvent::Final` with transcribed text
2. **Buffering**: `InjectionSession` buffers transcriptions and manages state transitions
3. **Timing**: When silence is detected (or other triggers occur), session transitions to `ReadyToInject`
4. **Strategy Selection**: `StrategyManager` selects injection method based on environment and success rates
5. **Pre-warming**: `PrewarmController` ensures necessary resources are prepared
6. **Injection**: Selected injector performs text injection using its specific mechanism
7. **Confirmation**: System attempts to confirm successful injection via AT-SPI events
8. **Fallback**: If injection fails, next method in the fallback chain is tried
9. **Metrics**: Performance data is collected and used to inform future strategy selection

## Environment Adaptation

### Wayland Support
- Primary method: AT-SPI direct insertion via accessibility API
- Fallback: Clipboard paste with AT-SPI paste actions
- Optional: Enigo input simulation (if libei support available)
- Portal integration: xdg-desktop-portal VirtualKeyboard support

### X11 Support
- Primary method: AT-SPI direct insertion
- Fallback: Clipboard paste with xdotool/ydotool key events
- Optional: Kdotool window activation for KDE
- Native X11 input where available

## Reliability Features

### Error Handling
- Per-method timeout protection
- Exponential backoff cooldown for failed methods
- Graceful degradation when capabilities are missing
- Comprehensive error reporting and metrics

### Confirmation Mechanism
- AT-SPI text change event monitoring
- Prefix matching for injection verification
- Bounded wait times (75ms default)
- Graceful handling of confirmation failures

### Performance Optimization
- Resource pre-warming to minimize first-shot latency
- Caching of expensive operations (connections, focus detection)
- Parallel execution of independent pre-warming steps
- Efficient state management with minimal allocations

## Configuration

The system is highly configurable through `InjectionConfig`, allowing customization of:
- Method preferences and enablement
- Timeout values and retry behavior
- Silence detection parameters
- Performance optimization settings
- Privacy controls (log redaction)
- Application allow/block lists

## Integration Points

The text injection system is designed to integrate with:
- **STT Processors**: Receives `TranscriptionEvent` from speech-to-text engines
- **Audio Pipeline**: Coordinates with voice activity detection
- **GUI Components**: Provides status and metrics for display
- **Configuration System**: Loads and applies user settings
- **Telemetry**: Reports performance and error metrics

This system represents a comprehensive solution for reliable text injection across the diverse landscape of Linux desktop environments, with intelligent adaptation to different capabilities and careful attention to user experience through minimal disruption and reliable operation.