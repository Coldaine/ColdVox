use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::thread::{self, JoinHandle};
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

use coldvox_foundation::{AudioError, DeviceEvent, DeviceStatus};
use super::device::DeviceManager;

/// Monitor for audio device changes and hotplug events
pub struct DeviceMonitor {
    device_manager: DeviceManager,
    event_tx: broadcast::Sender<DeviceEvent>,
    monitor_interval: Duration,
    last_devices: HashMap<String, DeviceStatus>,
    current_device: Option<String>,
}

impl DeviceMonitor {
    pub fn new(monitor_interval: Duration) -> Result<(Self, broadcast::Receiver<DeviceEvent>), AudioError> {
        let device_manager = DeviceManager::new()?;
        let (event_tx, event_rx) = broadcast::channel(32);
        
        let monitor = Self {
            device_manager,
            event_tx,
            monitor_interval,
            last_devices: HashMap::new(),
            current_device: None,
        };
        
        Ok((monitor, event_rx))
    }

    /// Start monitoring in a background thread
    pub fn start_monitoring(
        mut self,
        running: Arc<AtomicBool>,
    ) -> JoinHandle<()> {
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
                let _ = self.event_tx.send(DeviceEvent::DeviceAdded { 
                    name: name.clone() 
                });
            }
            
            new_device_map.insert(name, status);
        }
        
        // Check for removed devices
        for (old_name, old_status) in &self.last_devices {
            if !new_device_map.contains_key(old_name) {
                debug!("Device removed: {}", old_name);
                let _ = self.event_tx.send(DeviceEvent::DeviceRemoved { 
                    name: old_name.clone() 
                });
                
                // If current device was removed, signal disconnection
                if old_status.is_current {
                    warn!("Current device disconnected: {}", old_name);
                    let _ = self.event_tx.send(DeviceEvent::CurrentDeviceDisconnected { 
                        name: old_name.clone() 
                    });
                }
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
        self.last_devices.get(device_name)
            .map(|status| status.is_available)
            .unwrap_or(false)
    }

    /// Get the current device name
    pub fn current_device(&self) -> Option<&String> {
        self.current_device.as_ref()
    }

    /// Get preferred device candidates in priority order
    pub fn get_device_candidates(&self) -> Vec<String> {
        self.device_manager.candidate_device_names()
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
        let (mut monitor, _rx) = DeviceMonitor::new(Duration::from_millis(100))
            .expect("Failed to create monitor");
        
        // Test setting current device
        monitor.set_current_device(Some("test_device".to_string()));
        assert_eq!(monitor.current_device(), Some(&"test_device".to_string()));
        
        // Test clearing current device
        monitor.set_current_device(None);
        assert_eq!(monitor.current_device(), None);
    }

    #[test]
    fn test_device_availability_check() {
        let (monitor, _rx) = DeviceMonitor::new(Duration::from_millis(100))
            .expect("Failed to create monitor");
        
        // Non-existent device should not be available
        assert!(!monitor.is_device_available("non_existent_device"));
    }
}