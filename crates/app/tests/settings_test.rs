use coldvox_app::Settings;
use coldvox_foundation::skip_test_unless;
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
    // Ensure we test pure defaults without loading repository config files
    std::env::set_var("COLDVOX_SKIP_CONFIG_DISCOVERY", "1");
    // Test default loading without file - Settings::new() will use defaults if no config found
    let settings = Settings::new().unwrap();
    assert_eq!(settings.resampler_quality.to_lowercase(), "balanced");
    assert_eq!(settings.activation_mode.to_lowercase(), "vad");
    assert_eq!(settings.injection.max_total_latency_ms, 800);
    assert!(settings.stt.failover_threshold > 0);
    std::env::remove_var("COLDVOX_SKIP_CONFIG_DISCOVERY");
}

#[test]
#[ignore]
fn test_settings_new_invalid_env_var_deserial() {
    skip_test_unless!(coldvox_foundation::test_env::TestRequirements::new()
        .requires_env_var("COLDVOX_TEST_ENV_OVERRIDE"));
    // Avoid reading files; exercise env-only path
    env::set_var("COLDVOX_SKIP_CONFIG_DISCOVERY", "1");
    // New environment parsing uses double-underscore segment separator
    env::set_var("COLDVOX__INJECTION__MAX_TOTAL_LATENCY_MS", "abc"); // Invalid for u64
    let result = Settings::new();
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("invalid digit found in string") || err.contains("deserialize"),
        "unexpected error message: {}",
        err
    );
    env::remove_var("COLDVOX__INJECTION__MAX_TOTAL_LATENCY_MS");
    env::remove_var("COLDVOX_SKIP_CONFIG_DISCOVERY");
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
#[ignore]
fn test_settings_new_with_env_override() {
    skip_test_unless!(coldvox_foundation::test_env::TestRequirements::new()
        .requires_env_var("COLDVOX_TEST_ENV_OVERRIDE"));
    env::set_var("COLDVOX_SKIP_CONFIG_DISCOVERY", "1");
    // New environment parsing uses double-underscore segment separator
    env::set_var("COLDVOX__ACTIVATION_MODE", "hotkey");
    let settings = Settings::new().unwrap();
    assert_eq!(settings.activation_mode, "hotkey");
    env::remove_var("COLDVOX__ACTIVATION_MODE");
    env::remove_var("COLDVOX_SKIP_CONFIG_DISCOVERY");
}

#[test]
#[ignore]
fn test_settings_new_validation_err() {
    skip_test_unless!(coldvox_foundation::test_env::TestRequirements::new()
        .requires_env_var("COLDVOX_TEST_ENV_OVERRIDE"));
    env::set_var("COLDVOX_SKIP_CONFIG_DISCOVERY", "1");
    // New environment parsing uses double-underscore segment separator
    env::set_var("COLDVOX__INJECTION__MAX_TOTAL_LATENCY_MS", "0");
    let result = Settings::new();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("max_total_latency_ms"));
    env::remove_var("COLDVOX__INJECTION__MAX_TOTAL_LATENCY_MS");
    env::remove_var("COLDVOX_SKIP_CONFIG_DISCOVERY");
}
