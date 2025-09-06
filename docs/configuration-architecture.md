# ColdVox Configuration Architecture

## Status: Design Document (Revised)
**Date:** September 6, 2025
**Revision:** Aligned with existing codebase structures
**Purpose:** Establish a comprehensive configuration system to eliminate magic numbers and prepare for GUI settings management

## Current State Analysis

### What Already Exists
- **InjectionConfig**: Detailed configuration in `crates/coldvox-text-injection/src/types.rs`
- **UnifiedVadConfig**: VAD settings in `crates/coldvox-vad/src/config.rs`
- **AudioConfig**: Basic struct in `crates/coldvox-foundation/src/error.rs` (only silence_threshold)
- **Serde support**: Already used in InjectionConfig and VadConfig

### What's Missing
- **ConfigManager**: No centralized configuration loading/management
- **Configuration files**: No TOML/JSON config file support
- **CLI overrides**: Arguments parsed but not integrated with config system
- **Centralized AudioConfig**: Audio settings scattered across multiple files
- **Persistence**: No saving of user preferences
- **Hot reload**: No runtime configuration updates

## Design Goals

1. **Centralization**: Single source of truth for all configuration
2. **Type Safety**: Leverage Rust's type system for validation
3. **Flexibility**: Support multiple configuration sources
4. **Extensibility**: Easy to add new settings
5. **GUI-Ready**: Foundation for future settings UI
6. **Live Updates**: Support runtime configuration changes where possible
7. **Documentation**: Self-documenting configuration structure

## Architecture Overview

### Configuration Layer Hierarchy

Configuration sources in priority order (highest to lowest):
1. **Command-line arguments** - Override any setting for this run
2. **Environment variables** - `COLDVOX_*` prefixed
3. **User config file** - `~/.config/coldvox/config.toml`
4. **System config file** - `/etc/coldvox/config.toml`
5. **Built-in defaults** - Hardcoded fallback values

### Module Structure

```
crates/coldvox-foundation/src/config/
├── mod.rs           # Main AppConfig struct and ConfigManager
├── audio.rs         # Audio configuration
├── vad.rs          # VAD configuration
├── stt.rs          # STT configuration
├── hotkey.rs       # Hotkey configuration
├── injection.rs    # Text injection configuration
├── gui.rs          # GUI preferences
└── validation.rs   # Constraint validation
```

## Configuration Schema

### Core Configuration Structure

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppConfig {
    #[serde(default)]
    pub audio: AudioConfig,

    #[serde(default)]
    pub vad: coldvox_vad::UnifiedVadConfig,  // Reuse existing

    #[serde(default)]
    pub stt: SttConfig,

    #[serde(default)]
    pub hotkeys: HotkeyConfig,

    #[serde(default)]
    pub text_injection: coldvox_text_injection::InjectionConfig,  // Reuse existing

    #[serde(default)]
    pub gui: GuiConfig,

    #[serde(default)]
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    /// Target sample rate in Hz
    #[serde(default = "default_sample_rate")]
    pub sample_rate: u32,

    /// Audio frame size in samples
    #[serde(default = "default_frame_size")]
    pub frame_size: usize,

    /// Resampler quality: "fast", "balanced", "quality"
    #[serde(default = "default_resampler_quality")]
    pub resampler_quality: ResamplerQuality,

    /// Preferred audio device (None = system default)
    pub device: Option<String>,

    /// Silence threshold (i16 amplitude) - migrating from existing AudioConfig
    #[serde(default = "default_silence_threshold")]
    pub silence_threshold: i16,

    /// Silence threshold in dB (additional option)
    #[serde(default = "default_silence_threshold_db")]
    pub silence_threshold_db: Option<f32>,

    /// Watchdog timeout in seconds
    #[serde(default = "default_watchdog_timeout")]
    pub watchdog_timeout_secs: u64,
}

// Note: VadConfig reuses the existing UnifiedVadConfig from coldvox-vad crate
// with its SileroConfig sub-structure. Activation mode is tracked separately.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeyConfig {
    /// Push-to-talk key combination
    #[serde(default = "default_ptt_hotkey")]
    pub push_to_talk: String,

    /// Toggle recording hotkey
    pub toggle_recording: Option<String>,

    /// Cancel/stop hotkey
    pub cancel: Option<String>,
}

// Note: TextInjectionConfig reuses the existing comprehensive InjectionConfig
// from crates/coldvox-text-injection/src/types.rs which includes:
// - Method-specific timeouts and cooldowns
// - Success rate tracking
// - Keystroke/paste modes
// - Focus caching
// - Application allowlist/blocklist
// See the existing struct for full details.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SttConfig {
    /// Enable STT processing
    #[serde(default = "default_stt_enabled")]
    pub enabled: bool,

    /// Path to Vosk model directory
    pub model_path: Option<PathBuf>,

    /// Language code (e.g., "en-US")
    #[serde(default = "default_language")]
    pub language: String,

    /// Save transcriptions to disk
    #[serde(default)]
    pub save_transcriptions: bool,

    /// Output directory for saved transcriptions
    pub output_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiConfig {
    /// Theme preference
    #[serde(default = "default_theme")]
    pub theme: String,

    /// Overlay window opacity
    #[serde(default = "default_overlay_opacity")]
    pub overlay_opacity: f32,

    /// Window position
    pub window_position: Option<String>,

    /// Show on startup
    #[serde(default = "default_show_on_startup")]
    pub show_on_startup: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level
    #[serde(default = "default_log_level")]
    pub level: String,

    /// Enable file logging
    #[serde(default = "default_file_enabled")]
    pub file_enabled: bool,

    /// Log file path
    #[serde(default = "default_log_path")]
    pub file_path: PathBuf,

    /// File rotation strategy
    #[serde(default = "default_rotation")]
    pub file_rotation: String,
}
```

### Default Functions

```rust
// Audio defaults
fn default_sample_rate() -> u32 { 16000 }
fn default_frame_size() -> usize { 512 }
fn default_silence_threshold() -> i16 { 100 }  // Matching existing AudioConfig
fn default_silence_threshold_db() -> Option<f32> { None }
fn default_watchdog_timeout() -> u64 { 5 }
fn default_resampler_quality() -> ResamplerQuality { ResamplerQuality::Balanced }

// Hotkey defaults
fn default_ptt_hotkey() -> String { "Ctrl+Alt+Space".to_string() }

// STT defaults
fn default_stt_enabled() -> bool { cfg!(feature = "vosk") }
fn default_language() -> String { "en-US".to_string() }

// GUI defaults
fn default_theme() -> String { "dark".to_string() }
fn default_overlay_opacity() -> f32 { 0.8 }
fn default_show_on_startup() -> bool { true }

// Logging defaults
fn default_log_level() -> String { "info".to_string() }
fn default_file_enabled() -> bool { true }
fn default_log_path() -> PathBuf { PathBuf::from("logs/coldvox.log") }
fn default_rotation() -> String { "daily".to_string() }
```

## Configuration Files

### TOML Format Example

```toml
# ~/.config/coldvox/config.toml
# ColdVox User Configuration

[audio]
# Audio processing settings
sample_rate = 16000
frame_size = 512
resampler_quality = "balanced"  # Options: "fast", "balanced", "quality"
# device = "USB Microphone"      # Uncomment to set specific device
silence_threshold = 100          # i16 amplitude value
# silence_threshold_db = -40.0   # Alternative: specify in dB
watchdog_timeout_secs = 5

[vad]
# Using the existing UnifiedVadConfig structure
mode = "silero"                  # Currently only "silero" is supported
frame_size_samples = 512
sample_rate_hz = 16000

[vad.silero]
# Silero VAD settings (from existing SileroConfig)
threshold = 0.3                  # Detection sensitivity (0.0-1.0)
min_speech_duration_ms = 250    # Minimum speech to trigger
min_silence_duration_ms = 500   # Silence duration to end speech (increased from default 100)
window_size_samples = 512

[stt]
enabled = true
model_path = "models/vosk-model-small-en-us-0.15"
language = "en-US"
# Alternative model paths can be specified:
# model_path = "~/.local/share/coldvox/models/vosk-model-en-us-0.22"

[hotkeys]
push_to_talk = "Ctrl+Alt+Space"
toggle_recording = "Ctrl+Alt+R"
cancel = "Escape"

[text_injection]
# Using the existing comprehensive InjectionConfig
allow_ydotool = false            # Requires external binary and uinput
allow_kdotool = false            # External KDE tool
allow_enigo = false              # Wayland/libei paths
restore_clipboard = false        # Restore clipboard after injection
inject_on_unknown_focus = true  # Allow when focus unknown (Wayland)
require_focus = false            # Require editable focus
redact_logs = true               # Privacy-first logging

# Timing and performance
max_total_latency_ms = 1000     # Overall timeout
per_method_timeout_ms = 200     # Per-method timeout
paste_action_timeout_ms = 500   # Paste action timeout
focus_cache_duration_ms = 1000  # Focus status cache

# Cooldown and retry logic
cooldown_initial_ms = 100       # Initial cooldown after failure
cooldown_backoff_factor = 2.0   # Exponential backoff
cooldown_max_ms = 5000          # Maximum cooldown

# Injection behavior
injection_mode = "paste"         # "keystroke", "paste", or "auto"
keystroke_rate_cps = 30         # Characters per second for keystroke mode
max_burst_chars = 100           # Max chars in single burst
paste_chunk_chars = 500         # Chunk size for paste operations
chunk_delay_ms = 50             # Delay between chunks

# Success tracking
min_success_rate = 0.5          # Minimum success rate before fallback
min_sample_size = 10            # Samples before trusting success rate

# Application filtering (regex patterns)
# allowlist = ["firefox", "chrome", "code"]
# blocklist = ["keepass", "1password"]

[gui]
# GUI preferences (for future use)
theme = "dark"
overlay_opacity = 0.8
window_position = "bottom-right"
show_on_startup = true

[logging]
level = "info"                   # Options: "error", "warn", "info", "debug", "trace"
file_enabled = true
file_path = "logs/coldvox.log"
file_rotation = "daily"          # Options: "daily", "hourly", "never"
```

## Configuration Manager

### Core Implementation

```rust
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;
use std::path::{Path, PathBuf};

pub struct ConfigManager {
    /// Current active configuration
    config: Arc<RwLock<AppConfig>>,

    /// Configuration file paths
    user_config_path: PathBuf,
    system_config_path: PathBuf,

    /// Change notification channel
    change_tx: broadcast::Sender<ConfigChangeEvent>,

    /// File watcher for hot reload
    #[cfg(feature = "hot-reload")]
    watcher: Option<notify::RecommendedWatcher>,
}

impl ConfigManager {
    /// Load configuration from all sources
    pub fn load() -> Result<Self, ConfigError> {
        let mut config = AppConfig::default();

        // Layer 1: System config
        if let Ok(system_config) = Self::load_toml("/etc/coldvox/config.toml") {
            config.merge(system_config)?;
        }

        // Layer 2: User config
        let user_path = Self::user_config_path()?;
        if let Ok(user_config) = Self::load_toml(&user_path) {
            config.merge(user_config)?;
        }

        // Layer 3: Environment variables
        config.merge_env()?;

        // Layer 4: CLI arguments (handled by caller)

        // Validate final configuration
        config.validate()?;

        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            // ... other fields
        })
    }

    /// Get current configuration
    pub fn get(&self) -> AppConfig {
        self.config.read().unwrap().clone()
    }

    /// Update configuration section
    pub fn update<F>(&self, updater: F) -> Result<(), ConfigError>
    where
        F: FnOnce(&mut AppConfig) -> Result<(), ConfigError>,
    {
        let mut config = self.config.write().unwrap();
        updater(&mut *config)?;
        config.validate()?;

        // Notify subscribers
        let _ = self.change_tx.send(ConfigChangeEvent::Updated);

        Ok(())
    }

    /// Subscribe to configuration changes
    pub fn subscribe(&self) -> broadcast::Receiver<ConfigChangeEvent> {
        self.change_tx.subscribe()
    }

    /// Save current configuration to user file
    pub fn save(&self) -> Result<(), ConfigError> {
        let config = self.config.read().unwrap();
        let toml = toml::to_string_pretty(&*config)?;
        std::fs::write(&self.user_config_path, toml)?;
        Ok(())
    }
}
```

## Integration Points

### 1. Runtime Integration

```rust
// In runtime.rs
pub async fn start(
    opts: AppRuntimeOptions,
    config: Arc<ConfigManager>,
) -> Result<AppHandle, Box<dyn std::error::Error + Send + Sync>> {
    let app_config = config.get();

    // Use configuration values instead of hardcoded constants
    let vad_cfg = UnifiedVadConfig {
        mode: app_config.vad.mode,
        silero: SileroConfig {
            threshold: app_config.vad.silero.threshold,
            min_speech_duration_ms: app_config.vad.silero.min_speech_duration_ms,
            min_silence_duration_ms: app_config.vad.silero.min_silence_duration_ms,
            window_size_samples: app_config.vad.silero.window_size_samples,
        },
        frame_size_samples: app_config.audio.frame_size,
        sample_rate_hz: app_config.audio.sample_rate,
    };

    // Subscribe to configuration changes
    let mut config_rx = config.subscribe();
    tokio::spawn(async move {
        while let Ok(event) = config_rx.recv().await {
            // Handle configuration updates
            match event {
                ConfigChangeEvent::Updated => {
                    // Reload applicable settings
                }
            }
        }
    });

    // ... rest of initialization
}
```

### 2. CLI Integration

```rust
// In main.rs - consolidate all CLI arguments
#[derive(Parser)]
struct Cli {
    /// Override config file path
    #[arg(long)]
    config: Option<PathBuf>,

    /// Audio device name or index
    #[arg(short = 'D', long)]
    device: Option<String>,

    /// Activation mode: vad or hotkey
    #[arg(long)]
    activation_mode: Option<String>,

    /// VAD threshold (0.0-1.0)
    #[arg(long)]
    vad_threshold: Option<f32>,

    /// Min silence duration in ms
    #[arg(long)]
    min_silence_ms: Option<u32>,

    /// Resampler quality: fast, balanced, quality
    #[arg(long)]
    resampler_quality: Option<String>,

    /// Enable STT transcription
    #[arg(long)]
    enable_stt: bool,

    /// Vosk model path
    #[arg(long)]
    vosk_model: Option<PathBuf>,

    /// Save transcriptions to disk
    #[arg(long)]
    save_transcriptions: bool,

    /// Output directory for transcriptions
    #[arg(long)]
    output_dir: Option<PathBuf>,

    /// Log level (error, warn, info, debug, trace)
    #[arg(long)]
    log_level: Option<String>,

    /// Generate default config file
    #[arg(long)]
    generate_config: bool,

    /// Show current configuration
    #[arg(long)]
    show_config: bool,

    /// Validate configuration without running
    #[arg(long)]
    validate_config: bool,
}

fn apply_cli_overrides(config: &mut AppConfig, cli: &Cli) {
    if let Some(device) = &cli.device {
        config.audio.device = Some(device.clone());
    }
    if let Some(threshold) = cli.vad_threshold {
        config.vad.silero.threshold = threshold;
    }
    // ... other overrides
}
```

### 3. GUI Integration (Future)

```rust
// Future GUI settings panel
impl SettingsPanel {
    fn on_vad_threshold_changed(&mut self, value: f32) {
        self.config_manager.update(|config| {
            config.vad.silero.threshold = value;
            Ok(())
        })?;

        // Changes automatically propagate to subscribed components
    }

    fn on_save_clicked(&mut self) {
        self.config_manager.save()?;
        self.show_notification("Settings saved");
    }
}
```

## Implementation Plan

### Complete Implementation Tasks

1. **Create ConfigManager** in `coldvox-foundation/src/config/`
   - Implement load/save/merge functionality
   - Add configuration layering (files → env → CLI)
   - Support for TOML serialization/deserialization
   - Configuration validation with range checks
   - Broadcast channel for configuration change notifications

2. **Extend AudioConfig**
   - Move from simple `silence_threshold` to comprehensive audio settings
   - Centralize all audio constants (sample_rate, frame_size, etc.)
   - Add all fields shown in the schema above

3. **Consolidate CLI parsing**
   - Single `Cli` struct in main.rs with all arguments
   - Feed parsed args to ConfigManager as highest priority
   - Implement `--generate-config`, `--show-config`, `--validate-config` commands

4. **Wire ConfigManager into runtime.rs**
   - Pass config to `start()` function
   - Replace all hardcoded values with config references
   - Subscribe to configuration changes for hot-reload where applicable

5. **Add config file support**
   - Load from `~/.config/coldvox/config.toml`
   - System config from `/etc/coldvox/config.toml`
   - Environment variable support (`COLDVOX_*` prefix)

6. **Unify existing configs**
   - Reuse `UnifiedVadConfig` from coldvox-vad
   - Reuse `InjectionConfig` from coldvox-text-injection
   - Create new `SttConfig` for STT settings
   - Create new `HotkeyConfig` for hotkey bindings
   - Create new `GuiConfig` for future GUI preferences
   - Create new `LoggingConfig` for logging control

7. **Hot reload support**
   - File watcher for config changes (using notify crate)
   - Mark which settings require restart vs runtime update
   - Components subscribe to relevant configuration sections

## Benefits

1. **User Customization**: Users can tune VAD sensitivity, audio settings, hotkeys
2. **No Magic Numbers**: All constants have named, documented homes
3. **Debugging**: Easy to share/compare configurations
4. **Testing**: Can test with different configurations without code changes
5. **Distribution**: Ship with sensible defaults, let users customize
6. **GUI Ready**: Clean API for future settings UI

## Example Usage

### User Workflow
```bash
# Generate default config
coldvox --generate-config > ~/.config/coldvox/config.toml

# Edit config
$EDITOR ~/.config/coldvox/config.toml

# Run with custom config
coldvox --config my-config.toml

# Override specific setting
coldvox --vad-threshold 0.5

# Check current configuration
coldvox --show-config
```

### Developer Workflow
```rust
// Access configuration in any component
let config = config_manager.get();
let threshold = config.vad.silero.threshold;

// Update configuration
config_manager.update(|cfg| {
    cfg.vad.silero.min_silence_duration_ms = 800;
    Ok(())
})?;

// Subscribe to changes
let mut rx = config_manager.subscribe();
while let Ok(_) = rx.recv().await {
    // Reload settings
}
```

## Validation Rules

Configuration validation ensures settings are within acceptable ranges:

```rust
impl AppConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Audio validation
        if self.audio.sample_rate < 8000 || self.audio.sample_rate > 48000 {
            return Err(ConfigError::InvalidSampleRate);
        }

        // VAD validation
        if self.vad.silero.threshold < 0.0 || self.vad.silero.threshold > 1.0 {
            return Err(ConfigError::InvalidThreshold);
        }

        if self.vad.silero.min_silence_duration_ms < 50 {
            return Err(ConfigError::SilenceDurationTooShort);
        }

        // ... other validations

        Ok(())
    }
}
```

## Testing Strategy

1. **Unit Tests**: Test configuration loading, merging, validation
2. **Integration Tests**: Test configuration with runtime components
3. **Config Examples**: Provide example configs for common use cases
4. **Migration Tests**: Ensure config updates don't break existing setups

## Security Considerations

1. **File Permissions**: User config should be user-readable only
2. **Path Validation**: Prevent directory traversal in config paths
3. **Value Sanitization**: Validate all string inputs (device names, paths)
4. **Resource Limits**: Enforce reasonable limits on numeric values

## Key Revisions from Original Plan

This revised architecture aligns with the existing codebase reality:

1. **Reuse Existing Structs**: Rather than creating new configuration structs from scratch, we reuse:
   - `InjectionConfig` from `coldvox-text-injection` (much more comprehensive than originally planned)
   - `UnifiedVadConfig` from `coldvox-vad` (already has the structure we need)

2. **ConfigManager is Priority #1**: The original plan had many configuration structs but the critical missing piece is the `ConfigManager` that ties everything together.

3. **AudioConfig Needs Extension**: The existing `AudioConfig` only has `silence_threshold` - it needs to be expanded to include all audio settings currently hardcoded.

4. **CLI Consolidation Required**: Multiple CLI parsing points exist; they need to be unified into a single `Cli` struct that feeds into the ConfigManager.

5. **Incremental Migration**: Rather than a big-bang rewrite, we can incrementally adopt the configuration system, starting with the most problematic areas (VAD settings with magic numbers).

## Conclusion

This configuration architecture provides a solid foundation for ColdVox settings management. It eliminates magic numbers, enables user customization, and prepares for future GUI integration while maintaining type safety and validation throughout the system.

The revised plan acknowledges existing work (InjectionConfig, UnifiedVadConfig) and focuses on the critical missing infrastructure (ConfigManager) rather than recreating what already exists. This pragmatic approach ensures faster implementation and better alignment with the current codebase.
