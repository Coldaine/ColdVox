use config::{Case, Config, ConfigError, Environment, File};
use serde::Deserialize;
use std::env;
use std::path::{Path, PathBuf};
use tracing;

#[derive(Debug, Deserialize)]
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

impl Default for InjectionSettings {
    fn default() -> Self {
        Self {
            fail_fast: false,
            allow_kdotool: false,
            allow_enigo: false,
            inject_on_unknown_focus: true,
            require_focus: false,
            pause_hotkey: "".to_string(),
            redact_logs: true,
            max_total_latency_ms: 800,
            per_method_timeout_ms: 250,
            paste_action_timeout_ms: 200,
            cooldown_initial_ms: 10000,
            cooldown_backoff_factor: 2.0,
            cooldown_max_ms: 300000,
            injection_mode: "auto".to_string(),
            keystroke_rate_cps: 20,
            max_burst_chars: 50,
            paste_chunk_chars: 500,
            chunk_delay_ms: 30,
            focus_cache_duration_ms: 200,
            enable_window_detection: true,
            clipboard_restore_delay_ms: 500,
            discovery_timeout_ms: 1000,
            allowlist: Vec::new(),
            blocklist: Vec::new(),
            min_success_rate: 0.3,
            min_sample_size: 5,
        }
    }
}

#[derive(Debug, Deserialize)]
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

impl Default for SttSettings {
    fn default() -> Self {
        Self {
            preferred: None,
            fallbacks: Vec::new(),
            require_local: false,
            max_mem_mb: None,
            language: None,
            failover_threshold: 5,
            failover_cooldown_secs: 10,
            model_ttl_secs: 300,
            disable_gc: false,
            metrics_log_interval_secs: 30,
            debug_dump_events: false,
            auto_extract: true,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub device: Option<String>,
    pub resampler_quality: String,
    pub enable_device_monitor: bool,
    pub activation_mode: String,
    pub injection: InjectionSettings,
    pub stt: SttSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
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
    fn build_config(explicit_path: Option<PathBuf>) -> Result<Config, ConfigError> {
        let mut builder = Config::builder()
            .set_default("resampler_quality", "balanced")?
            .set_default("activation_mode", "vad")?
            .set_default("enable_device_monitor", true)?
            // Injection settings defaults
            .set_default("injection.fail_fast", false)?
            .set_default("injection.allow_kdotool", false)?
            .set_default("injection.allow_enigo", false)?
            .set_default("injection.inject_on_unknown_focus", true)?
            .set_default("injection.require_focus", false)?
            .set_default("injection.pause_hotkey", "")?
            .set_default("injection.redact_logs", true)?
            .set_default("injection.max_total_latency_ms", 800)?
            .set_default("injection.per_method_timeout_ms", 250)?
            .set_default("injection.paste_action_timeout_ms", 200)?
            .set_default("injection.cooldown_initial_ms", 10000)?
            .set_default("injection.cooldown_backoff_factor", 2.0)?
            .set_default("injection.cooldown_max_ms", 300000)?
            .set_default("injection.injection_mode", "auto")?
            .set_default("injection.keystroke_rate_cps", 20)?
            .set_default("injection.max_burst_chars", 50)?
            .set_default("injection.paste_chunk_chars", 500)?
            .set_default("injection.chunk_delay_ms", 30)?
            .set_default("injection.focus_cache_duration_ms", 200)?
            .set_default("injection.enable_window_detection", true)?
            .set_default("injection.clipboard_restore_delay_ms", 500)?
            .set_default("injection.discovery_timeout_ms", 1000)?
            .set_default("injection.allowlist", Vec::<String>::new())?
            .set_default("injection.blocklist", Vec::<String>::new())?
            .set_default("injection.min_success_rate", 0.3)?
            .set_default("injection.min_sample_size", 5)?
            // STT settings defaults
            .set_default("stt.preferred", Option::<String>::None)?
            .set_default("stt.fallbacks", Vec::<String>::new())?
            .set_default("stt.require_local", false)?
            .set_default("stt.max_mem_mb", Option::<u32>::None)?
            .set_default("stt.language", Option::<String>::None)?
            .set_default("stt.failover_threshold", 5)?
            .set_default("stt.failover_cooldown_secs", 10)?
            .set_default("stt.model_ttl_secs", 300)?
            .set_default("stt.disable_gc", false)?
            .set_default("stt.metrics_log_interval_secs", 30)?
            .set_default("stt.debug_dump_events", false)?
            .set_default("stt.auto_extract", true)?;

        // Check if a config file will be loaded
        let config_file_exists = if let Some(path) = &explicit_path {
            path.exists()
        } else {
            Self::discover_config_path().is_some()
        };

        if let Some(path) = explicit_path {
            if path.exists() {
                builder = builder.add_source(File::from(path));
            } else {
                return Err(ConfigError::Message(format!(
                    "Config file not found at {}",
                    path.display()
                )));
            }
        } else if let Some(path) = Self::discover_config_path() {
            builder = builder.add_source(File::from(path));
        }

        builder = builder.add_source(
            Environment::with_prefix("COLDVOX")
                .separator("__")
                .prefix_separator("_")
                .convert_case(Case::Snake)
                .try_parsing(true),
        );

        let config = builder.build()?;
        
        // Log a warning if no config file was found
        if !config_file_exists {
            tracing::warn!("No config file found, using default values only");
        }

        Ok(config)
    }

    fn discover_config_path() -> Option<PathBuf> {
        if let Ok(custom) = env::var("COLDVOX_CONFIG_PATH") {
            let path = PathBuf::from(custom);
            if path.exists() {
                return Some(path);
            }
        }

        if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
            let candidate = Path::new(&manifest_dir)
                .join("../..")
                .join("config/default.toml");
            if candidate.exists() {
                return Some(candidate);
            }
        }

        let cwd_candidate = PathBuf::from("config/default.toml");
        if cwd_candidate.exists() {
            return Some(cwd_candidate);
        }

        if let Ok(cwd) = env::current_dir() {
            for ancestor in cwd.ancestors() {
                let candidate = ancestor.join("config/default.toml");
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }

        if let Ok(xdg_home) = env::var("XDG_CONFIG_HOME") {
            let candidate = Path::new(&xdg_home).join("coldvox/default.toml");
            if candidate.exists() {
                return Some(candidate);
            }
        }

        if let Ok(home) = env::var("HOME") {
            let candidate = Path::new(&home).join(".config/coldvox/default.toml");
            if candidate.exists() {
                return Some(candidate);
            }
        }

        None
    }

    /// Load settings from a specific config file path (for tests)
    pub fn from_path(config_path: impl AsRef<Path>) -> Result<Self, String> {
        let config = Self::build_config(Some(config_path.as_ref().to_path_buf()))
            .map_err(|e| format!("Failed to build config: {}", e))?;

        let mut settings: Settings = config
            .try_deserialize()
            .map_err(|e| format!("Failed to deserialize settings: {}", e))?;

        settings.validate().map_err(|e| e.to_string())?;
        Ok(settings)
    }

    pub fn new() -> Result<Self, String> {
        let config = Self::build_config(None)
            .map_err(|e| format!("Failed to build config (likely invalid env vars): {}", e))?;

        let mut settings: Settings = config
            .try_deserialize()
            .map_err(|e| format!("Failed to deserialize settings from config: {}", e))?;

        settings.validate().map_err(|e| e.to_string())?;

        Ok(settings)
    }

    pub fn validate(&mut self) -> Result<(), String> {
        let mut errors = Vec::new();

        // Validate resampler_quality
        if !["fast", "balanced", "quality"]
            .contains(&self.resampler_quality.to_lowercase().as_str())
        {
            tracing::warn!(
                "Invalid resampler_quality '{}'. Defaulting to 'balanced'.",
                self.resampler_quality
            );
            self.resampler_quality = "balanced".to_string();
        }

        // Validate activation_mode
        if !["vad", "hotkey"].contains(&self.activation_mode.to_lowercase().as_str()) {
            tracing::warn!(
                "Invalid activation_mode '{}'. Defaulting to 'vad'.",
                self.activation_mode
            );
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
        if self.injection.cooldown_backoff_factor <= 0.0
            || self.injection.cooldown_backoff_factor > 10.0
        {
            tracing::warn!(
                "Invalid cooldown_backoff_factor {}. Clamping to 2.0.",
                self.injection.cooldown_backoff_factor
            );
            self.injection.cooldown_backoff_factor = 2.0;
        }
        if !["keystroke", "paste", "auto"]
            .contains(&self.injection.injection_mode.to_lowercase().as_str())
        {
            tracing::warn!(
                "Invalid injection_mode '{}'. Defaulting to 'auto'.",
                self.injection.injection_mode
            );
            self.injection.injection_mode = "auto".to_string();
        }
        if self.injection.keystroke_rate_cps == 0 || self.injection.keystroke_rate_cps > 100 {
            tracing::warn!(
                "Invalid keystroke_rate_cps {}. Clamping to 20.",
                self.injection.keystroke_rate_cps
            );
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
            tracing::warn!(
                "Invalid min_success_rate {}. Clamping to 0.3.",
                self.injection.min_success_rate
            );
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
