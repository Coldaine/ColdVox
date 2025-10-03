use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

use super::device::DeviceManager;
use coldvox_foundation::{AudioError, DeviceEvent, DeviceStatus};

/// Monitor for audio device changes and hotplug events
pub struct DeviceMonitor {
    device_manager: DeviceManager,
    event_tx: broadcast::Sender<DeviceEvent>,
    monitor_interval: Duration,
    last_devices: HashMap<String, DeviceStatus>,
    current_device: Option<String>,
    preferred_devices: Vec<String>,
    // Track how many consecutive scans a device has been missing
    missing_count: HashMap<String, u32>,
}

impl DeviceMonitor {
    pub fn new(
        monitor_interval: Duration,
    ) -> Result<(Self, broadcast::Receiver<DeviceEvent>), AudioError> {
        let device_manager = DeviceManager::new()?;
        let (event_tx, event_rx) = broadcast::channel(32);

        let monitor = Self {
            device_manager,
            event_tx,
            monitor_interval,
            last_devices: HashMap::new(),
            current_device: None,
            preferred_devices: Vec::new(),
            missing_count: HashMap::new(),
        };

        Ok((monitor, event_rx))
    }

    /// Set preferred device order for automatic switching
    pub fn set_preferred_devices(&mut self, devices: Vec<String>) {
        self.preferred_devices = devices;
        info!(
            "Updated preferred device list: {:?}",
            self.preferred_devices
        );
    }

    /// Start monitoring in a background thread
    pub fn start_monitoring(mut self, running: Arc<AtomicBool>) -> JoinHandle<()> {
        thread::Builder::new()
            .name("device-monitor".to_string())
            .spawn(move || {
                info!("Device monitor started");

                // Initialize with current device list
                if let Err(e) = self.scan_and_update_devices() {
                    warn!("Initial device scan failed: {}", e);
                }

                while running.load(Ordering::Relaxed) {
                    if let Err(e) = self.scan_and_update_devices() {
                        warn!("Device scan failed: {}", e);
                    }

                    thread::sleep(self.monitor_interval);
                }

                info!("Device monitor stopping");
            })
            .expect("Failed to spawn device monitor thread")
    }

    /// Set the currently active device for monitoring
    pub fn set_current_device(&mut self, device_name: Option<String>) {
        let old_current = self.current_device.clone();
        self.current_device = device_name.clone();

        // Update current device status
        if let Some(old_name) = old_current {
            if let Some(status) = self.last_devices.get_mut(&old_name) {
                status.is_current = false;
            }
        }

        if let Some(new_name) = device_name {
            if let Some(status) = self.last_devices.get_mut(&new_name) {
                status.is_current = true;
            }
        }
    }

    fn scan_and_update_devices(&mut self) -> Result<(), AudioError> {
        let current_devices = self.device_manager.enumerate_devices();
        let now = Instant::now();
        let mut new_device_map = HashMap::new();

        // Process current devices
        for device_info in current_devices {
            let name = device_info.name.clone();
            let is_current = self.current_device.as_ref() == Some(&name);

            let status = DeviceStatus {
                name: name.clone(),
                is_current,
                is_available: true,
                is_default: device_info.is_default,
                last_seen: now,
            };

            // Check if this is a new device
            if !self.last_devices.contains_key(&name) {
                debug!("New device detected: {}", name);
                let _ = self
                    .event_tx
                    .send(DeviceEvent::DeviceAdded { name: name.clone() });
            }

            new_device_map.insert(name, status);
        }

        // Check for removed devices (with debouncing to prevent false positives)
        const REMOVAL_THRESHOLD: u32 = 3; // Device must be missing for 3 consecutive scans
        
        for (old_name, old_status) in &self.last_devices {
            if !new_device_map.contains_key(old_name) {
                // Increment missing count
                let count = self.missing_count.entry(old_name.clone()).or_insert(0);
                *count += 1;
                
                debug!("Device '{}' not seen in scan (missing {} times)", old_name, count);
                
                // Only emit removal events after threshold
                if *count >= REMOVAL_THRESHOLD {
                    warn!("Device removed after {} consecutive absences: {}", REMOVAL_THRESHOLD, old_name);
                    let _ = self.event_tx.send(DeviceEvent::DeviceRemoved {
                        name: old_name.clone(),
                    });

                    // If current device was removed, signal disconnection
                    if old_status.is_current {
                        warn!("Current device disconnected: {}", old_name);
                        let _ = self.event_tx.send(DeviceEvent::CurrentDeviceDisconnected {
                            name: old_name.clone(),
                        });
                    }
                    
                    // Clear from missing count after emitting removal
                    self.missing_count.remove(old_name);
                }
            } else {
                // Device reappeared, reset missing count
                self.missing_count.remove(old_name);
            }
        }

        self.last_devices = new_device_map;
        Ok(())
    }

    /// Get current device status list
    pub fn get_device_status(&self) -> Vec<DeviceStatus> {
        self.last_devices.values().cloned().collect()
    }

    /// Check if a specific device is available
    pub fn is_device_available(&self, device_name: &str) -> bool {
        self.last_devices
            .get(device_name)
            .map(|status| status.is_available)
            .unwrap_or(false)
    }

    /// Get the current device name
    pub fn current_device(&self) -> Option<&String> {
        self.current_device.as_ref()
    }

    /// Request a manual device switch
    pub fn request_device_switch(&self, target_device: String) -> Result<(), AudioError> {
        if !self.is_device_available(&target_device) {
            return Err(AudioError::DeviceNotFound {
                name: Some(target_device),
            });
        }

        info!("Requesting manual switch to device: {}", target_device);
        let _ = self.event_tx.send(DeviceEvent::DeviceSwitchRequested {
            target: target_device,
        });

        Ok(())
    }

    /// Get preferred device candidates, considering user preferences
    pub fn get_preferred_candidates(&self) -> Vec<String> {
        let mut candidates = Vec::new();

        // First, add user-preferred devices that are available
        for device in &self.preferred_devices {
            if self.is_device_available(device) {
                candidates.push(device.clone());
            }
        }

        // Then add system candidates not already in the list
        let system_candidates = self.device_manager.candidate_device_names();
        for device in system_candidates {
            if !candidates.contains(&device) {
                candidates.push(device);
            }
        }

        candidates
    }

    /// Get the default device candidate priority list (fallback when no preferences set)
    pub fn get_device_candidates(&self) -> Vec<String> {
        if self.preferred_devices.is_empty() {
            self.device_manager.candidate_device_names()
        } else {
            self.get_preferred_candidates()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_device_monitor_creation() {
        let result = DeviceMonitor::new(Duration::from_millis(100));
        assert!(result.is_ok(), "Should be able to create device monitor");
    }

    #[test]
    fn test_device_status_tracking() {
        let (mut monitor, _rx) =
            DeviceMonitor::new(Duration::from_millis(100)).expect("Failed to create monitor");

        // Test setting current device
        monitor.set_current_device(Some("test_device".to_string()));
        assert_eq!(monitor.current_device(), Some(&"test_device".to_string()));

        // Test clearing current device
        monitor.set_current_device(None);
        assert_eq!(monitor.current_device(), None);
    }

    #[test]
    fn test_device_preferences() {
        let (mut monitor, _rx) =
            DeviceMonitor::new(Duration::from_millis(100)).expect("Failed to create monitor");

        // Test setting preferred devices
        let preferred = vec!["device1".to_string(), "device2".to_string()];
        monitor.set_preferred_devices(preferred.clone());

        // Test getting candidates with preferences
        let candidates = monitor.get_device_candidates();
        // Should include system candidates since preferred devices may not be available
        assert!(!candidates.is_empty());
    }

    #[test]
    fn test_manual_device_switch_request() {
        let (monitor, _rx) =
            DeviceMonitor::new(Duration::from_millis(100)).expect("Failed to create monitor");

        // Test requesting switch to non-existent device
        let result = monitor.request_device_switch("non_existent_device".to_string());
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(matches!(e, AudioError::DeviceNotFound { .. }));
        }
    }

    #[test]
    fn test_device_switch_requested_event() {
        // Test that we can create device switch request event
        let request_event = DeviceEvent::DeviceSwitchRequested {
            target: "target_device".to_string(),
        };
        assert!(matches!(
            request_event,
            DeviceEvent::DeviceSwitchRequested { .. }
        ));
    }

    #[test]
    fn test_device_monitor_integration() {
        // Integration test showing device monitor usage
        let (mut monitor, mut event_rx) =
            DeviceMonitor::new(Duration::from_millis(200)).expect("Failed to create monitor");

        // Test the complete workflow
        let device_candidates = monitor.get_device_candidates();
        println!("Available device candidates: {:?}", device_candidates);

        // Test setting preferences
        if !device_candidates.is_empty() {
            let first_device = device_candidates[0].clone();
            monitor.set_preferred_devices(vec![first_device.clone()]);

            // Test preference-aware candidates
            let preferred_candidates = monitor.get_device_candidates();
            println!("Preferred candidates: {:?}", preferred_candidates);

            // Should get the preferred device first (if available)
            assert!(!preferred_candidates.is_empty());
        }

        // Test monitoring for a short time
        let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        let monitor_handle = monitor.start_monitoring(running.clone());

        // Wait briefly for monitor to initialize
        std::thread::sleep(Duration::from_millis(300));

        // Check if any events were generated
        match event_rx.try_recv() {
            Ok(event) => {
                println!("Device monitor generated event: {:?}", event);
            }
            Err(_) => {
                println!("No device events in test period (expected in CI)");
            }
        }

        // Clean shutdown
        running.store(false, std::sync::atomic::Ordering::Relaxed);
        let _ = monitor_handle.join();

        println!("Device monitor integration test completed successfully");
    }

    #[test]
    fn test_device_availability_check() {
        let (monitor, _rx) =
            DeviceMonitor::new(Duration::from_millis(100)).expect("Failed to create monitor");

        // Non-existent device should not be available
        assert!(!monitor.is_device_available("non_existent_device"));
    }
}
