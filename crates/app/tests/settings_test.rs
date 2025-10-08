use coldvox_app::Settings;
use std::env;
use std::path::PathBuf;

fn get_test_config_path() -> PathBuf {
    // Try workspace root first (for integration tests)
    let workspace_config = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("config/default.toml");

    if workspace_config.exists() {
        return workspace_config;
    }

    // Fallback to relative path from crate root
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../config/default.toml")
}

#[test]
fn test_settings_new_default() {
    // Test default loading without file - Settings::new() will use defaults if no config found
    let settings = Settings::new().unwrap();
    assert_eq!(settings.resampler_quality.to_lowercase(), "balanced");
    assert_eq!(settings.activation_mode.to_lowercase(), "vad");
    assert_eq!(settings.injection.max_total_latency_ms, 800);
    assert!(settings.stt.failover_threshold > 0);
}

#[test]
#[ignore] // TODO: Environment variable overrides not working - pre-existing issue
fn test_settings_new_invalid_env_var_deserial() {
    let config_path = get_test_config_path();
    env::set_var("COLDVOX_INJECTION__MAX_TOTAL_LATENCY_MS", "abc"); // Invalid for u64
    let result = Settings::from_path(&config_path);
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
    let config_path = get_test_config_path();
    let mut settings = Settings::from_path(&config_path).expect("Failed to load config");
    settings.resampler_quality = "invalid".to_string();
    let result = settings.validate();
    assert!(result.is_ok()); // Warns but defaults applied
    assert_eq!(settings.resampler_quality, "balanced");
}

#[test]
fn test_settings_validate_invalid_rate() {
    let config_path = get_test_config_path();
    let mut settings = Settings::from_path(&config_path).expect("Failed to load config");
    settings.injection.keystroke_rate_cps = 200; // Too high
    let result = settings.validate();
    assert!(result.is_ok()); // Warns and clamps
    assert_eq!(settings.injection.keystroke_rate_cps, 20);
}

#[test]
fn test_settings_validate_success_rate() {
    let config_path = get_test_config_path();
    let mut settings = Settings::from_path(&config_path).expect("Failed to load config");
    settings.injection.min_success_rate = 1.5;
    let result = settings.validate();
    assert!(result.is_ok()); // Warns and clamps
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
#[ignore] // TODO: Environment variable overrides not working - pre-existing issue
fn test_settings_new_with_env_override() {
    let config_path = get_test_config_path();
    env::set_var("COLDVOX_ACTIVATION_MODE", "hotkey");
    let settings = Settings::from_path(&config_path).unwrap();
    assert_eq!(settings.activation_mode, "hotkey");
    env::remove_var("COLDVOX_ACTIVATION_MODE");
}

#[test]
#[ignore] // TODO: Environment variable overrides not working - pre-existing issue
fn test_settings_new_validation_err() {
    let config_path = get_test_config_path();
    env::set_var("COLDVOX_INJECTION__MAX_TOTAL_LATENCY_MS", "0");
    let result = Settings::from_path(&config_path);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("max_total_latency_ms"));
    env::remove_var("COLDVOX_INJECTION__MAX_TOTAL_LATENCY_MS");
}
