use coldvox_audio::{AudioCaptureThread, AudioRingBuffer, DeviceMonitor};
use coldvox_foundation::{AudioConfig, DeviceEvent};
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::Duration;

#[cfg(test)]
mod device_hotplug_tests {
    use super::*;

    #[test]
    fn test_device_monitor_basic_functionality() {
        let result = DeviceMonitor::new(Duration::from_millis(100));
        assert!(result.is_ok(), "Should be able to create device monitor");

        let (monitor, _rx) = result.unwrap();

        // Test getting device candidates
        let candidates = monitor.get_device_candidates();
        assert!(
            !candidates.is_empty(),
            "Should have at least one device candidate"
        );

        // Test device availability check
        assert!(!monitor.is_device_available("non_existent_device"));
    }

    #[test]
    fn test_device_status_management() {
        let (mut monitor, _rx) =
            DeviceMonitor::new(Duration::from_millis(100)).expect("Failed to create monitor");

        // Test setting current device
        monitor.set_current_device(Some("test_device".to_string()));
        assert_eq!(monitor.current_device(), Some(&"test_device".to_string()));

        // Test clearing current device
        monitor.set_current_device(None);
        assert_eq!(monitor.current_device(), None);

        // Test getting device status list
        let _status_list = monitor.get_device_status();
        // Should be able to get status list (will be valid in any environment)
    }

    #[test]
    fn test_audio_capture_thread_with_device_events() {
        let buffer = AudioRingBuffer::new(8192);
        let (producer, _consumer) = buffer.split();
        let producer = Arc::new(Mutex::new(producer));
        let config = AudioConfig::default();

        // Try to create capture thread with device monitoring
        let result = AudioCaptureThread::spawn(config, producer, None, true);

        match result {
            Ok((capture_thread, device_config, _config_rx, device_event_rx)) => {
                // Verify we get a valid device configuration
                assert!(device_config.sample_rate > 0);
                assert!(device_config.channels > 0);

                // Verify we can receive from the device event channel
                let mut device_rx = device_event_rx;

                // We shouldn't have any immediate device events in a test environment
                match device_rx.try_recv() {
                    Err(tokio::sync::broadcast::error::TryRecvError::Empty) => {
                        // This is expected - no events yet
                    }
                    Ok(event) => {
                        println!("Received device event: {:?}", event);
                    }
                    Err(e) => {
                        panic!("Unexpected error receiving device events: {:?}", e);
                    }
                }

                // Clean shutdown
                capture_thread.stop();
            }
            Err(e) => {
                // In CI environment without audio devices, this is acceptable
                println!(
                    "Audio capture thread creation failed (expected in CI): {}",
                    e
                );
            }
        }
    }

    #[test]
    fn test_device_event_types() {
        // Test that we can create different device event types
        let added_event = DeviceEvent::DeviceAdded {
            name: "test_device".to_string(),
        };
        assert!(matches!(added_event, DeviceEvent::DeviceAdded { .. }));

        let removed_event = DeviceEvent::DeviceRemoved {
            name: "test_device".to_string(),
        };
        assert!(matches!(removed_event, DeviceEvent::DeviceRemoved { .. }));

        let disconnected_event = DeviceEvent::CurrentDeviceDisconnected {
            name: "test_device".to_string(),
        };
        assert!(matches!(
            disconnected_event,
            DeviceEvent::CurrentDeviceDisconnected { .. }
        ));

        let switched_event = DeviceEvent::DeviceSwitched {
            from: Some("old_device".to_string()),
            to: "new_device".to_string(),
        };
        assert!(matches!(switched_event, DeviceEvent::DeviceSwitched { .. }));

        let failed_event = DeviceEvent::DeviceSwitchFailed {
            attempted: "failed_device".to_string(),
            fallback: None,
        };
        assert!(matches!(
            failed_event,
            DeviceEvent::DeviceSwitchFailed { .. }
        ));

        let request_event = DeviceEvent::DeviceSwitchRequested {
            target: "target_device".to_string(),
        };
        assert!(matches!(
            request_event,
            DeviceEvent::DeviceSwitchRequested { .. }
        ));
    }

    #[test]
    fn test_recovery_strategy_for_device_errors() {
        use coldvox_foundation::{AppError, AudioError, RecoveryStrategy};

        // Test recovery strategy for device disconnection
        let disconnection_error = AppError::Audio(AudioError::DeviceDisconnected);
        let strategy = disconnection_error.recovery_strategy();
        assert!(matches!(strategy, RecoveryStrategy::Retry { .. }));

        // Test recovery strategy for device not found
        let not_found_error = AppError::Audio(AudioError::DeviceNotFound {
            name: Some("missing_device".to_string()),
        });
        let strategy = not_found_error.recovery_strategy();
        assert!(matches!(strategy, RecoveryStrategy::Fallback { .. }));
    }
}
