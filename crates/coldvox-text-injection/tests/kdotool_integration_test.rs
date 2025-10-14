// tests/kdotool_integration_test.rs

use coldvox_text_injection::kdotool_injector::{KdotoolInjector, WindowDetails};
use coldvox_text_injection::types::{InjectionConfig, InjectionError};
use serial_test::serial;
use std::env;

/// Helper to determine if we are in a headless CI environment
fn is_headless_ci() -> bool {
    env::var("CI").is_ok()
        && env::var("DISPLAY").is_err()
        && env::var("WAYLAND_DISPLAY").is_err()
}

/// Helper to check if kdotool is available on the system.
fn is_kdotool_available() -> bool {
    let output = std::process::Command::new("which")
        .arg("kdotool")
        .output();
    output.map(|o| o.status.success()).unwrap_or(false)
}

#[tokio::test]
#[serial]
#[cfg(feature = "kdotool")]
async fn test_get_active_window_details_mocked() {
    // This test doesn't actually run kdotool, but it sets up the structure
    // for a more complex mock test if we were to use a mocking framework
    // for `tokio::process::Command`. For now, we simulate the success path.

    // Simulate successful parsing of kdotool output
    let json_output = r#"{"id":"12345","pid":6789,"class":"org.kde.konsole"}"#;
    let details: WindowDetails = serde_json::from_str(json_output).unwrap();

    assert_eq!(
        details,
        WindowDetails {
            id: "12345".to_string(),
            pid: 6789,
            class: "org.kde.konsole".to_string(),
        }
    );
}

#[tokio::test]
#[serial]
#[cfg(feature = "kdotool")]
#[ignore] // This test requires a live KDE session and should be run manually.
async fn test_get_active_window_live() {
    if is_headless_ci() || !is_kdotool_available() {
        println!("Skipping live kdotool test: No display or kdotool not found.");
        return;
    }

    let config = InjectionConfig::default();
    let injector = KdotoolInjector::new(config);

    // This test assumes a window is open and active.
    // In a real CI environment, you would need to spawn a window.
    let result = injector.get_active_window().await;

    // We can't assert the exact window ID, but we can assert that we got one.
    assert!(
        result.is_ok(),
        "Expected to get an active window ID, but got error: {:?}",
        result.err()
    );
    let window_id = result.unwrap();
    assert!(
        !window_id.is_empty(),
        "Expected a non-empty window ID."
    );
    // Window IDs from kdotool are typically numeric.
    assert!(
        window_id.parse::<u64>().is_ok(),
        "Expected window ID to be a numeric value, but got: {}",
        window_id
    );
}

#[tokio::test]
#[serial]
#[cfg(feature = "kdotool")]
#[ignore] // This test requires a live KDE session and should be run manually.
async fn test_get_active_window_details_live() {
    if is_headless_ci() || !is_kdotool_available() {
        println!("Skipping live kdotool test: No display or kdotool not found.");
        return;
    }

    let config = InjectionConfig::default();
    let injector = KdotoolInjector::new(config);

    // This test assumes a window is open and active.
    let result = injector.get_active_window_details().await;

    assert!(
        result.is_ok(),
        "Expected to get active window details, but got error: {:?}",
        result.err()
    );

    let details = result.unwrap();
    assert!(!details.id.is_empty(), "Expected a non-empty window ID.");
    assert!(details.pid > 0, "Expected a valid PID.");
    assert!(!details.class.is_empty(), "Expected a non-empty window class.");
}

#[tokio::test]
#[serial]
#[cfg(feature = "kdotool")]
async fn test_ensure_focus_handles_no_window_id() {
    if is_headless_ci() || !is_kdotool_available() {
        println!("Skipping live kdotool test: No display or kdotool not found.");
        return;
    }

    let config = InjectionConfig::default();
    let injector = KdotoolInjector::new(config);

    // This test will fail if no window is active, which is expected.
    // The goal is to ensure the code path for `window_id: None` is exercised.
    let result = injector.ensure_focus(None).await;

    // In a headless environment, this will fail as no window is active.
    // In a live environment, it should succeed if a window is active.
    // We check for either success or a specific failure mode.
    if result.is_err() {
        let err = result.unwrap_err();
        match err {
            InjectionError::MethodFailed(msg) => {
                assert!(
                    msg.contains("kdotool getactivewindow failed") || msg.contains("Could not find details"),
                    "Expected a getactivewindow failure, but got: {}",
                    msg
                );
            }
            _ => panic!("Unexpected error type: {:?}", err),
        }
    }
}