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

/// Comprehensive test of settings loading from defaults and config files.
///
/// Tests the complete config loading behavior:
/// - Loading pure defaults without config file
/// - Loading from config file with expected values
/// - Config discovery and file resolution
#[test]
fn test_settings_loading() {
    // Test 1: Pure defaults (no config file discovery)
    std::env::set_var("COLDVOX_SKIP_CONFIG_DISCOVERY", "1");
    let default_settings = Settings::new().unwrap();
    assert_eq!(default_settings.resampler_quality.to_lowercase(), "balanced",
        "Default resampler quality should be balanced");
    assert_eq!(default_settings.activation_mode.to_lowercase(), "vad",
        "Default activation mode should be VAD");
    assert_eq!(default_settings.injection.max_total_latency_ms, 800,
        "Default max latency should be 800ms");
    assert!(default_settings.stt.failover_threshold > 0,
        "Default failover threshold should be positive");
    std::env::remove_var("COLDVOX_SKIP_CONFIG_DISCOVERY");

    // Test 2: Load from config file
    let config_path = get_test_config_path();
    let file_settings = Settings::from_path(&config_path)
        .expect("Should successfully load config from file");
    // Verify some config values loaded correctly
    assert!(!file_settings.resampler_quality.is_empty(),
        "Config file should specify resampler quality");
    assert!(!file_settings.activation_mode.is_empty(),
        "Config file should specify activation mode");
}

/// Comprehensive test of settings validation logic.
///
/// Tests all validation rules:
/// - Zero value detection (invalid timeouts, thresholds)
/// - Invalid enum values (with auto-correction)
/// - Range clamping (keystroke rate, success rate)
/// - Validation error messages
#[test]
fn test_settings_validation() {
    let config_path = get_test_config_path();

    // Test 1: Zero timeout should fail
    let mut settings = Settings::default();
    settings.injection.max_total_latency_ms = 0;
    let result = settings.validate();
    assert!(result.is_err(), "Zero timeout should fail validation");
    assert!(result.unwrap_err().contains("max_total_latency_ms"),
        "Error should mention the invalid field");

    // Test 2: Zero failover threshold should fail
    let mut settings = Settings::default();
    settings.stt.failover_threshold = 0;
    let result = settings.validate();
    assert!(result.is_err(), "Zero failover threshold should fail validation");
    assert!(result.unwrap_err().contains("failover_threshold"),
        "Error should mention the invalid field");

    // Test 3: Invalid resampler quality should auto-correct
    let mut settings = Settings::from_path(&config_path).expect("Failed to load config");
    settings.resampler_quality = "invalid".to_string();
    let result = settings.validate();
    assert!(result.is_ok(), "Invalid quality should warn but not error");
    assert_eq!(settings.resampler_quality, "balanced",
        "Invalid quality should default to balanced");

    // Test 4: Out-of-range keystroke rate should clamp
    let mut settings = Settings::from_path(&config_path).expect("Failed to load config");
    settings.injection.keystroke_rate_cps = 200; // Too high
    let result = settings.validate();
    assert!(result.is_ok(), "Out-of-range should warn but not error");
    assert_eq!(settings.injection.keystroke_rate_cps, 20,
        "Keystroke rate should be clamped to maximum");

    // Test 5: Invalid success rate should clamp
    let mut settings = Settings::from_path(&config_path).expect("Failed to load config");
    settings.injection.min_success_rate = 1.5; // Too high
    let result = settings.validate();
    assert!(result.is_ok(), "Out-of-range should warn but not error");
    assert_eq!(settings.injection.min_success_rate, 0.3,
        "Success rate should be clamped to valid range");
}

/// Test environment variable overrides and error handling.
///
/// Tests the complete environment variable override system:
/// - Valid overrides apply correctly
/// - Invalid type conversions fail gracefully
/// - Validation still applies to overridden values
#[test]
#[ignore]
fn test_settings_environment_overrides() {
    skip_test_unless!(coldvox_foundation::test_env::TestRequirements::new()
        .requires_env_var("COLDVOX_TEST_ENV_OVERRIDE"));

    // Test 1: Valid environment override
    env::set_var("COLDVOX_SKIP_CONFIG_DISCOVERY", "1");
    env::set_var("COLDVOX__ACTIVATION_MODE", "hotkey");
    let settings = Settings::new().unwrap();
    assert_eq!(settings.activation_mode, "hotkey",
        "Environment variable should override activation mode");
    env::remove_var("COLDVOX__ACTIVATION_MODE");
    env::remove_var("COLDVOX_SKIP_CONFIG_DISCOVERY");

    // Test 2: Invalid type conversion should fail
    env::set_var("COLDVOX_SKIP_CONFIG_DISCOVERY", "1");
    env::set_var("COLDVOX__INJECTION__MAX_TOTAL_LATENCY_MS", "abc"); // Invalid for u64
    let result = Settings::new();
    assert!(result.is_err(), "Invalid type should fail deserialization");
    let err = result.unwrap_err();
    assert!(
        err.contains("invalid digit found in string") || err.contains("deserialize"),
        "Error should indicate deserialization failure, got: {}",
        err
    );
    env::remove_var("COLDVOX__INJECTION__MAX_TOTAL_LATENCY_MS");
    env::remove_var("COLDVOX_SKIP_CONFIG_DISCOVERY");

    // Test 3: Environment override that fails validation
    env::set_var("COLDVOX_SKIP_CONFIG_DISCOVERY", "1");
    env::set_var("COLDVOX__INJECTION__MAX_TOTAL_LATENCY_MS", "0"); // Invalid: zero
    let result = Settings::new();
    assert!(result.is_err(), "Zero timeout should fail validation even from env var");
    assert!(result.unwrap_err().contains("max_total_latency_ms"),
        "Error should mention the invalid field");
    env::remove_var("COLDVOX__INJECTION__MAX_TOTAL_LATENCY_MS");
    env::remove_var("COLDVOX_SKIP_CONFIG_DISCOVERY");
}
