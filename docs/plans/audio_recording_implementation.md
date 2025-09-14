# Audio Recording Implementation Plan

## Current State Analysis

After reviewing the codebase, we've confirmed that the TUI does not currently record or save raw audio while running. The analysis is correct:

1. The TUI only subscribes to VAD and STT events after starting the runtime
2. There is no subscription to raw audio frames in the TUI
3. The runtime `AppHandle` exposes audio frames internally via a broadcast channel (`audio_tx`)
4. The `AppHandle` already has a public `subscribe_audio()` method

## Issues with Original Plan

The original plan proposed two options:

1. Add audio subscription to runtime and implement dumper in TUI
2. Implement dumper inside the runtime

However, there are some issues with this approach:

1. **Mixing Concerns**: Implementing the audio dumper in the TUI mixes UI concerns with file I/O operations
2. **Missing Configuration**: The plan doesn't address how to make audio dumping configurable
3. **File Management**: No consideration for file naming, rotation, or error handling
4. **Partially Implemented**: The `subscribe_audio()` method already exists in `AppHandle`

## Recommended Approach

We recommend a hybrid approach that builds on the existing infrastructure:

### 1. Runtime-Centric Implementation

Implement the audio dumper as part of the runtime (similar to option 2) but make it configurable through `AppRuntimeOptions`:

```rust
/// Options for starting the ColdVox runtime
#[derive(Clone, Debug)]
pub struct AppRuntimeOptions {
    pub device: Option<String>,
    pub resampler_quality: ResamplerQuality,
    pub activation_mode: ActivationMode,
    /// STT plugin selection configuration
    pub stt_selection: Option<coldvox_stt::plugin::PluginSelectionConfig>,
    /// Audio dumping configuration
    pub audio_dump: Option<AudioDumpConfig>,  // NEW
    // ... other options
}

/// Configuration for audio dumping
#[derive(Clone, Debug)]
pub struct AudioDumpConfig {
    pub enabled: bool,
    pub directory: Option<String>,  // Default: logs/audio_dumps/
    pub format: AudioDumpFormat,    // Raw PCM, WAV, etc.
}

#[derive(Clone, Debug)]
pub enum AudioDumpFormat {
    RawPCM,
    WAV,
}
```

### 2. Enhanced AppHandle

Add methods to `AppHandle` to control audio dumping:

```rust
impl AppHandle {
    /// Start audio dumping with the given configuration
    pub async fn start_audio_dump(&self, config: AudioDumpConfig) -> Result<(), Box<dyn std::error::Error>> {
        // Implementation details
    }

    /// Stop audio dumping
    pub async fn stop_audio_dump(&self) {
        // Implementation details
    }

    /// Check if audio dumping is currently active
    pub fn is_audio_dump_active(&self) -> bool {
        // Implementation details
    }
}
```

### 3. CLI Integration

Add CLI flags to the TUI to enable this feature:

```rust
struct Cli {
    /// Audio device name
    #[arg(short = 'D', long)]
    device: Option<String>,

    /// Enable audio dumping
    #[arg(long = "dump-audio")]
    dump_audio: bool,

    /// Directory for audio dumps
    #[arg(long = "dump-dir")]
    dump_dir: Option<String>,

    // ... other options
}
```

### 4. Implementation Details

#### Audio Dumper Implementation
- Use a buffered writer for efficiency
- Drop frames if the writer lags (log drop counts at debug/trace level)
- Default format: raw PCM LE i16 (fast and unchanging)
- Optional WAV support (write header then backpatch length on close)
- Default directory: logs/audio_dumps/ with timestamped filenames
- Honor COLDVOX_DISABLE_AUDIO_DUMP environment variable as emergency kill switch

#### File Naming and Rotation
- Format: `logs/audio_dumps/session_<timestamp>.pcm` or `.wav`
- Rotate per session (new file when pipeline starts)
- Consider size-based rotation for long sessions

#### Error Handling
- Graceful degradation if disk space is low
- Continue operation even if audio dumping fails
- Log errors but don't crash the application

## Benefits of This Approach

1. **Separation of Concerns**: Runtime handles audio dumping, TUI handles UI
2. **Configurability**: Audio dumping is configurable through runtime options
3. **Reusability**: Any binary that uses the runtime can enable audio dumping
4. **Existing Infrastructure**: Leverages the already existing `subscribe_audio()` method
5. **Control**: Provides methods to start/stop dumping at runtime

## Implementation Steps

1. Add `AudioDumpConfig` to `AppRuntimeOptions`
2. Implement audio dumper task in the runtime
3. Add start/stop methods to `AppHandle`
4. Add CLI flags to TUI
5. Wire up the feature in TUI startup
6. Test with various configurations
7. Document the feature

This approach provides a clean, maintainable solution that follows the existing architecture patterns while addressing the original need for audio recording capability.
