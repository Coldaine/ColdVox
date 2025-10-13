# ColdVox STT Architecture Diagrams

These Mermaid diagrams illustrate the current and target STT architectures, including noise reduction flows. They are based on the migration plan in [STT_Migration_Plan.md](STT_Migration_Plan.md).

## Current STT Pipeline (Vosk Plugin-Based)

```mermaid
graph TD
    A[Audio Capture<br/>coldvox-audio: capture.rs] --> B[Resample & Chunk<br/>16kHz, 512-sample frames]
    B --> C[VAD Gating<br/>coldvox-vad: engine.rs<br/>RMS/Silero]
    C --> D{Speech Detected?}
    D -->|No| E[Silence: Discard Frame]
    D -->|Yes| F[Buffer Audio<br/>SttProcessor: SpeechActive State]
    F --> G[STT Plugin Registry<br/>coldvox-stt: SttPluginRegistry]
    G --> H[Vosk Plugin<br/>coldvox-stt-vosk: plugin.rs<br/>Load Model & Transcribe]
    H --> I[EventBasedTranscriber<br/>accept_frame() -> TranscriptionEvent]
    I --> J[Partial/Final Events<br/>text, words, timestamps]
    J --> K[Text Injection<br/>coldvox-text-injection]
    L[Config: Plugin Selection<br/>preferred: vosk] --> G
    style E fill:#ffcccc
    style K fill:#ccffcc
```

- **Key Components**: Plugin indirection (registry, factories), Vosk-specific model loading, VAD-buffered processing.
- **Issues**: Overhead from failover/config, Vosk CPU-only limitations.

## Target STT Pipeline (Direct Backends with Denoising)

```mermaid
graph TD
    A[Audio Capture<br/>coldvox-audio: capture.rs] --> B[Resample<br/>16kHz Mono PCM]
    B --> C[Denoise Stage<br/>coldvox-audio: denoiser.rs<br/>RNNoise/NeMo ONNX]
    C --> D[VAD Gating<br/>coldvox-vad: engine.rs<br/>Updated for Denoised Input]
    D --> E{Speech Detected?}
    E -->|No| F[Silence: Discard]
    E -->|Yes| G[Buffer Frames<br/>SttProcessor: SpeechActive]
    G --> H[Backend Selector<br/>SttBackend Enum: Whisper/Parakeet/Mock]
    H --> I{Backend Type}
    I -->|Whisper| J[WhisperContext<br/>whisper-rs: full_transcribe()<br/>Incremental Chunks]
    I -->|Parakeet| K[ONNX Session<br/>ort: run() on Exported NeMo Model<br/>GPU via CUDA Provider]
    I -->|Mock| L[Mock Transcription]
    J --> M[TranscriptionEvent<br/>Partial/Final with Timestamps]
    K --> M
    L --> M
    M --> N[Text Injection<br/>coldvox-text-injection]
    O[Config: backend, enable_denoise, denoiser_type] --> H
    O --> C
    P[GPU Detection<br/>nvml-wrapper] --> H
    style F fill:#ffcccc
    style N fill:#ccffcc
    style C fill:#ffffcc
```

- **Key Changes**: Direct enum selection, no plugins; denoising pre-VAD; GPU paths for Parakeet/NeMo.
- **Benefits**: Reduced latency, modular denoising, easy backend swap.

## Noise Reduction Flow

```mermaid
flowchart LR
    A[Raw Audio Frame<br/>e.g., 512 samples @ 16kHz] --> B{Denoiser Enabled?}
    B -->|No| C[VAD Input: Raw Frame]
    B -->|Yes| D{Denoiser Type}
    D -->|RNNoise| E[nnnoiseless::Denoise.process()<br/>CPU: RNN Frame Denoise<br/><1ms, 20-30dB SNR Gain]
    D -->|NeMo| F[ort::Session.run()<br/>ONNX RTC Denoiser<br/>GPU: <10ms, 15-25% WER Improve]
    D -->|Demucs| G[PyO3/ONNX Source Sep<br/>GPU: 50-100ms Segment<br/>Vocals Isolation, Offline Preferred]
    E --> H[Denoised Frame: i16 PCM]
    F --> H
    G --> H
    H --> I[VAD: Speech Detection<br/>Threshold on Cleaned Audio]
    I --> J{STT Buffer?}
    J -->|Yes| K[Feed to Backend<br/>Whisper/Parakeet]
    J -->|No| L[Discard]
    subgraph "Config Influences"
        M[enable_denoise: bool<br/>denoiser_type: enum<br/>strength: f32]
    end
    M --> B
    M --> D
    style L fill:#ffcccc
    style K fill:#ccffcc
```

- **Flow Details**: Per-frame denoising; type selected via config/GPU check. Outputs cleaned audio to VAD/STT.
- **Performance**: RNNoise for real-time CPU; NeMo for GPU synergy with Parakeet.

## Implementation Notes
- Diagrams use Mermaid syntax for VSCode preview.
- Current: Emphasizes plugin complexity.
- Target: Streamlined, with denoising branch.
- Validate: Render in Markdown viewer; adjust for clarity.

These diagrams can be referenced in code/docs. Next: User review for refinements.