//! Hardware capability tests for the self-hosted runner.
//!
//! These tests verify that the runner environment has access to the necessary
//! hardware resources (audio input, display server, etc.) to run the full pipeline.
//! They are not "Golden Master" tests because they are non-deterministic.

mod common;

#[cfg(test)]
mod hardware_tests {
    use crate::common::logging::init_test_logging;
    use coldvox_foundation::AudioConfig;

    fn should_skip(env_var: &str) -> bool {
        match std::env::var(env_var) {
            Ok(v) => matches!(
                v.to_ascii_lowercase().as_str(),
                "0" | "false" | "off" | "no"
            ),
            Err(_) => false, // Default to running (opt-out only)
        }
    }

    /// Verifies that we can open the default audio input device.
    #[test]
    #[ignore = "Requires real audio hardware"]
    fn test_audio_capture_device_open() {
        let _guard = init_test_logging("hardware_check_audio");

        // Skip only if explicitly opted out
        if should_skip("COLDVOX_E2E_REAL_AUDIO") {
            println!("Skipping audio hardware test: COLDVOX_E2E_REAL_AUDIO set to false/0");
            return;
        }

        println!("Attempting to open default audio capture device...");

        // We just want to see if it panics or errors out immediately.
        let config = AudioConfig {
            silence_threshold: 100,
            capture_buffer_samples: 1024,
        };

        // This is a bit tricky because AudioCapture might not expose a simple "check" method
        // without starting the stream. We'll try to instantiate the ring buffer and capture.
        let ring_buffer = coldvox_audio::AudioRingBuffer::new(config.capture_buffer_samples);
        let (producer, _consumer) = ring_buffer.split();
        let _producer = std::sync::Arc::new(std::sync::Mutex::new(producer));

        // Use cpal directly to verify hardware access
        use cpal::traits::{DeviceTrait, HostTrait};
        let host = cpal::default_host();
        let device = host.default_input_device();

        assert!(device.is_some(), "No default input device found!");
        let device = device.unwrap();
        println!(
            "Found default input device: {}",
            device.name().unwrap_or_default()
        );
    }

    /// Verifies that the text injection subsystem is available.
    #[tokio::test]
    #[ignore = "Requires display server"]
    async fn test_text_injector_availability() {
        let _guard = init_test_logging("hardware_check_injection");

        if should_skip("COLDVOX_E2E_REAL_INJECTION") {
            println!("Skipping injection hardware test: COLDVOX_E2E_REAL_INJECTION set to false/0");
            return;
        }

        // Check for display server
        let has_display =
            std::env::var("DISPLAY").is_ok() || std::env::var("WAYLAND_DISPLAY").is_ok();
        if !has_display {
            panic!("No display server detected (DISPLAY or WAYLAND_DISPLAY missing).");
        }

        println!("Display server detected.");

        // We can't easily access the internal test harness from here,
        // but verifying the environment variables is a good first step.
        // The actual injection test is covered by `coldvox-text-injection` crate tests.
    }
}
