//! Manual testing guide for TTS synthesis functionality

# TTS Synthesis Manual Testing Guide

This guide describes how to manually test the TTS synthesis functionality once the system dependencies are available.

## System Setup

### Install eSpeak

**Ubuntu/Debian:**
```bash
sudo apt update
sudo apt install espeak espeak-data
```

**Alternative eSpeak-NG (recommended):**
```bash
sudo apt install espeak-ng espeak-ng-data
```

**Verify Installation:**
```bash
espeak --version
espeak --voices | head -10
```

## Testing TTS Core Functionality

### 1. Basic TTS Synthesis Example

```bash
cd crates/app
cargo run --features tts-espeak,examples --example tts_synthesis_example
```

**Expected Output:**
- Lists available eSpeak voices
- Synthesizes sample text
- Saves WAV files to local directory
- Reports audio format details (sample rate, channels, size)

**Verification:**
- Check that `tts_output.wav` and `tts_output_custom.wav` files are created
- Play audio files: `aplay tts_output.wav` (Linux) or use your audio player
- Verify different speech parameters (rate, pitch, volume) in custom file

### 2. TTS Integration Example

```bash
cd crates/app  
cargo run --features tts-espeak,examples --example tts_integration_example
```

**Expected Behavior:**
- Creates TTS processor with eSpeak engine
- Simulates transcription events
- Synthesizes each final transcription
- Saves audio files to `/tmp/tts_output_*.wav`
- Processes error events appropriately

**Verification:**
- Check console output for transcription processing logs
- Verify audio files created in `/tmp/` directory
- Listen to generated speech for each test phrase

### 3. Unit Tests

```bash
# Test TTS core functionality
cargo test -p coldvox-tts

# Test eSpeak engine
cargo test -p coldvox-tts-espeak

# Test with actual eSpeak installed
cargo test -p coldvox-tts-espeak --features espeak
```

**Expected Results:**
- All core TTS tests pass
- eSpeak availability tests succeed when installed
- Engine lifecycle tests complete without panics

## TTS Features to Verify

### Voice Management
```bash
# List available voices
espeak --voices

# Test different voices in example
# Edit examples to use specific voices like:
# - en-us (American English)
# - en-gb (British English)  
# - fr (French)
# - de (German)
```

### Speech Parameters
- **Rate**: Test values 80-400 WPM (words per minute)
- **Pitch**: Test values 0.5-2.0 (1.0 = normal)
- **Volume**: Test values 0.0-1.0 (0.8 = default)

### Audio Output
- **Format**: Verify WAV files are playable
- **Quality**: 22050 Hz, 16-bit, mono (typical eSpeak output)
- **Size**: Reasonable file sizes for text length

## Integration Testing

### With STT Pipeline
1. Run STT transcription (if available)
2. Pipe transcription events to TTS processor
3. Verify speech synthesis of transcribed text
4. Test error handling and recovery

### With Text Injection
1. Enable text injection features
2. Configure TTS announcements
3. Test "injection successful" notifications
4. Verify accessibility announcements

### Performance Testing
1. **Synthesis Speed**: Should be 5-50x real-time
2. **Memory Usage**: Should remain stable during synthesis
3. **CPU Usage**: Should be low for eSpeak
4. **Latency**: Should be under 1 second for short phrases

## Troubleshooting

### Common Issues

**"eSpeak not available"**
```bash
# Check eSpeak installation
which espeak
espeak --version

# Install if missing
sudo apt install espeak-ng
```

**"No voices found"**
```bash
# Check voice data
espeak --voices
ls /usr/share/espeak-ng-data/

# Install voice data
sudo apt install espeak-ng-data
```

**"Synthesis failed"**
- Check input text is not empty
- Verify selected voice exists
- Check file permissions for output directory
- Monitor system logs for process errors

**"Audio files not playable"**
- Verify WAV format with: `file output.wav`
- Check audio system: `aplay -l`
- Test with different audio players

### Debug Logging
```bash
# Enable debug output
RUST_LOG=debug cargo run --features tts-espeak,examples --example tts_synthesis_example

# Trace level for detailed debugging  
RUST_LOG=trace cargo run --features tts-espeak,examples --example tts_integration_example
```

## Production Deployment

### System Requirements
- eSpeak or eSpeak-NG installed
- Audio output capability (for playback)
- File system write access (for temp files)
- Sufficient disk space for audio caching

### Configuration
```toml
[tts]
enabled = true
default_voice = "en-us"
speech_rate = 180
pitch = 1.0
volume = 0.8
announce_errors = false
save_audio_files = false
```

### Monitoring
- Monitor TTS synthesis success/failure rates
- Track audio generation performance
- Watch for voice availability issues
- Monitor disk usage for audio files

## Next Steps

After successful manual testing:

1. **Audio Playback**: Integrate with CPAL or system audio
2. **Voice Configuration**: Add voice selection UI
3. **SSML Support**: Enhanced markup for speech control  
4. **Neural TTS**: Add Piper or cloud-based TTS engines
5. **Streaming**: Implement real-time synthesis for long texts

## Verification Checklist

- [ ] eSpeak installed and accessible
- [ ] TTS core tests pass
- [ ] eSpeak engine tests pass
- [ ] Basic synthesis example works
- [ ] Integration example works
- [ ] Audio files generated and playable
- [ ] Different voices work
- [ ] Speech parameters adjustable
- [ ] Error handling functions
- [ ] Performance acceptable
- [ ] Memory usage stable
- [ ] No resource leaks