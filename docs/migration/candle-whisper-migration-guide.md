# Candle Whisper Migration Guide

## Overview

This guide provides step-by-step instructions for migrating from Python-based STT backends (Faster-Whisper) to the new pure Rust Candle Whisper implementation in ColdVox.

## Migration Benefits

### Before Migration (Python Faster-Whisper)
- Requires Python 3.8+ installation
- Needs `faster-whisper` package installation
- Higher memory overhead from Python runtime
- Slower startup due to Python interpreter
- Complex dependency management
- Platform-specific Python setup challenges

### After Migration (Candle Whisper)
- **Zero Python dependencies** - Pure Rust implementation
- **Faster startup** - Compiled binary execution
- **Lower memory usage** - Optimized Rust runtime
- **Simplified deployment** - Single executable
- **Better performance** - Native compilation
- **Cross-platform consistency** - Same behavior everywhere

## Migration Steps

### Step 1: System Requirements Check

#### Minimum Requirements
- **RAM**: 4GB (8GB recommended)
- **Storage**: 2GB free space for models
- **CPU**: 2+ cores (4+ recommended)
- **OS**: Linux, Windows, or macOS

#### Optional GPU Requirements
- **NVIDIA GPU**: For CUDA acceleration
- **CUDA Version**: 11.0+ recommended
- **VRAM**: 4GB+ for optimal performance

### Step 2: Remove Python Dependencies (If Migrating from Python Backend)

#### Linux
```bash
# Remove Python (if not needed for other applications)
sudo apt remove python3 python3-pip
sudo apt autoremove

# Or keep Python but remove faster-whisper specifically
pip3 uninstall faster-whisper
```

#### macOS
```bash
# Using Homebrew
brew uninstall python@3.x  # If not needed for other apps

# Or remove just the faster-whisper package
pip3 uninstall faster-whisper
```

#### Windows
```bash
# Remove Python from Programs & Features (if not needed)
# Or uninstall faster-whisper via pip
pip uninstall faster-whisper
```

### Step 3: Build ColdVox with Candle Whisper

#### Development Build
```bash
cd /path/to/coldvox
cargo build --features "text-injection,candle-whisper"
```

#### Production Build
```bash
cargo build --release --features "text-injection,candle-whisper"
```

#### Features Explanation
- `text-injection`: Enable text injection functionality
- `candle-whisper`: Enable the pure Rust Whisper backend
- `release`: Optimized build for production

### Step 4: Configuration Update

#### Update plugins.json
Edit `config/plugins.json`:

```json
{
  "stt": {
    "backend": "candle-whisper",
    "model_path": "openai/whisper-base.en",
    "device": "auto",
    "language": "en",
    "include_words": true,
    "streaming": true
  }
}
```

#### Environment Variables (Optional)
```bash
# Device selection
export CANDLE_WHISPER_DEVICE="auto"     # auto, cpu, cuda

# Model path override
export WHISPER_MODEL_PATH="/path/to/custom/model"

# Language hint
export WHISPER_LANGUAGE="en"

# Model cache directory
export HF_HOME="/custom/cache/directory"
```

### Step 5: Model Download

#### Automatic Download (First Run)
The first run will automatically download the specified model:
```bash
./target/release/coldvox --stt-backend candle-whisper
```

#### Manual Download
```bash
# Create models directory
mkdir -p ~/.cache/huggingface/hub

# The model will be downloaded automatically on first use
# Model ID: "openai/whisper-base.en"
# Size: ~142MB
```

#### Available Models
- `openai/whisper-tiny` (~39MB) - Fastest, good for real-time
- `openai/whisper-base.en` (~142MB) - Recommended balance
- `openai/whisper-small.en` (~466MB) - Higher accuracy
- `openai/whisper-medium.en` (~1.5GB) - Best accuracy

### Step 6: Test Migration

#### Basic Functionality Test
```bash
# Test with default settings
./target/release/coldvox --stt-backend candle-whisper --list-devices

# Test with specific model
./target/release/coldvox --stt-backend candle-whisper --model openai/whisper-base.en
```

#### Performance Test
```bash
# Monitor startup time
time ./target/release/coldvox --stt-backend candle-whisper

# Monitor memory usage
./target/release/coldvox --stt-backend candle-whisper &
pid=$(pgrep coldvox)
watch -n 1 "ps -p $pid -o pid,ppid,pcpu,pmem,vsz,rss"
```

## Configuration Reference

### Complete Configuration Example

#### config/plugins.json
```json
{
  "stt": {
    "backend": "candle-whisper",
    "model_path": "openai/whisper-base.en",
    "device": "auto",
    "language": "en",
    "include_words": true,
    "streaming": true,
    "buffer_size_ms": 512,
    "max_alternatives": 1,
    "partial_results": true
  },
  "vad": {
    "backend": "silero",
    "threshold": 0.5,
    "min_speech_duration_ms": 250,
    "max_speech_duration_s": 30
  },
  "text_injection": {
    "backend": "auto",
    "confirm_before_inject": true,
    "preview_inject": true
  }
}
```

### Environment Variables Reference

| Variable | Default | Description |
|----------|---------|-------------|
| `CANDLE_WHISPER_DEVICE` | `auto` | Device selection (auto/cpu/cuda) |
| `WHISPER_MODEL_PATH` | Model ID | Model path or Hugging Face model ID |
| `WHISPER_LANGUAGE` | None | Language hint for transcription |
| `HF_HOME` | `~/.cache/huggingface` | Hugging Face cache directory |
| `CANDLE_LOG_LEVEL` | `info` | Logging level (debug/info/warn/error) |

## Troubleshooting

### Common Issues and Solutions

#### 1. Model Download Failures
```bash
# Check internet connection
ping huggingface.co

# Check disk space
df -h

# Clear cache and retry
rm -rf ~/.cache/huggingface/hub
./coldvox --stt-backend candle-whisper
```

#### 2. CUDA Issues
```bash
# Check CUDA installation
nvidia-smi

# Check CUDA version
nvcc --version

# Fallback to CPU
export CANDLE_WHISPER_DEVICE=cpu
```

#### 3. Memory Issues
```bash
# Use smaller model
export WHISPER_MODEL_PATH="openai/whisper-tiny"

# Monitor memory usage
top -p $(pgrep coldvox)

# Enable quantization (reduce memory)
# Note: This feature is planned for future release
```

#### 4. Permission Issues
```bash
# Fix model directory permissions
chmod -R 755 ~/.cache/huggingface

# Use custom cache directory
export HF_HOME="/tmp/coldvox_cache"
mkdir -p "$HF_HOME"
```

#### 5. Build Issues
```bash
# Update Rust toolchain
rustup update

# Clean build
cargo clean
cargo build --features "candle-whisper,text-injection"

# Check system dependencies (Linux)
sudo apt install build-essential cmake pkg-config
```

### Performance Optimization

#### GPU Optimization
```bash
# Force GPU usage
export CANDLE_WHISPER_DEVICE=cuda

# Monitor GPU usage
nvidia-smi -l 1

# Check VRAM usage
nvidia-smi --query-gpu=memory.used,memory.total --format=csv
```

#### CPU Optimization
```bash
# Set CPU optimization flags
export RUSTFLAGS="-C target-cpu=native"

# Monitor CPU usage
top -p $(pgrep coldvox)
```

## Validation Checklist

### Pre-Migration
- [ ] Current Python faster-whisper setup documented
- [ ] Performance baseline established
- [ ] Test audio files prepared
- [ ] System requirements verified

### Migration
- [ ] Python dependencies removed (if desired)
- [ ] Candle Whisper version built successfully
- [ ] Configuration files updated
- [ ] Environment variables set
- [ ] Model downloaded and verified

### Post-Migration
- [ ] Basic transcription functionality works
- [ ] Performance meets or exceeds previous setup
- [ ] Memory usage within acceptable limits
- [ ] Text injection functionality works
- [ ] VAD integration functions properly
- [ ] Error handling works as expected

### Production Readiness
- [ ] Startup time acceptable (< 10 seconds)
- [ ] Memory usage stable
- [ ] No memory leaks detected
- [ ] Error recovery works
- [ ] Logging provides sufficient detail
- [ ] Configuration backup created

## Rollback Plan

If issues occur during migration:

### Immediate Rollback
```bash
# Switch back to Python backend
# Edit config/plugins.json:
{
  "stt": {
    "backend": "whisper",  # Change back to whisper
    "model_path": "base.en"
  }
}

# Rebuild without candle-whisper
cargo build --release --features "text-injection,whisper"
```

### Complete Rollback
```bash
# Reinstall Python dependencies
pip install faster-whisper

# Revert configuration
git checkout HEAD -- config/plugins.json

# Rebuild and test
cargo build --release --features "text-injection,whisper"
```

## Support and Resources

### Documentation
- [Candle Whisper Implementation Guide](../implementation/final-implementation-summary.md)
- [ColdVox Main Documentation](../../README.md)
- [Plugin Architecture Guide](../architecture/plugin-system.md)

### Testing Resources
- [Test Audio Files](../../test/test_audio/)
- [Integration Test Suite](../testing/integration-tests.md)
- [Performance Benchmarks](../performance/benchmarks.md)

### Community
- ColdVox GitHub Issues
- Candle Framework Documentation
- Rust Community Forums

---

**Migration Status**: Ready for Production  
**Risk Level**: Low  
**Estimated Migration Time**: 30-60 minutes  
**Rollback Time**: 5-10 minutes  

*This guide ensures a smooth transition from Python-based STT to the new pure Rust implementation while maintaining all functionality and improving performance.*