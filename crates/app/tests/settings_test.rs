use coldvox_app::Settings;
use std::env;
use std::fs;
use std::path::Path;

fn setup_test_config() {
    let config_dir = Path::new("config");
    if !config_dir.exists() {
        fs::create_dir_all(config_dir).unwrap();
    }
    if !Path::new("config/default.toml").exists() {
        fs::copy("../config/default.toml", "config/default.toml")
            .or_else(|_| fs::copy("../../config/default.toml", "config/default.toml"))
            .expect("Failed to copy config for tests");
    }
}

#[test]
fn test_settings_new_default() {
    setup_test_config();
    // Test default loading without file
    let settings = Settings::new().unwrap();
    assert_eq!(settings.resampler_quality.to_lowercase(), "balanced");
    assert_eq!(settings.activation_mode.to_lowercase(), "vad");
    assert!(settings.injection.max_total_latency_ms > 0);
    assert!(settings.stt.failover_threshold > 0);
}

#[test]
fn test_settings_new_invalid_env_var_deserial() {
    setup_test_config();
    env::set_var("COLDVOX_INJECTION__MAX_TOTAL_LATENCY_MS", "abc");  // Invalid for u64
    let result = Settings::new();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("deserialize"));
    env::remove_var("COLDVOX_INJECTION__MAX_TOTAL_LATENCY_MS");
}

#[test]
fn test_settings_validate_zero_timeout() {
    let mut settings = Settings::default();
    settings.injection.max_total_latency_ms = 0;
    let result = settings.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("max_total_latency_ms"));
}

#[test]
fn test_settings_validate_invalid_mode() {
    let mut settings = Settings::default();
    settings.resampler_quality = "invalid".to_string();
    let result = settings.validate();
    assert!(result.is_ok());  // Warns but defaults applied
    assert_eq!(settings.resampler_quality, "balanced");
}

#[test]
fn test_settings_validate_invalid_rate() {
    let mut settings = Settings::default();
    settings.injection.keystroke_rate_cps = 200;  // Too high
    let result = settings.validate();
    assert!(result.is_ok());  // Warns and clamps
    assert_eq!(settings.injection.keystroke_rate_cps, 20);
}

#[test]
fn test_settings_validate_success_rate() {
    let mut settings = Settings::default();
    settings.injection.min_success_rate = 1.5;
    let result = settings.validate();
    assert!(result.is_ok());  // Warns and clamps
    assert_eq!(settings.injection.min_success_rate, 0.3);
}

#[test]
fn test_settings_validate_zero_validation() {
    let mut settings = Settings::default();
    settings.stt.failover_threshold = 0;
    let result = settings.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("failover_threshold"));
}

#[test]
fn test_settings_new_with_env_override() {
    setup_test_config();
    env::set_var("COLDVOX_ACTIVATION_MODE", "hotkey");
    let settings = Settings::new().unwrap();
    assert_eq!(settings.activation_mode, "hotkey");
    env::remove_var("COLDVOX_ACTIVATION_MODE");
}

#[test]
fn test_settings_new_validation_err() {
    setup_test_config();
    env::set_var("COLDVOX_INJECTION__MAX_TOTAL_LATENCY_MS", "0");
    let result = Settings::new();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("max_total_latency_ms"));
    env::remove_var("COLDVOX_INJECTION__MAX_TOTAL_LATENCY_MS");
}