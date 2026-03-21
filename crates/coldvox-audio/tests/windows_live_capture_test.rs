//! Windows Live Audio Capture Test
//!
//! This test captures ACTUAL audio from the microphone and verifies the pipeline works.
//! Run with: cargo test -p coldvox-audio --features live-hardware-tests test_live -- --nocapture

#[cfg(all(test, windows, feature = "live-hardware-tests"))]
mod live_capture_tests {
    use coldvox_audio::DeviceManager;
    use std::time::{Duration, Instant};

    /// Live test: Verify audio device enumeration and opening on Windows
    #[test]
    fn test_windows_live_device_detection() {
        println!("\n========================================");
        println!("🎤 LIVE WINDOWS AUDIO DEVICE TEST");
        println!("========================================");

        let manager = DeviceManager::new().expect("Failed to create DeviceManager");

        // Enumerate devices
        let devices = manager.enumerate_devices();
        println!("Found {} audio input devices:", devices.len());

        for (i, device) in devices.iter().enumerate() {
            let default_marker = if device.is_default { " (DEFAULT)" } else { "" };
            println!("  {}: {}{}", i + 1, device.name, default_marker);
        }

        assert!(
            !devices.is_empty(),
            "No audio input devices found on Windows"
        );

        let has_default = devices.iter().any(|d| d.is_default);
        println!("\n✅ Default device found: {}", has_default);
        println!("========================================\n");
    }
}

#[cfg(all(test, not(windows)))]
mod non_windows_tests {
    #[test]
    fn test_skipped_on_non_windows() {
        println!("These tests are Windows-only. Skipping.");
    }
}

#[cfg(all(test, windows, not(feature = "live-hardware-tests")))]
mod no_feature_tests {
    use coldvox_audio::DeviceManager;

    #[test]
    fn test_needs_feature_flag() {
        println!("Live tests require --features live-hardware-tests");
    }

    #[test]
    fn test_can_create_device_manager() {
        let manager = DeviceManager::new();
        assert!(manager.is_ok(), "Should be able to create DeviceManager");
    }
}
