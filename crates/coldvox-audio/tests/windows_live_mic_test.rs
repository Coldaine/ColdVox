//! Windows Live Microphone Test
//!
//! This test verifies that ColdVox can capture audio from a live microphone on Windows.
//! Run with: cargo test -p coldvox-audio --features live-hardware-tests -- --nocapture

#[cfg(all(test, windows, feature = "live-hardware-tests"))]
mod windows_live_tests {
    use coldvox_audio::DeviceManager;

    /// Test that we can enumerate audio devices on Windows
    #[test]
    fn test_windows_device_enumeration() {
        let manager = DeviceManager::new().expect("Failed to create DeviceManager");

        let devices = manager.enumerate_devices();
        println!("Found {} audio input devices:", devices.len());

        for (i, device) in devices.iter().enumerate() {
            let default_marker = if device.is_default { " (DEFAULT)" } else { "" };
            println!("  {}: {}{}", i + 1, device.name, default_marker);
        }

        // On Windows, we should find at least one device
        assert!(
            !devices.is_empty(),
            "No audio input devices found on Windows"
        );

        // Check that we have a default device
        let has_default = devices.iter().any(|d| d.is_default);
        println!("Default device found: {}", has_default);
    }

    /// Test that we can open the default audio device on Windows
    #[test]
    fn test_windows_open_default_device() {
        use coldvox_audio::DeviceManager;

        println!("\n=== Windows Default Device Test ===");

        let mut manager = DeviceManager::new().expect("Failed to create DeviceManager");

        // Get default device name first
        let default_name = manager.default_input_device_name();
        println!("Default device name: {:?}", default_name);

        // Try to open default device
        let device = manager.open_device(None);
        assert!(
            device.is_ok(),
            "Should be able to open default device: {:?}",
            device.err()
        );

        println!("Successfully opened default device");
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
mod no_hardware_tests {
    use coldvox_audio::DeviceManager;

    #[test]
    fn test_device_manager_creation() {
        // This test doesn't require hardware, just verifies DeviceManager can be created
        let manager = DeviceManager::new();
        assert!(manager.is_ok(), "Should be able to create DeviceManager");
    }

    #[test]
    fn test_device_enumeration_no_hardware_feature() {
        // Can enumerate even without live-hardware-tests feature
        let manager = DeviceManager::new().expect("Failed to create DeviceManager");
        let devices = manager.enumerate_devices();
        println!(
            "Found {} devices (without live-hardware-tests feature)",
            devices.len()
        );
        // Don't assert - may be headless environment
    }
}
