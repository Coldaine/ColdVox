---
doc_type: troubleshooting
subsystem: gui
version: 1.0.0
status: draft
owners: Documentation Working Group
last_reviewed: 2025-10-19
---

/# ColdVox Architecture Diagram - Updated 2025-09-03

```mermaid
flowchart TD
    %% External inputs
    MIC[Audio Input Device] --> AC[AudioCapture]
    HK[Global Hotkeys] --> |VadEvent| EVENTS[VAD Event Channel]

    %% Audio capture and preprocessing
    AC --> |CPAL callback| ARB[AudioRingBuffer]
    ARB --> |Consumer| FR[FrameReader]
    FR --> |Variable frames| CHUNKER[AudioChunker]

    %% Broadcast distribution system
    CHUNKER --> |512 sample chunks| BROADCAST{Broadcast Channel<br/>AudioFrame}

    %% VAD processing branch
    BROADCAST --> |Subscribe| VAD[VadProcessor]
    VAD --> |VadAdapter| VADENG{VAD Engine}
    VADENG --> |SileroEngine (default)| SILERO[SileroEngine<br/>ML-based VAD]
    VADENG --> |Level3 (disabled)| ENERGY[Level3Vad<br/>Energy-based VAD]

    %% VAD state management
    VAD --> |VAD Events| VADFSM[VadStateMachine<br/>Debouncing]
    VADFSM --> |SpeechStart/End| EVENTS

    %% STT processing branch
    BROADCAST --> |Subscribe| STT[SttProcessor]
    EVENTS --> STT
    STT --> |Gated by VAD / Activation Mode| WHISPER[WhisperTranscriber]
    WHISPER --> |Transcription| LOGS[Structured Logs]

    %% Text injection pipeline
    LOGS --> |TranscriptionEvent| TEXTINJ[TextInjectionProcessor]
    TEXTINJ --> |Strategy Selection| STRATEGY[StrategyManager]
    STRATEGY --> |Platform Detection| BACKENDS{Text Injection Backends}
    BACKENDS --> |AT-SPI| ATSPI[AT-SPI Injector<br/>Linux Accessibility]
    BACKENDS --> |Clipboard| CLIP[Clipboard Injector<br/>Cross-platform]
    BACKENDS --> |ydotool| YDOT[ydotool Injector<br/>Wayland]
    BACKENDS --> |kdotool| KDOT[kdotool Injector<br/>X11]
    BACKENDS --> |Enigo| ENIGO[Enigo Injector<br/>Cross-platform]
    ATSPI --> APPS[Active Applications]
    CLIP --> APPS
    YDOT --> APPS
    KDOT --> APPS
    ENIGO --> APPS

    %% User Interface Components
    EVENTS --> |Subscribe| TUI[TUI Dashboard<br/>Real-time Display]
    LOGS --> TUI

    %% Foundation components
    subgraph Foundation [Foundation Layer]
        SM[StateManager]
        HM[HealthMonitor]
        SH[ShutdownHandler]
        METRICS[PipelineMetrics]
    end

    %% System monitoring
    AC -.-> METRICS
    VAD -.-> METRICS
    STT -.-> METRICS
    TEXTINJ -.-> METRICS
    HK -.-> METRICS

    %% Shutdown flow
    SH --> |Graceful Stop| AC
    SH --> |Abort Tasks| VAD
    SH --> |Abort Tasks| STT
    SH --> |Abort Tasks| TEXTINJ
    SH --> |Abort Tasks| CHUNKER
    SH --> |Abort Tasks| HK

    %% Component states
    classDef processing fill:#4a90e2,stroke:#333,stroke-width:2px,color:#fff
    classDef vad fill:#7ed321,stroke:#333,stroke-width:2px,color:#000
    classDef stt fill:#f5a623,stroke:#333,stroke-width:2px,color:#000
    classDef textinj fill:#e91e63,stroke:#333,stroke-width:2px,color:#fff
    classDef ui fill:#9013fe,stroke:#333,stroke-width:2px,color:#fff
    classDef foundation fill:#d0021b,stroke:#333,stroke-width:2px,color:#fff
    classDef disabled fill:#9b9b9b,stroke:#333,stroke-width:2px,color:#fff,stroke-dasharray: 5 5

    class AC,ARB,FR,CHUNKER,BROADCAST processing
    class VAD,VADENG,SILERO,VADFSM,EVENTS,HK vad
    class STT,WHISPER stt
    class TEXTINJ,STRATEGY,BACKENDS,ATSPI,CLIP,YDOT,KDOT,ENIGO,APPS textinj
    class TUI ui
    class SM,HM,SH,METRICS foundation
    class ENERGY disabled
```

## Key Architecture Changes Since Original Diagram

### 1. **Broadcast-Based Audio Distribution**
- Replaced linear pipeline with fan-out broadcast system
- Single `AudioChunker` feeds multiple subscribers via `broadcast::channel`
- Enables parallel processing of VAD and STT without blocking

### 2. **STT Integration (New)**
- `SttProcessor` subscribes to both audio frames and VAD events
- Whisper transcriber only processes audio when VAD indicates speech
- Produces structured logging output with partial and final transcriptions

### 3. **Global Hotkey System (New)**
- `spawn_hotkey_listener` captures system-wide hotkeys
- Sends `VadEvent` messages directly to the VAD event channel
- Enables manual control of voice activity detection state
- Integrated with KDE KGlobalAccel backend for Plasma environments

### 4. **TUI Dashboard (New)**
- Real-time monitoring interface (`tui_dashboard` binary)
- Subscribes to VAD events and transcription logs
- Displays live status, partial/final transcripts, and system metrics
- Separate from main application for dedicated monitoring

### 5. **Simplified VAD Architecture**
- `VadAdapter` provides unified interface to different VAD engines
- Silero ML-based VAD is now the default (Level3 energy VAD disabled)
- `VadStateMachine` handles debouncing and state transitions

### 6. **Enhanced Foundation Layer**
- `StateManager` tracks application lifecycle
- `HealthMonitor` provides system health checks
- `ShutdownHandler` ensures graceful cleanup of all components
- `PipelineMetrics` for cross-thread monitoring

### 7. **Async Task Management**
- All processing components run as independent Tokio tasks
- Proper task lifecycle management with spawn/abort pattern
- Channel-based communication between components

## Data Flow Summary

1. **Audio Capture**: Device → CPAL → AudioRingBuffer → FrameReader
2. **Hotkey Input**: System hotkeys → VadEvent channel (bypasses audio processing)
3. **Chunking**: Variable frames → Fixed 512-sample chunks
4. **Distribution**: Broadcast channel distributes to VAD + STT processors
5. **VAD**: Audio frames → Silero engine → State machine → Events
6. **STT**: Audio frames + VAD events → Whisper transcriber → Logs
7. **UI**: VAD events + transcription logs → TUI dashboard display
8. **Shutdown**: Graceful stop sequence with proper task cleanup

## Thread Architecture

- **Main Thread**: Orchestration, lifecycle management, shutdown handling
- **Audio Capture Thread**: Dedicated CPAL callback handling
- **Hotkey Listener Thread**: System-wide hotkey capture and event generation
- **Chunker Task**: Frame reading and chunking (async)
- **VAD Processor Task**: Voice activity detection (async)
- **STT Processor Task**: Speech-to-text transcription (async)
- **TUI Dashboard Task**: Real-time monitoring interface (optional, separate binary)
