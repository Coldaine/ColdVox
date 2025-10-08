use config::{Config, Environment, File};
use serde::Deserialize;
use std::path::Path;
use tracing;

#[derive(Debug, Deserialize, Default)]
pub struct InjectionSettings {
    pub fail_fast: bool,
    pub allow_kdotool: bool,
    pub allow_enigo: bool,
    pub inject_on_unknown_focus: bool,
    pub require_focus: bool,
    pub pause_hotkey: String,
    pub redact_logs: bool,
    pub max_total_latency_ms: u64,
    pub per_method_timeout_ms: u64,
    pub paste_action_timeout_ms: u64,
    pub cooldown_initial_ms: u64,
    pub cooldown_backoff_factor: f64,
    pub cooldown_max_ms: u64,
    pub injection_mode: String,
    pub keystroke_rate_cps: u32,
    pub max_burst_chars: u32,
    pub paste_chunk_chars: u32,
    pub chunk_delay_ms: u64,
    pub focus_cache_duration_ms: u64,
    pub enable_window_detection: bool,
    pub clipboard_restore_delay_ms: u64,
    pub discovery_timeout_ms: u64,
    pub allowlist: Vec<String>,
    pub blocklist: Vec<String>,
    pub min_success_rate: f32,
    pub min_sample_size: u32,
}

#[derive(Debug, Deserialize, Default)]
pub struct SttSettings {
    pub preferred: Option<String>,
    pub fallbacks: Vec<String>,
    pub require_local: bool,
    pub max_mem_mb: Option<u32>,
    pub language: Option<String>,
    pub failover_threshold: u32,
    pub failover_cooldown_secs: u32,
    pub model_ttl_secs: u32,
    pub disable_gc: bool,
    pub metrics_log_interval_secs: u32,
    pub debug_dump_events: bool,
    pub auto_extract: bool,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub device: Option<String>,
    pub resampler_quality: String,
    pub enable_device_monitor: bool,
    pub activation_mode: String,
    #[serde(default)]
    pub injection: InjectionSettings,
    #[serde(default)]
    pub stt: SttSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            device: None,
            resampler_quality: "balanced".to_string(),
            enable_device_monitor: true,
            activation_mode: "vad".to_string(),
            injection: InjectionSettings::default(),
            stt: SttSettings::default(),
        }
    }
}

impl Settings {
    /// Load settings from a specific config file path (for tests)
    pub fn from_path(config_path: impl AsRef<Path>) -> Result<Self, String> {
        let mut builder = Config::builder();

        // Set defaults for required fields to prevent deserialization errors.
        builder = builder
            .set_default("resampler_quality", "balanced").unwrap()
            .set_default("activation_mode", "vad").unwrap()
            .set_default("enable_device_monitor", true).unwrap();

        // Add the specific file source.
        builder = builder.add_source(File::from(config_path.as_ref()).required(true));

        // Add environment variables, which will override the file's settings.
        builder = builder.add_source(
            Environment::with_prefix("COLDVOX")
                .separator("__")
                .list_separator(" "),
        );

        // Build and deserialize
        let config = builder.build().map_err(|e| format!("Failed to build config: {}", e))?;

        let mut settings: Settings = config.try_deserialize().map_err(|e| format!("Failed to deserialize settings: {}", e))?;

        // Validate the final settings
        settings.validate().map_err(|e| e.to_string())?;

        Ok(settings)
    }

    pub fn new() -> Result<Self, String> {
        let mut builder = Config::builder();

        // Set defaults for required fields to prevent deserialization errors if no config file is found.
        builder = builder
            .set_default("resampler_quality", "balanced").unwrap()
            .set_default("activation_mode", "vad").unwrap();

        // Find and add config file source.
        let config_path = Path::new("config/default.toml");
        if config_path.exists() {
            tracing::info!("Loading configuration from: {}", config_path.display());
            builder = builder.add_source(File::from(config_path).required(true));
        } else {
            tracing::warn!("No configuration file at 'config/default.toml'. Using defaults and environment variables.");
        }

        // Add environment variables, which will override the file's settings.
        builder = builder.add_source(
            Environment::with_prefix("COLDVOX")
                .separator("__")
                .list_separator(" "),
        );

        // Build and deserialize
        let config = builder.build().map_err(|e| format!("Failed to build config: {}", e))?;

        let mut settings: Settings = config.try_deserialize().map_err(|e| format!("Failed to deserialize settings: {}", e))?;

        // Validate the final settings
        settings.validate().map_err(|e| e.to_string())?;

        Ok(settings)
    }

    pub fn validate(&mut self) -> Result<(), String> {
        let mut errors = Vec::new();

        // Validate resampler_quality
        if !["fast", "balanced", "quality"].contains(&self.resampler_quality.to_lowercase().as_str()) {
            tracing::warn!("Invalid resampler_quality '{}'. Defaulting to 'balanced'.", self.resampler_quality);
            self.resampler_quality = "balanced".to_string();
        }

        // Validate activation_mode
        if !["vad", "hotkey"].contains(&self.activation_mode.to_lowercase().as_str()) {
            tracing::warn!("Invalid activation_mode '{}'. Defaulting to 'vad'.", self.activation_mode);
            self.activation_mode = "vad".to_string();
        }

        // Validate injection settings
        if self.injection.max_total_latency_ms == 0 {
            errors.push("Injection max_total_latency_ms must be >0".to_string());
        }
        if self.injection.per_method_timeout_ms == 0 {
            errors.push("Injection per_method_timeout_ms must be >0".to_string());
        }
        if self.injection.paste_action_timeout_ms == 0 {
            errors.push("Injection paste_action_timeout_ms must be >0".to_string());
        }
        if self.injection.cooldown_initial_ms == 0 {
            errors.push("Injection cooldown_initial_ms must be >0".to_string());
        }
        if self.injection.cooldown_max_ms == 0 {
            errors.push("Injection cooldown_max_ms must be >0".to_string());
        }
        if self.injection.cooldown_backoff_factor <= 0.0 || self.injection.cooldown_backoff_factor > 10.0 {
            tracing::warn!("Invalid cooldown_backoff_factor {}. Clamping to 2.0.", self.injection.cooldown_backoff_factor);
            self.injection.cooldown_backoff_factor = 2.0;
        }
        if !["keystroke", "paste", "auto"].contains(&self.injection.injection_mode.to_lowercase().as_str()) {
            tracing::warn!("Invalid injection_mode '{}'. Defaulting to 'auto'.", self.injection.injection_mode);
            self.injection.injection_mode = "auto".to_string();
        }
        if self.injection.keystroke_rate_cps == 0 || self.injection.keystroke_rate_cps > 100 {
            tracing::warn!("Invalid keystroke_rate_cps {}. Clamping to 20.", self.injection.keystroke_rate_cps);
            self.injection.keystroke_rate_cps = 20;
        }
        if self.injection.max_burst_chars == 0 {
            errors.push("Injection max_burst_chars must be >0".to_string());
        }
        if self.injection.paste_chunk_chars == 0 {
            errors.push("Injection paste_chunk_chars must be >0".to_string());
        }
        if self.injection.chunk_delay_ms == 0 {
            errors.push("Injection chunk_delay_ms must be >0".to_string());
        }
        if self.injection.focus_cache_duration_ms == 0 {
            errors.push("Injection focus_cache_duration_ms must be >0".to_string());
        }
        if self.injection.clipboard_restore_delay_ms == 0 {
            errors.push("Injection clipboard_restore_delay_ms must be >0".to_string());
        }
        if self.injection.discovery_timeout_ms == 0 {
            errors.push("Injection discovery_timeout_ms must be >0".to_string());
        }
        if self.injection.min_success_rate < 0.0 || self.injection.min_success_rate > 1.0 {
            tracing::warn!("Invalid min_success_rate {}. Clamping to 0.3.", self.injection.min_success_rate);
            self.injection.min_success_rate = 0.3;
        }
        if self.injection.min_sample_size == 0 {
            errors.push("Injection min_sample_size must be >0".to_string());
        }

        // Validate STT settings
        if self.stt.failover_threshold == 0 {
            errors.push("STT failover_threshold must be >0".to_string());
        }
        if self.stt.failover_cooldown_secs == 0 {
            errors.push("STT failover_cooldown_secs must be >0".to_string());
        }
        if self.stt.model_ttl_secs == 0 {
            errors.push("STT model_ttl_secs must be >0".to_string());
        }

        if !errors.is_empty() {
            let error_msg = format!("Critical config validation errors: {:?}", errors);
            return Err(error_msg);
        }

        // Log non-critical warnings if any were applied
        tracing::info!("Configuration validation completed successfully.");

        Ok(())
    }
}

pub mod audio;
pub mod clock;
pub mod foundation;
pub mod hotkey;
pub mod probes;
pub mod runtime;
pub mod sleep_instrumentation;
pub mod stt;
pub mod telemetry;
#[cfg(feature = "text-injection")]
pub mod text_injection;
#[cfg(feature = "tui")]
pub mod tui;
pub mod vad;

#[cfg(test)]
pub mod test_utils;