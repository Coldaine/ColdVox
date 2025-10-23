# Whisper Model Size Configuration Guide

This document explains how to configure Whisper model sizes for different environments in ColdVox.

## Overview

ColdVox supports multiple Whisper model sizes with automatic selection based on environment, available memory, and explicit configuration. The system intelligently chooses an appropriate model size to balance accuracy, speed, and resource usage.

## Available Model Sizes

| Model Size | Memory Usage | Speed | Accuracy | Best For |
|------------|--------------|-------|----------|----------|
| tiny | ~100MB | Fastest | Basic | CI/Testing, resource-constrained systems |
| base | ~200MB | Fast | Good | Development, general use |
| small | ~500MB | Medium | Better | Production, improved accuracy |
| medium | ~1500MB | Slow | Good | High-accuracy needs, ample resources |
| large / large-v2 / large-v3 | ~3000MB | Slowest | Best | Maximum accuracy, powerful hardware |

## Configuration Methods

### 1. Environment Variable (Highest Priority)

Set the `WHISPER_MODEL_SIZE` environment variable to override automatic selection:

```bash
# Set model size for current session
export WHISPER_MODEL_SIZE=small

# Set model size for a single command
WHISPER_MODEL_SIZE=medium coldvox

# Available values: tiny, base, small, medium, large, large-v2, large-v3
```

### 2. Configuration File

Edit `config/default.toml` to set the default model size:

```toml
[stt]
model_size = "base"  # Default model size
```

### 3. Environment-Specific Configuration

The `plugins.json` file defines environment-specific defaults:

```json
{
  "model_size": {
    "default": "base",
    "environments": {
      "ci": "tiny",
      "development": "base",
      "production": "small"
    }
  }
}
```

## Environment Detection

ColdVox automatically detects the environment and selects an appropriate model size:

### CI Environment
Detected by presence of CI-related environment variables:
- `CI`
- `CONTINUOUS_INTEGRATION`
- `GITHUB_ACTIONS`
- `GITLAB_CI`
- `TRAVIS`
- `CIRCLECI`
- `JENKINS_URL`
- `BUILDKITE`

**Recommended model:** `tiny` (conserves resources)

### Development Environment
Detected by:
- `RUST_BACKTRACE` environment variable
- `DEBUG` environment variable
- `DEV` environment variable
- Presence of `.git` directory

**Recommended model:** `base` (balanced for development)

### Production Environment
Default when neither CI nor development indicators are present.

**Recommended model:** `small` (better accuracy for production use)

## Memory-Based Selection

When enabled, ColdVox can select model size based on available system memory:

```json
{
  "model_size": {
    "memory_based_selection": {
      "enabled": true,
      "thresholds": {
        "tiny_mb": 500,
        "base_mb": 1000,
        "small_mb": 2000,
        "medium_mb": 4000
      }
    }
  }
}
```

Memory thresholds:
- < 500MB: `tiny`
- 500-1000MB: `base`
- 1000-2000MB: `small`
- 2000-4000MB: `medium`
- > 4000MB: `base` (conservative default for stability)

## Priority Order

Model size selection follows this priority order:

1. `WHISPER_MODEL_SIZE` environment variable (explicit override)
2. Environment-specific default (ci/development/production)
3. Memory-based selection (if enabled and memory can be determined)
4. Configuration file default
5. Hardcoded fallback (`base`)

## Recommendations by Use Case

### Development Workstations
```bash
export WHISPER_MODEL_SIZE=base
```
Balanced performance and accuracy for development work.

### Continuous Integration
```bash
export WHISPER_MODEL_SIZE=tiny
```
Minimize resource usage in CI pipelines.

### Production Servers
```bash
export WHISPER_MODEL_SIZE=small
```
Better accuracy for production workloads.

### High-Performance Systems
```bash
export WHISPER_MODEL_SIZE=medium
```
For systems with ample memory and CPU resources.

### Resource-Constrained Devices
```bash
export WHISPER_MODEL_SIZE=tiny
```
For devices with limited memory or CPU.

## Troubleshooting

### Model Loading Issues
If the model fails to load due to insufficient memory:
1. Set a smaller model size: `export WHISPER_MODEL_SIZE=tiny`
2. Check available memory with system monitoring tools
3. Consider increasing system memory or using a more powerful machine

### Performance Issues
If transcription is too slow:
1. Try a smaller model size: `export WHISPER_MODEL_SIZE=base`
2. Check if CPU is being throttled
3. Consider GPU acceleration if available

### Accuracy Issues
If transcription accuracy is insufficient:
1. Try a larger model size: `export WHISPER_MODEL_SIZE=small`
2. Ensure audio quality is good (clear speech, minimal background noise)
3. Check that the correct language is configured

## Advanced Configuration

### Custom Model Path
For custom or fine-tuned models:

```bash
export WHISPER_MODEL_PATH=/path/to/custom/model
```

### Device Selection
Force specific compute device:

```bash
# Use CPU (default)
export WHISPER_DEVICE=cpu

# Use CUDA GPU (if available)
export WHISPER_DEVICE=cuda

# Use OpenCL (if available)
export WHISPER_DEVICE=opencl
```

### Compute Type
Control model precision:

```bash
# 8-bit integer (fastest, less accurate)
export WHISPER_COMPUTE=int8

# 16-bit floating point (balanced)
export WHISPER_COMPUTE=float16

# 32-bit floating point (most accurate, slowest)
export WHISPER_COMPUTE=float32