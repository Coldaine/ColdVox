use cpal::{Device, Host, StreamConfig};
use cpal::traits::{DeviceTrait, HostTrait};
use crate::foundation::error::AudioError;

pub struct DeviceManager {
    host: Host,
    preferred_device: Option<String>,
    current_device: Option<Device>,
}

impl DeviceManager {
    pub fn new() -> Result<Self, AudioError> {
        let host = cpal::default_host();
        Ok(Self {
            host,
            preferred_device: None,
            current_device: None,
        })
    }
    
    pub fn enumerate_devices(&self) -> Vec<DeviceInfo> {
        let mut devices = Vec::new();
        
        // Input devices
        if let Ok(inputs) = self.host.input_devices() {
            for device in inputs {
                if let Ok(name) = device.name() {
                    devices.push(DeviceInfo {
                        name: name.clone(),
                        is_default: false,
                        supported_configs: self.get_supported_configs(&device),
                    });
                }
            }
        }
        
        // Mark default
        if let Some(default) = self.host.default_input_device() {
            if let Ok(default_name) = default.name() {
                for device in &mut devices {
                    if device.name == default_name {
                        device.is_default = true;
                    }
                }
            }
        }
        
        devices
    }
    
    pub fn open_device(&mut self, name: Option<&str>) -> Result<Device, AudioError> {
        // If a specific name is provided, try it first
        if let Some(preferred) = name {
            if let Some(device) = self.find_device_by_name(preferred) {
                self.current_device = Some(device.clone());
                return Ok(device);
            }
            tracing::warn!("Preferred device '{}' not found, attempting auto-selection", preferred);
        }

        // Auto-prefer likely microphone hardware on Linux (e.g., HyperX/QuadCast) before default bridge
        if let Some(device) = self.find_preferred_hardware(&["HyperX", "QuadCast", "Microphone"]) {
            self.current_device = Some(device.clone());
            return Ok(device);
        }

        // Fall back to OS default
        self.host
            .default_input_device()
            .ok_or(AudioError::DeviceNotFound { name: None })
            .map(|device| {
                self.current_device = Some(device.clone());
                device
            })
    }
    
    fn find_device_by_name(&self, name: &str) -> Option<Device> {
        if let Ok(devices) = self.host.input_devices() {
            for device in devices {
                if let Ok(device_name) = device.name() {
                    if device_name == name {
                        return Some(device);
                    }
                }
            }
        }
        None
    }

    fn find_preferred_hardware(&self, patterns: &[&str]) -> Option<Device> {
        if let Ok(devices) = self.host.input_devices() {
            // Prefer concrete device names over virtual bridges like "default"/"pipewire"/"sysdefault"
            let blacklist = ["default", "sysdefault", "pipewire"]; 
            for device in devices {
                if let Ok(name) = device.name() {
                    let is_blacklisted = blacklist.iter().any(|b| name.eq_ignore_ascii_case(b));
                    if is_blacklisted { continue; }
                    if patterns.iter().any(|p| name.to_lowercase().contains(&p.to_lowercase())) {
                        return Some(device);
                    }
                }
            }
        }
        None
    }
    
    fn get_supported_configs(&self, device: &Device) -> Vec<StreamConfig> {
        // Get all supported configs, prioritize 16kHz mono
        let mut configs = Vec::new();
        
        if let Ok(supported) = device.supported_input_configs() {
            for config in supported {
                // We prefer 16kHz, but will take anything
                let sample_rate = if config.min_sample_rate().0 <= 16000 
                    && config.max_sample_rate().0 >= 16000 {
                    cpal::SampleRate(16000)
                } else {
                    config.max_sample_rate()
                };
                
                configs.push(StreamConfig {
                    channels: config.channels(),
                    sample_rate,
                    buffer_size: cpal::BufferSize::Default,
                });
            }
        }
        
        configs
    }
}

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub name: String,
    pub is_default: bool,
    pub supported_configs: Vec<StreamConfig>,
}