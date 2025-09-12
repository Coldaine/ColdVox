---
id: CODE:repo://crates/coldvox-stt-vosk/src/vosk_transcriber.rs
type: IMP
level: 6
title: Vosk STT Implementation
status: implemented
area: STT
module: Vosk
owners: [@team-stt]
updated: 2025-09-11
links:
  implements: [COLDVOX-SPEC5-004-stt-engine-interface]
  depends_on: []
  verified_by: [COLDVOX-TST6-005-stt-processor-tests]
  related_to: [COLDVOX-ADR3-001-vosk-model-distribution]
---

## Summary
Implementation of Vosk speech-to-text engine via FFI.

## Description
This implementation provides speech-to-text transcription using the Vosk library via FFI, supporting offline operation with pre-trained models.

## Key Components
- Vosk library integration via FFI
- Model loading and management
- Transcription processing
- Event generation (Partial, Final, Error)

## Code Structure
```rust
// Vosk transcriber implementation
pub struct VoskTranscriber {
    recognizer: Recognizer,
    config: SttConfig,
}

impl Transcriber for VoskTranscriber {
    fn new(config: &SttConfig) -> Result<Self, SttError> {
        let model = Model::new(&config.model_path)?;
        let recognizer = Recognizer::new(&model, config.sample_rate as f32)?;
        
        Ok(Self {
            recognizer,
            config: config.clone(),
        })
    }
    
    fn transcribe(&mut self, audio: &[i16]) -> Result<TranscriptionEvent, SttError> {
        let result = self.recognizer.accept_waveform(audio);
        
        if result {
            let result_str = self.recognizer.result();
            Ok(TranscriptionEvent::Final {
                text: parse_vosk_result(result_str)?,
                confidence: 0.9,
            })
        } else {
            let partial_str = self.recognizer.partial_result();
            Ok(TranscriptionEvent::Partial {
                text: parse_vosk_result(partial_str)?,
                confidence: 0.7,
            })
        }
    }
}
```

## Dependencies
- vosk = "0.1"
- serde = { version = "1.0", features = ["derive"] }
- serde_json = "1.0"

---
implements: COLDVOX-SPEC5-004-stt-engine-interface  
depends_on:  
verified_by: COLDVOX-TST6-005-stt-processor-tests  
related_to: COLDVOX-ADR3-001-vosk-model-distribution