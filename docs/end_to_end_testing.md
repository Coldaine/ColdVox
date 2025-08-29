# End-to-End WAV File Testing

This document describes how to run the comprehensive end-to-end test that processes WAV files through the entire ColdVox pipeline from audio input to text injection.

## Overview

The end-to-end test (`test_end_to_end_wav_pipeline`) simulates the complete ColdVox pipeline:

1. **WAV File Loading**: Loads and streams WAV files as if they were live microphone input
2. **Audio Processing**: Chunking, resampling, and mono conversion
3. **VAD Processing**: Speech activity detection using Silero VAD
4. **STT Processing**: Speech-to-text transcription using Vosk
5. **Text Injection**: Mock text injection that captures results for verification

## Prerequisites

### 1. Vosk Model

Download a Vosk model for speech recognition:

```bash
# Create models directory
mkdir -p models

# Download a small English model (37MB)
wget -O models/vosk-model-small-en-us-0.15.zip \
    https://alphacephei.com/vosk/models/vosk-model-small-en-us-0.15.zip

# Extract the model
cd models
unzip vosk-model-small-en-us-0.15.zip
cd ..

# Set environment variable
export VOSK_MODEL_PATH=models/vosk-model-small-en-us-0.15
```

Alternatively, use a larger, more accurate model:

```bash
# Download medium English model (328MB) - better accuracy
wget -O models/vosk-model-en-us-0.22-lgraph.zip \
    https://alphacephei.com/vosk/models/vosk-model-en-us-0.22-lgraph.zip

cd models
unzip vosk-model-en-us-0.22-lgraph.zip
export VOSK_MODEL_PATH=models/vosk-model-en-us-0.22-lgraph
cd ..
```

### 2. Test Audio Files

The test suite includes pre-recorded WAV files with corresponding transcripts in the `test_data/` directory.

#### Automatic Test Data Selection

The test automatically:
1. **Randomly selects** a WAV file from `test_data/` directory
2. **Loads the corresponding transcript** from the `.txt` file with the same name
3. **Extracts keywords** from the transcript (words ≥4 characters)
4. **Verifies transcription** by checking if at least one keyword appears in the output

#### Test Data Files

The repository includes 13 test WAV files (`test_1.wav` through `test_12.wav` and `pipeline_test.wav`) with transcripts. Each transcript contains the expected text in uppercase, for example:
- `test_1.txt`: "ON AUGUST TWENTY SEVENTH EIGHTEEN THIRTY SEVEN SHE WRITES"
- `test_5.txt`: "YOUR PLAY MUST BE NOT MERELY A GOOD PLAY BUT A SUCCESSFUL ONE"

#### Option A: Use Existing Test Data

```bash
# Run test with random file selection from test_data/
VOSK_MODEL_PATH=models/vosk-model-small-en-us-0.15 \
    cargo test --features vosk test_end_to_end_wav_pipeline -- --ignored
```

#### Option B: Record Custom Test Audio

```bash
# Record a 10-second test file (speak clearly)
cargo run --example record_10s

# Use your custom recording
TEST_WAV=recording_16khz_10s_1672531200.wav \
VOSK_MODEL_PATH=models/vosk-model-small-en-us-0.15 \
    cargo test --features vosk test_end_to_end_wav_pipeline -- --ignored
```

#### Option C: Use Existing Audio

Convert existing audio files to the required format:

```bash
# Using ffmpeg to convert any audio file
ffmpeg -i input_audio.mp3 -ar 16000 -ac 1 -sample_fmt s16 test_audio_16k.wav

# Or using SoX
sox input_audio.wav -r 16000 -c 1 -b 16 test_audio_16k.wav
```

## Running the Test

### Basic Test Execution

```bash
# Run with automatic random test file selection
VOSK_MODEL_PATH=models/vosk-model-small-en-us-0.15 \
    cargo test --features vosk test_end_to_end_wav_pipeline -- --ignored --nocapture

# Run with a specific WAV file
TEST_WAV=test_audio_16k.wav VOSK_MODEL_PATH=models/vosk-model-small-en-us-0.15 \
    cargo test --features vosk test_end_to_end_wav_pipeline -- --ignored --nocapture
```

### With Different Models

```bash
# Test with the larger, more accurate model
TEST_WAV=recording_16khz_10s_1672531200.wav \
VOSK_MODEL_PATH=models/vosk-model-en-us-0.22-lgraph \
    cargo test --features vosk test_end_to_end_wav_pipeline -- --ignored --nocapture
```

### Environment Variables

- `TEST_WAV`: Path to the WAV file to test (default: `test_audio_16k.wav`)
- `VOSK_MODEL_PATH`: Path to the Vosk model directory
- `RUST_LOG`: Set to `debug` or `trace` for detailed logging

## Test Validation

The test performs several validations:

1. **Audio Loading**: Verifies WAV file loads and converts correctly
2. **Pipeline Setup**: Ensures all components initialize properly
3. **Speech Detection**: VAD should detect speech segments in the audio
4. **Transcription**: STT should produce text output from detected speech
5. **Text Injection**: Mock injector should capture the transcribed text
6. **Content Verification**: Checks that at least one expected keyword is present

### Validation Strategy

The test uses a **flexible keyword matching** approach:
- Extracts keywords (≥4 characters) from the reference transcript
- Checks if at least one keyword appears in the transcription
- Accounts for STT accuracy limitations (not expecting 100% accuracy)
- Handles variations in pronunciation and recognition errors

### Example Test Output

```
Testing with WAV file: test_data/test_4.wav
Expected keywords: ["american", "school", "boys"]
✅ Test passed! Injections: ["can schoolboys read with emotions of horror", ...]
```

In this example, the keyword "schoolboys" (containing "school") was successfully matched.

## Creating Good Test Audio

For reliable test results:

1. **Clear Speech**: Speak clearly and at normal volume
2. **Quiet Environment**: Minimize background noise
3. **Simple Phrases**: Use common words that Vosk recognizes well
4. **Appropriate Length**: 5-15 seconds is ideal
5. **Proper Format**: 16kHz, mono, 16-bit PCM WAV

### Example Test Phrases

Record yourself saying:
- "Hello world, this is a test"
- "The quick brown fox jumps over the lazy dog"
- "ColdVox is working correctly"
- "Testing speech recognition pipeline"

## Troubleshooting

### Common Issues

1. **Model Not Found**
   ```
   Error: Vosk model not found at 'models/vosk-model-small-en-us-0.15'
   ```
   Solution: Download and extract the Vosk model as described above.

2. **No Speech Detected**
   ```
   No speech detected. Possible issues:
   - Threshold too high (current: 0.2)
   - Audio file contains no speech
   ```
   Solutions:
   - Check audio file has audible speech
   - Lower VAD threshold in test code
   - Verify audio format is correct

3. **Poor Transcription Quality**
   - Use a larger, more accurate Vosk model
   - Ensure clear speech in test audio
   - Check for background noise

4. **Test Timeout**
   - Increase test duration for longer audio files
   - Check for component initialization issues

### Debug Mode

Run with detailed logging:

```bash
RUST_LOG=debug TEST_WAV=test_audio_16k.wav \
VOSK_MODEL_PATH=models/vosk-model-small-en-us-0.15 \
    cargo test --features vosk test_end_to_end_wav_pipeline -- --ignored --nocapture
```

## Integration with CI/CD

The test can be integrated into automated testing with proper setup:

```yaml
# Example GitHub Actions step
- name: Download Vosk Model
  run: |
    mkdir -p models
    wget -O models/model.zip https://alphacephei.com/vosk/models/vosk-model-small-en-us-0.15.zip
    cd models && unzip model.zip

- name: Run End-to-End Test
  env:
    VOSK_MODEL_PATH: models/vosk-model-small-en-us-0.15
    TEST_WAV: test_data/sample_speech.wav
  run: cargo test --features vosk test_end_to_end_wav_pipeline -- --ignored
```

## Test Architecture

The test creates a complete pipeline simulation:

```
WAV File → AudioRingBuffer → AudioChunker → VadProcessor
                                                ↓
Mock Text Injector ← STT Processor ← Audio Frames
```

### Key Components

1. **WavFileLoader**: Streams WAV file data with realistic timing
   - Loads WAV files using the `hound` crate
   - Simulates real-time audio streaming (32ms chunks)
   - Handles format conversion to 16kHz mono i16

2. **Mock Text Injector**: Captures transcriptions for verification
   - Implements the `TextInjector` trait
   - Stores injected text in a thread-safe collection
   - Enables validation without actual system text injection

3. **Mock Injection Processor**: Manages transcription sessions
   - Buffers partial transcriptions
   - Implements silence timeout logic (1.5 seconds)
   - Simulates production text injection behavior

4. **Random Test Selection**: Ensures comprehensive coverage
   - Randomly selects from available test files
   - Loads corresponding transcripts automatically
   - Extracts meaningful keywords for validation

This ensures the test validates the actual production code paths and component interactions, providing confidence in the full system integration.