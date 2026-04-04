---
doc_type: reference
subsystem: stt
version: 1.0.0
status: draft
owners: STT Team
last_reviewed: 2025-11-09
---

# Whisper Backend (Legacy)

The previous Python-dependent Whisper backend and its model-size guidance have been removed as part of a backend pivot.

Next steps:
- A new pure-Rust Whisper backend will replace the legacy one.
- Until then, select `mock` or another available STT plugin in `config/plugins.json`.

This page will be replaced with updated configuration guidance when the new backend lands.

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

Canonical STT selection and model configuration lives in `config/plugins.json`. Legacy files like `./plugins.json` or `crates/app/plugins.json` are deprecated and ignored at runtime (a warning is logged on startup if they exist).

You can also set defaults in `config/default.toml`:

```toml
[stt]
model_size = "base"  # Default model size
```

### 3. Environment-Specific Configuration

The canonical STT selection configuration lives at `config/plugins.json`.
This file can define environment-specific defaults:

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

Behavior:
- Defaults to `base` for balanced development.
- On high-performance desktops with ample free memory (>= 12 GB available), automatically prefers `large-v3` for maximum accuracy.
  - You can always override with `WHISPER_MODEL_SIZE`.

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

Memory thresholds (general guidance):
- < 500MB: `tiny`
- 500-1000MB: `base`
- 1000-2000MB: `small`
- 2000-4000MB: `medium`
- > 4000MB: `base` (conservative default for stability)

Note: In the Development environment only, if available memory is >= 12 GB, ColdVox will auto-select `large-v3`.

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
On powerful developer workstations (>= 12 GB available), `large-v3` is selected automatically in the Development environment.
To force selection explicitly or on other environments:
```bash
export WHISPER_MODEL_SIZE=large-v3
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

## Future Migration to Candle

ColdVox is planning a migration from the current `faster-whisper-rs` implementation to the Candle ML framework's Whisper implementation. This migration aims to leverage Candle's performance, Rust-native ecosystem, and flexibility.

For detailed information about this migration plan, see: [Candle Whisper Integration Plan](plans/stt-candle-whisper-migration.md)

The migration will not affect the model configuration options described in this document, as the new implementation will maintain compatibility with the existing configuration system.

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
### Simulate Available Memory (advanced/testing)
You can simulate available memory detection (useful for testing) by setting:
```bash
export WHISPER_AVAILABLE_MEM_MB=16384
```
This overrides the system probe used for memory-based selection.
Control model precision:

```bash
# 8-bit integer (fastest, less accurate)
export WHISPER_COMPUTE=int8

# 16-bit floating point (balanced)
export WHISPER_COMPUTE=float16

# 32-bit floating point (most accurate, slowest)
export WHISPER_COMPUTE=float32