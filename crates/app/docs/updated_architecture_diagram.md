# ColdVox Architecture Diagram - Updated 2025-08-26

```mermaid
graph TD
    %% External inputs
    MIC[Audio Input Device] --> AC[AudioCapture]
    
    %% Audio capture and preprocessing
    AC --> |CPAL callback| ARB[AudioRingBuffer]
    ARB --> |Consumer| FR[FrameReader]
    FR --> |Variable frames| CHUNKER[AudioChunker]
    
    %% Broadcast distribution system
    CHUNKER --> |512 sample chunks| BROADCAST{Broadcast Channel<br/>AudioFrame}
    
    %% VAD processing branch
    BROADCAST --> |Subscribe| VAD[VadProcessor]
    VAD --> |VadAdapter| VADENG{VAD Engine}
    VADENG --> |SileroEngine Default| SILERO[SileroEngine<br/>ML-based VAD]
    VADENG --> |Level3Vad Disabled| ENERGY[Level3Vad<br/>Energy-based VAD]
    
    %% VAD state management
    VAD --> |VAD Events| VADFSM[VadStateMachine<br/>Debouncing]
    VADFSM --> |SpeechStart/End| EVENTS[VAD Event Channel]
    
    %% STT processing branch
    BROADCAST --> |Subscribe| STT[SttProcessor]
    EVENTS --> STT
    STT --> |Gated by VAD| VOSK[VoskTranscriber]
    VOSK --> |Transcription| LOGS[Structured Logs]
    
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
    
    %% Shutdown flow
    SH --> |Graceful Stop| AC
    SH --> |Abort Tasks| VAD
    SH --> |Abort Tasks| STT
    SH --> |Abort Tasks| CHUNKER
    
    %% Component states
    classDef processing fill:#4a90e2,stroke:#333,stroke-width:2px,color:#fff
    classDef vad fill:#7ed321,stroke:#333,stroke-width:2px,color:#000
    classDef stt fill:#f5a623,stroke:#333,stroke-width:2px,color:#000
    classDef foundation fill:#d0021b,stroke:#333,stroke-width:2px,color:#fff
    classDef disabled fill:#9b9b9b,stroke:#333,stroke-width:2px,color:#fff,stroke-dasharray: 5 5
    
    class AC,ARB,FR,CHUNKER,BROADCAST processing
    class VAD,VADENG,SILERO,VADFSM,EVENTS vad
    class STT,VOSK stt
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
- Vosk transcriber only processes audio when VAD indicates speech
- Produces structured logging output with partial and final transcriptions

### 3. **Simplified VAD Architecture**
- `VadAdapter` provides unified interface to different VAD engines
- Silero ML-based VAD is now the default (Level3 energy VAD disabled)
- `VadStateMachine` handles debouncing and state transitions

### 4. **Enhanced Foundation Layer**
- `StateManager` tracks application lifecycle
- `HealthMonitor` provides system health checks
- `ShutdownHandler` ensures graceful cleanup of all components
- `PipelineMetrics` for cross-thread monitoring

### 5. **Async Task Management**
- All processing components run as independent Tokio tasks
- Proper task lifecycle management with spawn/abort pattern
- Channel-based communication between components

## Data Flow Summary

1. **Audio Capture**: Device → CPAL → AudioRingBuffer → FrameReader
2. **Chunking**: Variable frames → Fixed 512-sample chunks
3. **Distribution**: Broadcast channel distributes to VAD + STT processors
4. **VAD**: Audio frames → Silero engine → State machine → Events
5. **STT**: Audio frames + VAD events → Vosk transcriber → Logs
6. **Shutdown**: Graceful stop sequence with proper task cleanup

## Thread Architecture

- **Main Thread**: Orchestration, lifecycle management, shutdown handling
- **Audio Capture Thread**: Dedicated CPAL callback handling
- **Chunker Task**: Frame reading and chunking (async)
- **VAD Processor Task**: Voice activity detection (async)
- **STT Processor Task**: Speech-to-text transcription (async)