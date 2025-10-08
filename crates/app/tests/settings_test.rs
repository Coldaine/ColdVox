use coldvox_app::Settings;
use serial_test::serial;
use std::env;
use std::path::PathBuf;

// This path discovery is based on the original plan and made more robust.
fn get_test_config_path() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    // Path from workspace root: <root>/crates/app -> <root>/config/default.toml
    // This is the primary expected path when running `cargo test --workspace`
    let path = manifest_dir.parent().unwrap().parent().unwrap().join("config/default.toml");

    if path.exists() {
        return path;
    }

    // A fallback that might work in other contexts, like running tests from the crate dir.
    let fallback_path = manifest_dir.join("../../config/default.toml");
    if fallback_path.exists() {
        return fallback_path;
    }

    panic!(
        "Could not find config/default.toml for tests. CWD: {}. Looked in {} and {}",
        env::current_dir().unwrap().display(),
        path.display(),
        fallback_path.canonicalize().unwrap_or(fallback_path).display() // Show absolute path on failure
    );
}

#[test]
#[serial]
fn test_settings_new_default() {
    let config_path = get_test_config_path();
    // Test default loading with a valid file path
    let settings = Settings::from_path(&config_path).unwrap();
    assert_eq!(settings.resampler_quality.to_lowercase(), "balanced");
    assert_eq!(settings.activation_mode.to_lowercase(), "vad");
    assert!(settings.injection.max_total_latency_ms > 0);
    assert!(settings.stt.failover_threshold > 0);
}

#[test]
#[serial]
fn test_settings_new_invalid_env_var_deserial() {
    let config_path = get_test_config_path();
    env::set_var("COLDVOX__INJECTION__MAX_TOTAL_LATENCY_MS", "abc"); // Invalid for u64
    let result = Settings::from_path(&config_path);
    assert!(result.is_err());
    // The error message from the config crate is about "invalid type", not "invalid digit".
    assert!(result.unwrap_err().contains("invalid type"));
    env::remove_var("COLDVOX__INJECTION__MAX_TOTAL_LATENCY_MS");
}

#[test]
fn test_settings_validate_zero_timeout() {
    // This test correctly uses default() because it expects validation to fail.
    let mut settings = Settings::default();
    settings.injection.max_total_latency_ms = 0;
    let result = settings.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("max_total_latency_ms"));
}

#[test]
#[serial]
fn test_settings_validate_invalid_mode() {
    // This test must load from a valid config, as default() is invalid.
    let config_path = get_test_config_path();
    let mut settings = Settings::from_path(&config_path).unwrap();
    settings.resampler_quality = "invalid".to_string();
    let result = settings.validate();
    assert!(result.is_ok()); // Warns but defaults applied
    assert_eq!(settings.resampler_quality, "balanced");
}

#[test]
#[serial]
fn test_settings_validate_invalid_rate() {
    // This test must load from a valid config, as default() is invalid.
    let config_path = get_test_config_path();
    let mut settings = Settings::from_path(&config_path).unwrap();
    settings.injection.keystroke_rate_cps = 200; // Too high
    let result = settings.validate();
    assert!(result.is_ok()); // Warns and clamps
    assert_eq!(settings.injection.keystroke_rate_cps, 20);
}

#[test]
#[serial]
fn test_settings_validate_success_rate() {
    // This test must load from a valid config, as default() is invalid.
    let config_path = get_test_config_path();
    let mut settings = Settings::from_path(&config_path).unwrap();
    settings.injection.min_success_rate = 1.5;
    let result = settings.validate();
    assert!(result.is_ok()); // Warns and clamps
    assert_eq!(settings.injection.min_success_rate, 0.3);
}

#[test]
fn test_settings_validate_zero_validation() {
    // This test correctly uses default() because it expects validation to fail.
    let mut settings = Settings::default();
    settings.stt.failover_threshold = 0;
    let result = settings.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("failover_threshold"));
}

#[test]
#[serial]
fn test_settings_new_with_env_override() {
    let config_path = get_test_config_path();
    // Correct the environment variable name for a top-level key.
    env::set_var("COLDVOX_ACTIVATION_MODE", "hotkey");
    let settings = Settings::from_path(&config_path).unwrap();
    assert_eq!(settings.activation_mode, "hotkey");
    env::remove_var("COLDVOX_ACTIVATION_MODE");
}

#[test]
#[serial]
fn test_settings_new_validation_err() {
    let config_path = get_test_config_path();
    env::set_var("COLDVOX__INJECTION__MAX_TOTAL_LATENCY_MS", "0");
    let result = Settings::from_path(&config_path);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("max_total_latency_ms"));
    env::remove_var("COLDVOX__INJECTION__MAX_TOTAL_LATENCY_MS");
}