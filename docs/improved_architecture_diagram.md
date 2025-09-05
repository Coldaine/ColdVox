# ColdVox Runtime Architecture Diagram - Improved

```mermaid
flowchart TD
    %% External inputs
    MIC[Audio Input Device] --> |Audio Stream| AC[AudioCapture<br>crates/app/src/audio/]
    HK[Global Hotkeys] --> |VadEvent| EVENTS[VAD Event Channel]
    USER[User Input] --> GUI[coldvox-gui]

    %% Audio capture and preprocessing
    AC --> |CPAL callback| ARB[AudioRingBuffer<br>crates/app/src/audio/]
    ARB --> |Consumer| FR[FrameReader<br>crates/app/src/audio/]
    FR --> |Variable frames| CHUNKER[AudioChunker<br>crates/app/src/audio/]

    %% Broadcast distribution system
    CHUNKER --> |512 sample chunks| BROADCAST{Broadcast Channel<br>AudioFrame}

    %% VAD processing branch
    BROADCAST --> |Subscribe| VAD[VadProcessor<br>crates/coldvox-vad/]
    VAD --> |VadAdapter| VADENG{VAD Engine}
    VADENG --> |SileroEngine (default)| SILERO[SileroEngine<br>crates/coldvox-vad-silero/]
    VADENG --> |Alternative VAD| VADALT[VoiceActivityDetector<br>crates/voice-activity-detector/]

    %% VAD state management
    VAD --> |VAD Events| VADFSM[VadStateMachine<br>Debouncing]
    VADFSM --> |SpeechStart/End| EVENTS

    %% STT processing branch
    BROADCAST --> |Subscribe| STT[SttProcessor<br>crates/coldvox-stt/]
    EVENTS --> STT
    STT --> |Gated by VAD / Activation Mode| VOSK[VoskTranscriber<br>crates/coldvox-stt-vosk/]
    VOSK --> |Transcription| LOGS[Structured Logs<br>crates/coldvox-telemetry/]

    %% Text injection pipeline
    LOGS --> |TranscriptionEvent| TEXTINJ[TextInjectionProcessor<br>crates/coldvox-text-injection/]
    TEXTINJ --> |Strategy Selection| STRATEGY[StrategyManager]
    STRATEGY --> |Platform Detection| BACKENDS{Text Injection Backends}
    BACKENDS --> |AT-SPI| ATSPI[AT-SPI Injector<br>Linux Accessibility]
    BACKENDS --> |Clipboard| CLIP[Clipboard Injector<br>Cross-platform]
    BACKENDS --> |ydotool| YDOT[ydotool Injector<br>Wayland]
    BACKENDS --> |kdotool| KDOT[kdotool Injector<br>X11]
    BACKENDS --> |Enigo| ENIGO[Enigo Injector<br>Cross-platform]
    ATSPI --> APPS[Active Applications]
    CLIP --> APPS
    YDOT --> APPS
    KDOT --> APPS
    ENIGO --> APPS

    %% User Interface Components
    EVENTS --> |Subscribe| TUI[TUI Dashboard<br>crates/app/src/bin/tui_dashboard.rs]
    LOGS --> TUI
    EVENTS --> |Status Updates| GUI
    LOGS --> |Transcription Updates| GUI
    GUI --> |User Commands| EVENTS

    %% Foundation components
    subgraph Foundation [Foundation Layer]
        SM[StateManager<br>crates/coldvox-foundation/]
        HM[HealthMonitor<br>crates/coldvox-foundation/]
        SH[ShutdownHandler<br>crates/coldvox-foundation/]
        METRICS[PipelineMetrics<br>crates/coldvox-telemetry/]
    end

    %% System monitoring
    AC -.-> |Metrics| METRICS
    VAD -.-> |Metrics| METRICS
    STT -.-> |Metrics| METRICS
    TEXTINJ -.-> |Metrics| METRICS
    HK -.-> |Metrics| METRICS
    GUI -.-> |Metrics| METRICS

    %% Shutdown flow
    SH --> |Graceful Stop| AC
    SH --> |Abort Tasks| VAD
    SH --> |Abort Tasks| STT
    SH --> |Abort Tasks| TEXTINJ
    SH --> |Abort Tasks| CHUNKER
    SH --> |Abort Tasks| HK
    SH --> |Abort Tasks| GUI

    %% CLI/Binaries
    subgraph CLI [CLI/Binaries]
        MAIN[main.rs<br>crates/app/src/main.rs]
        MIC_PROBE[mic_probe.rs<br>crates/app/src/bin/mic_probe.rs]
        TUI_DASH[tui_dashboard.rs<br>crates/app/src/bin/tui_dashboard.rs]
    end

    MAIN --> AC
    MAIN --> VAD
    MAIN --> STT
    MAIN --> TEXTINJ
    MAIN --> HM
    MAIN --> SH
    MAIN --> GUI

    %% Examples
    subgraph Examples [Examples]
        FOUNDATION_PROBE[foundation_probe.rs<br>examples/foundation_probe.rs]
        INJECT_DEMO[inject_demo.rs<br>examples/inject_demo.rs]
        RECORD_10S[record_10s.rs<br>examples/record_10s.rs]
        HOTKEY_BACKEND[test_hotkey_backend.rs<br>examples/test_hotkey_backend.rs]
        KGLOBALACCEL[test_kglobalaccel_hotkey.rs<br>examples/test_kglobalaccel_hotkey.rs]
        SILERO_MINIMAL[test_silero_minimal.rs<br>examples/test_silero_minimal.rs]
        SILERO_WAV[test_silero_wav.rs<br>examples/test_silero_wav.rs]
        TEXT_INJECTION_PROBE[text_injection_probe.rs<br>examples/text_injection_probe.rs]
        VOSK_TEST[vosk_test.rs<br>examples/vosk_test.rs]
    end

    FOUNDATION_PROBE --> SM
    INJECT_DEMO --> TEXTINJ
    RECORD_10S --> AC
    HOTKEY_BACKEND --> HK
    KGLOBALACCEL --> HK
    SILERO_MINIMAL --> SILERO
    SILERO_WAV --> SILERO
    TEXT_INJECTION_PROBE --> STRATEGY
    VOSK_TEST --> VOSK

    %% Scripts
    subgraph Scripts [Scripts]
        DETECT_GPU[detect-target-gpu.sh<br>scripts/detect-target-gpu.sh]
        GPU_HOOK[gpu-conditional-hook.sh<br>scripts/gpu-conditional-hook.sh]
        SETUP_TEXT_INJECTION[setup_text_injection.sh<br>scripts/setup_text_injection.sh]
        TEST_FEATURES_PY[test-features.py<br>test-features.py]
        TEST_TEXT_INJECTION_MOCK[test_text_injection_mock.sh<br>test_text_injection_mock.sh]
    end

    DETECT_GPU --> VADENG
    GPU_HOOK --> VADENG
    SETUP_TEXT_INJECTION --> BACKENDS
    TEST_FEATURES_PY --> METRICS
    TEST_TEXT_INJECTION_MOCK --> TEXTINJ

    %% Component styling
    classDef processing fill:#4a90e2,stroke:#333,stroke-width:2px,color:#fff
    classDef vad fill:#7ed321,stroke:#333,stroke-width:2px,color:#000
    classDef stt fill:#f5a623,stroke:#333,stroke-width:2px,color:#000
    classDef textinj fill:#e91e63,stroke:#333,stroke-width:2px,color:#fff
    classDef ui fill:#9013fe,stroke:#333,stroke-width:2px,color:#fff
    classDef foundation fill:#d0021b,stroke:#333,stroke-width:2px,color:#fff
    classDef cli fill:#50e3c2,stroke:#333,stroke-width:2px,color:#000
    classDef examples fill:#f8e71c,stroke:#333,stroke-width:2px,color:#000
    classDef scripts fill:#bd10e0,stroke:#333,stroke-width:2px,color:#fff

    class AC,ARB,FR,CHUNKER,BROADCAST processing
    class VAD,VADENG,SILERO,VADFSM,EVENTS,HK,VADALT vad
    class STT,VOSK stt
    class TEXTINJ,STRATEGY,BACKENDS,ATSPI,CLIP,YDOT,KDOT,ENIGO,APPS textinj
    class TUI,GUI ui
    class SM,HM,SH,METRICS foundation
    class MAIN,MIC_PROBE,TUI_DASH cli
    class FOUNDATION_PROBE,INJECT_DEMO,RECORD_10S,HOTKEY_BACKEND,KGLOBALACCEL,SILERO_MINIMAL,SILERO_WAV,TEXT_INJECTION_PROBE,VOSK_TEST examples
    class DETECT_GPU,GPU_HOOK,SETUP_TEXT_INJECTION,TEST_FEATURES_PY,TEST_TEXT_INJECTION_MOCK scripts
```

## Key Improvements in This Diagram

### 1. **Complete Component Coverage**
- Added missing `coldvox-gui` component with bidirectional communication
- Added `voice-activity-detector` as an alternative VAD engine
- Included all CLI binaries with proper file paths
- Added all examples with correct file paths from the `examples/` directory
- Included all scripts with their actual locations

### 2. **Enhanced Runtime Data Flow**
- Clear audio flow from input device through processing pipeline
- Detailed VAD processing with both primary and alternative engines
- Comprehensive STT pipeline with VAD gating
- Complete text injection flow with multiple backend strategies
- Bidirectional communication between GUI and core components

### 3. **Improved Visual Organization**
- Logical grouping of components by functionality
- Clear hierarchical structure with subgraphs
- Consistent styling with color coding for different component types
- Better spacing and layout for improved readability

### 4. **Detailed Dependencies**
- Foundation layer shows clear relationships with all components
- Metrics collection from all major components
- Proper shutdown flow affecting all active components
- Examples and scripts connected to their relevant components

### 5. **Accurate File Paths**
- All file paths match the actual repository structure
- Correct crate locations and module structures
- Proper binary locations in `crates/app/src/bin/`
- Accurate example file locations in `examples/`

## Component Legend

- **Processing (Blue)**: Audio capture and preprocessing components
- **VAD (Green)**: Voice activity detection components
- **STT (Orange)**: Speech-to-text processing components
- **Text Injection (Pink)**: Text output and injection components
- **UI (Purple)**: User interface components
- **Foundation (Red)**: Core system services and utilities
- **CLI (Teal)**: Command-line interface binaries
- **Examples (Yellow)**: Example applications and test programs
- **Scripts (Purple)**: Build and test scripts

## Data Flow Summary

1. **Audio Input**: Device → AudioCapture → AudioRingBuffer → FrameReader → AudioChunker
2. **Broadcast Distribution**: AudioChunker → Broadcast Channel (fan-out to subscribers)
3. **VAD Processing**: Broadcast Channel → VadProcessor → VAD Engine → StateMachine → Events
4. **STT Processing**: Broadcast Channel + VAD Events → SttProcessor → VoskTranscriber → Logs
5. **Text Injection**: Logs → TextInjectionProcessor → StrategyManager → Backends → Applications
6. **User Interfaces**: Events + Logs → TUI Dashboard + GUI
7. **System Management**: Foundation components monitor and manage all processing components
8. **Examples & Scripts**: Various test and utility programs interact with specific components

This diagram provides a comprehensive view of the ColdVox runtime architecture, showing all components, their relationships, and the data flow between them.
