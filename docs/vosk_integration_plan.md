# Vosk Integration Plan

## Model Setup

### Download Location

```bash
# Models should be stored in:
/home/coldaine/Projects/ColdVox/models/

# Download the SMALL Vosk model (40MB - recommended):
cd /home/coldaine/Projects/ColdVox
mkdir -p models
cd models
wget https://alphacephei.com/vosk/models/vosk-model-small-en-us-0.15.zip
unzip vosk-model-small-en-us-0.15.zip
rm vosk-model-small-en-us-0.15.zip

# The extracted folder will be: vosk-model-small-en-us-0.15
```

## System Dependencies

### Install libvosk

```bash
# Fedora/Nobara (your system):
sudo dnf install vosk

# Or build from source:
git clone https://github.com/alphacep/vosk-api
cd vosk-api/src
make
sudo make install
```

## Implementation Steps

### 1. Fix Cargo.toml

```toml
# crates/app/Cargo.toml
[dependencies]
vosk = "0.3.1"  # Update to latest
```

### 2. Update VoskTranscriber

```rust
// crates/app/src/stt/vosk.rs
use vosk::{Model, Recognizer};

pub struct VoskTranscriber {
    model: Model,
    recognizer: Recognizer,
}

impl VoskTranscriber {
    pub fn new(model_path: &str) -> Result<Self, AppError> {
        let model = Model::new(model_path)?;
        let recognizer = Recognizer::new(&model, 16000.0)?;
        Ok(Self { model, recognizer })
    }
    
    pub fn accept_waveform(&mut self, data: &[i16]) -> Result<(), AppError> {
        // Convert i16 to bytes for Vosk
        let bytes: Vec<u8> = data.iter()
            .flat_map(|&sample| sample.to_le_bytes())
            .collect();
        
        self.recognizer.accept_waveform(&bytes);
        Ok(())
    }
    
    pub fn get_partial_result(&self) -> String {
        self.recognizer.partial_result()
    }
    
    pub fn get_final_result(&mut self) -> String {
        self.recognizer.final_result()
    }
}
```

### 3. Wire Into Main Application

```rust
// crates/app/src/main.rs
// Add to your existing main function:

const VOSK_MODEL_PATH: &str = "/home/coldaine/Projects/ColdVox/models/vosk-model-small-en-us-0.15";

// In your pipeline setup:
let vosk = VoskTranscriber::new(VOSK_MODEL_PATH)?;
let stt_processor = TranscriptionProcessor::new(vosk, stt_rx, events_tx);
```

### 4. Connect to Audio Pipeline

```rust
// crates/app/src/stt/processor.rs
// Your existing TranscriptionProcessor should:

impl TranscriptionProcessor {
    pub async fn run(&mut self) {
        while let Ok(frame) = self.stt_rx.recv().await {
            // Feed audio to Vosk
            self.vosk.accept_waveform(&frame.data)?;
            
            // Get partial result
            let partial = self.vosk.get_partial_result();
            if !partial.is_empty() {
                self.events_tx.send(TranscriptionEvent::Partial { 
                    text: partial 
                }).await?;
            }
            
            // On VAD end event, get final result
            if frame.is_speech_end {
                let final_result = self.vosk.get_final_result();
                self.events_tx.send(TranscriptionEvent::Final { 
                    text: final_result 
                }).await?;
            }
        }
    }
}
```

## Testing

### 1. Check Dependencies

```bash
# Verify libvosk is installed
pkg-config --libs vosk

# Test compilation
cd crates/app
cargo build --features vosk
```

### 2. Test With Live Audio

```bash
# Run the main application
cargo run --features vosk

# You should see transcription output when you speak
```

### 3. Test With WAV File

```rust
// crates/app/examples/test_vosk.rs
use std::fs::File;
use hound::WavReader;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model_path = "/home/coldaine/Projects/ColdVox/models/vosk-model-small-en-us-0.15";
    let mut vosk = VoskTranscriber::new(model_path)?;
    
    // Read test WAV file (16kHz mono)
    let reader = WavReader::open("test_audio.wav")?;
    let samples: Vec<i16> = reader.into_samples().collect::<Result<Vec<_>, _>>()?;
    
    // Process in chunks
    for chunk in samples.chunks(512) {
        vosk.accept_waveform(chunk)?;
        println!("Partial: {}", vosk.get_partial_result());
    }
    
    println!("Final: {}", vosk.get_final_result());
    Ok(())
}
```

## Configuration

```rust
// Add to your existing config
pub struct TranscriptionConfig {
    pub enabled: bool,
    pub model_path: String,
    pub partial_results: bool,
}

impl Default for TranscriptionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            model_path: "/home/coldaine/Projects/ColdVox/models/vosk-model-small-en-us-0.15".into(),
            partial_results: true,
        }
    }
}
```

## Common Issues and Fixes

### libvosk not found

```bash
# Add to your .bashrc or run before cargo build:
export LD_LIBRARY_PATH=/usr/local/lib:$LD_LIBRARY_PATH
```

### Model loading fails

```bash
# Check model exists and has correct permissions:
ls -la /home/coldaine/Projects/ColdVox/models/
chmod -R 755 models/
```

### High memory usage

The small model uses ~40MB RAM. If you need better accuracy, you can use larger models:

```bash
# Medium model (1.8GB):
wget https://alphacephei.com/vosk/models/vosk-model-en-us-0.22.zip

# Large model (2.3GB):
wget https://alphacephei.com/vosk/models/vosk-model-en-us-0.42-gigaspeech.zip
```

## Next Steps

1. Install libvosk
2. Download the model
3. Update the code as shown above
4. Test with `cargo run --features vosk`
5. Adjust buffer sizes if needed for real-time performance

That's it. This gets Vosk working with your existing pipeline.