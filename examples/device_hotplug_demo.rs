use coldvox_audio::{AudioCaptureThread, AudioRingBuffer, DeviceMonitor};
use coldvox_foundation::{AudioConfig, DeviceEvent};
use std::time::Duration;

/// Example demonstrating audio device hotplug support
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    println!("Audio Device Hotplug Demo");
    println!("========================");

    // Create audio ring buffer
    let buffer = AudioRingBuffer::new(8192);
    let (producer, _consumer) = buffer.split();

    // Create audio configuration
    let config = AudioConfig::default();

    // Start audio capture with device monitoring
    let result = AudioCaptureThread::spawn(config, producer, None);

    match result {
        Ok((capture_thread, device_config, _config_rx, mut device_event_rx)) => {
            println!("Audio capture started successfully!");
            println!("Device config: {:?}", device_config);

            // Monitor device events for a few seconds
            println!("\nMonitoring device events (press Ctrl+C to exit)...");

            let start_time = std::time::Instant::now();
            while start_time.elapsed() < Duration::from_secs(30) {
                match device_event_rx.try_recv() {
                    Ok(event) => match event {
                        DeviceEvent::DeviceAdded { name } => {
                            println!("✅ Device added: {}", name);
                        }
                        DeviceEvent::DeviceRemoved { name } => {
                            println!("❌ Device removed: {}", name);
                        }
                        DeviceEvent::CurrentDeviceDisconnected { name } => {
                            println!("🔌 Current device disconnected: {}", name);
                        }
                        DeviceEvent::DeviceSwitched { from, to } => {
                            println!("🔄 Device switched from {:?} to {}", from, to);
                        }
                        DeviceEvent::DeviceSwitchFailed {
                            attempted,
                            fallback,
                        } => {
                            println!(
                                "❌ Device switch failed: attempted={}, fallback={:?}",
                                attempted, fallback
                            );
                        }
                        DeviceEvent::DeviceSwitchRequested { target } => {
                            println!("📝 Device switch requested to: {}", target);
                        }
                    },
                    Err(tokio::sync::broadcast::error::TryRecvError::Empty) => {
                        // No events, wait a bit
                        std::thread::sleep(Duration::from_millis(100));
                    }
                    Err(e) => {
                        eprintln!("Error receiving device events: {:?}", e);
                        break;
                    }
                }
            }

            println!("\nDemo completed. Shutting down...");
            capture_thread.stop();
        }
        Err(e) => {
            eprintln!("Failed to start audio capture: {}", e);
            eprintln!("This is expected in CI environments without audio hardware.");

            // Demonstrate device monitoring separately
            println!("\nTesting device monitor separately...");
            let (monitor, mut event_rx) = DeviceMonitor::new(Duration::from_millis(500))?;

            println!(
                "Available device candidates: {:?}",
                monitor.get_device_candidates()
            );

            // Test device monitor for a short time
            let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
            let monitor_handle = monitor.start_monitoring(running.clone());

            std::thread::sleep(Duration::from_secs(2));

            // Check for any events
            match event_rx.try_recv() {
                Ok(event) => println!("Device monitor event: {:?}", event),
                Err(_) => println!("No device events detected in test period"),
            }

            running.store(false, std::sync::atomic::Ordering::Relaxed);
            let _ = monitor_handle.join();
        }
    }

    Ok(())
}
