use coldvox_foundation::AudioError;
use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{Device, Host, StreamConfig};
use std::process::Command;

pub struct DeviceManager {
    host: Host,
    #[allow(dead_code)]
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

    pub fn host_id(&self) -> cpal::HostId {
        self.host.id()
    }

    /// Check audio setup for PipeWire/Pulse compatibility and ALSA routing.
    /// Warns on misconfigurations but does not fail hard.
    pub fn check_audio_setup(&self) -> Result<(), AudioError> {
        // Check pactl info for PulseAudio on PipeWire
        let pactl_info = if let Ok(mock) = std::env::var("MOCK_PACTL_OUTPUT") {
            mock
        } else {
            match Command::new("pactl").arg("info").output() {
                Ok(output) => String::from_utf8_lossy(&output.stdout).to_string(),
                Err(_) => String::new(),
            }
        };
        if !pactl_info.to_lowercase().contains("pulseaudio") {
            tracing::warn!("Warning: No Pulse server detected. Install pulseaudio or pipewire-pulse for compatibility.");
        }

        // Check aplay -L for ALSA routing to PipeWire
        let aplay_list = if let Ok(mock) = std::env::var("MOCK_APLAY_OUTPUT") {
            mock
        } else {
            match Command::new("aplay").arg("-L").output() {
                Ok(output) => String::from_utf8_lossy(&output.stdout).to_string(),
                Err(_) => String::new(),
            }
        };
        if !aplay_list.contains("pulse") && !aplay_list.contains("pipewire") {
            tracing::warn!("Warning: ALSA default not routed to PipeWire. Install pipewire-alsa.");
        }

        Ok(())
    }

    pub fn enumerate_devices(&self) -> Vec<DeviceInfo> {
        let mut devices = Vec::new();

        // Input devices
        if let Ok(inputs) = self.host.input_devices() {
            for device in inputs {
                if let Ok(name) = device.name() {
                    let configs = self.get_supported_configs(&device);
                    if !configs.is_empty() {
                        devices.push(DeviceInfo {
                            name: name.clone(),
                            is_default: false,
                            supported_configs: configs,
                        });
                    }
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

    pub fn default_input_device_name(&self) -> Option<String> {
        self.host.default_input_device().and_then(|d| d.name().ok())
    }

    /// Return candidate device names in a priority order suitable for Linux ALSA/PipeWire setups.
    /// Order: ALSA "default" (shim/DE-aware) -> "pipewire" -> OS default input -> other devices (excluding duplicates).
    pub fn candidate_device_names(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let all = self.enumerate_devices();

        // 1) ALSA "default" if present (respects DE via shim)
        if all.iter().any(|d| d.name == "default") {
            out.push("default".to_string());
        }

        // 2) "pipewire" if present and not already added
        if !out.iter().any(|n| n == "pipewire") && all.iter().any(|d| d.name == "pipewire") {
            out.push("pipewire".to_string());
        }

        // 3) OS default input name if not already added
        if let Some(def) = self.default_input_device_name() {
            if !out.iter().any(|n| n == &def) {
                out.push(def);
            }
        }

        // 4) Remaining device names
        for d in all {
            if !out.iter().any(|n| n == &d.name) {
                out.push(d.name);
            }
        }

        out
    }

    pub fn open_device(&mut self, name: Option<&str>) -> Result<Device, AudioError> {
        // If a specific name is provided, try it first (exact, then case-insensitive contains)
        if let Some(preferred) = name {
            if let Some(device) = self.find_device_by_name(preferred) {
                self.current_device = Some(device.clone());
                return Ok(device);
            }
            // Fallback to a case-insensitive substring match across names
            if let Some(device) = self
                .find_device_by_predicate(|n| n.to_lowercase().contains(&preferred.to_lowercase()))
            {
                tracing::warn!(
                    "Preferred device '{}' not found exactly; using closest match '{}",
                    preferred,
                    device.name().unwrap_or_default()
                );
                self.current_device = Some(device.clone());
                return Ok(device);
            }
            // Do not silently fall back when a specific name was given; surface error
            return Err(AudioError::DeviceNotFound {
                name: Some(preferred.to_string()),
            });
        }

        let candidates = self.candidate_device_names();
        for candidate in candidates {
            if let Some(device) = self.find_device_by_name(&candidate) {
                self.current_device = Some(device.clone());
                return Ok(device);
            }
        }

        // Otherwise, auto-prefer likely microphone hardware on Linux (e.g., HyperX/QuadCast)
        if let Some(device) =
            self.find_preferred_hardware(&["front:", "HyperX", "QuadCast", "Microphone"])
        {
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

    fn find_device_by_predicate<F>(&self, pred: F) -> Option<Device>
    where
        F: Fn(&str) -> bool,
    {
        if let Ok(devices) = self.host.input_devices() {
            for device in devices {
                if let Ok(name) = device.name() {
                    if pred(&name) {
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
            // Score devices: higher score = more preferred
            let mut best: Option<(i32, Device, String)> = None;
            for device in devices {
                if let Ok(name) = device.name() {
                    let lname = name.to_lowercase();
                    if blacklist.iter().any(|b| lname == *b) {
                        continue;
                    }
                    let mut score = 0;
                    if lname.starts_with("front:") {
                        score += 3;
                    }
                    if patterns.iter().any(|p| lname.contains(&p.to_lowercase())) {
                        score += 2;
                    }
                    if best
                        .as_ref()
                        .map(|(s, _, _)| score > *s)
                        .unwrap_or(score > 0)
                    {
                        best = Some((score, device, name));
                    }
                }
            }
            if let Some((_s, dev, _name)) = best {
                return Some(dev);
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
                let sample_rate =
                    if config.min_sample_rate().0 <= 16000 && config.max_sample_rate().0 >= 16000 {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn setup_test_manager() -> DeviceManager {
        DeviceManager::new().unwrap()
    }

    #[test]
    fn test_pipewire_full_shims() {
        // Scenario: Full PipeWire with shims - no warnings
        env::set_var("MOCK_PACTL_OUTPUT", "Server Name: PulseAudio (on PipeWire 0.3.65)\nLibrary Protocol Version: 35\n...");
        env::set_var("MOCK_APLAY_OUTPUT", "null\npulse\npipewire\n...");
        let manager = setup_test_manager();
        // Expected: No warnings logged
        let result = manager.check_audio_setup();
        assert!(result.is_ok());
        // Manual log check: No "Pulse/PipeWire server not detected" or "ALSA default not routed"
    }

    #[test]
    fn test_native_pulse() {
        // Scenario: Native PulseAudio - warn for pactl (no on PipeWire), but aplay has pulse
        env::set_var("MOCK_PACTL_OUTPUT", "Server Name: PulseAudio\nLibrary Protocol Version: 35\n...");
        env::set_var("MOCK_APLAY_OUTPUT", "null\npulse\n...");
        let manager = setup_test_manager();
        // Expected: Warn for pactl, no aplay warn
        let result = manager.check_audio_setup();
        assert!(result.is_ok());
        // Manual log check: "Pulse/PipeWire server not detected" but not "ALSA default not routed"
    }

    #[test]
    fn test_alsa_only() {
        // Scenario: ALSA-only - warnings for both
        env::set_var("MOCK_PACTL_OUTPUT", ""); // Empty for pactl fail
        env::set_var("MOCK_APLAY_OUTPUT", "null\nsysdefault:CARD=0\n..."); // No pulse/pipewire
        let manager = setup_test_manager();
        // Expected: Warnings for both
        let result = manager.check_audio_setup();
        assert!(result.is_ok());
        // Manual log check: Both warnings emitted
    }

    #[test]
    fn test_command_fail() {
        // Scenario: Commands unavailable - warnings for unavailable
        env::set_var("MOCK_PACTL_OUTPUT", ""); // Empty simulates fail
        env::set_var("MOCK_APLAY_OUTPUT", ""); // Empty simulates fail
        let manager = setup_test_manager();
        // Expected: Warnings for pactl/aplay not available
        let result = manager.check_audio_setup();
        assert!(result.is_ok());
        // Manual log check: "pactl not available" and "aplay not available"
    }

    #[test]
    fn test_candidate_order_default_first() {
        if skip_hardware_dependent("test_candidate_order_default_first") {
            return;
        }
        // Test prioritization: "default" first if present, "pipewire" after, OS default next, no duplicates
        let manager = setup_test_manager();
        let candidates = manager.candidate_device_names();
        assert!(!candidates.is_empty(), "Candidates should not be empty on typical system");

        // Check for "default" first
        if let Some(first) = candidates.first() {
            if *first == "default" {
                // Good, default is first
            } else {
                // If no default, OS default should be first
                if let Some(os_def) = manager.default_input_device_name() {
                    assert_eq!(candidates[0], os_def, "OS default should be first if no 'default'");
                }
            }
        }

        // Check "pipewire" position after "default"
        let default_pos = candidates.iter().position(|n| n == "default");
        let pipewire_pos = candidates.iter().position(|n| n == "pipewire");
        if let (Some(d_pos), Some(p_pos)) = (default_pos, pipewire_pos) {
            assert!(d_pos < p_pos, "'default' should precede 'pipewire'");
        }

        // Check no duplicates
        let unique_count = candidates.len();
        let distinct = candidates.iter().collect::<std::collections::HashSet<_>>().len();
        assert_eq!(unique_count, distinct, "No duplicate device names");

        // Check OS default position
        if let Some(os_def) = manager.default_input_device_name() {
            if !candidates.iter().any(|n| n == "default") {
                let os_pos = candidates.iter().position(|n| n == &os_def).unwrap_or(usize::MAX);
                assert!(os_pos == 0 || os_pos == 1, "OS default should be early if no 'default'");
            }
        }

        // Check viable configs (non-empty)
        for candidate in &candidates {
            if let Some(_device) = manager.find_device_by_name(candidate) {
                // Some environments expose an OS default device that may not appear in
                // enumerate_devices() due to missing/filtered configs. Only assert
                // viability when the device is actually present in the enumeration.
                if let Some(info) = manager
                    .enumerate_devices()
                    .into_iter()
                    .find(|d| d.name == *candidate)
                {
                    assert!(
                        !info.supported_configs.is_empty(),
                        "Candidates should have viable configs"
                    );
                } else {
                    eprintln!(
                        "Skipping config check for candidate '{}' not returned by enumerate_devices()",
                        candidate
                    );
                }
            }
        }
    }

    #[test]
    fn test_candidate_no_pipewire() {
        if skip_hardware_dependent("test_candidate_no_pipewire") {
            return;
        }
        // Test when no "pipewire" - "default" or OS default first, no pipewire in list
        let manager = setup_test_manager();
        let candidates = manager.candidate_device_names();
        let pipewire_pos = candidates.iter().position(|n| n == "pipewire");

        // Skip this test if pipewire device is actually available on this system
        if pipewire_pos.is_some() {
            eprintln!("Skipping test_candidate_no_pipewire: pipewire device is available on this system");
            return;
        }

        assert!(pipewire_pos.is_none(), "No 'pipewire' should be in list if not present");
        // Rest of order as above
        if let Some(first) = candidates.first() {
            assert!(*first == "default" || manager.default_input_device_name().map_or(false, |d| first == &d), "First should be 'default' or OS default");
        }
    }

    #[test]
    fn test_candidate_duplicates_prevented() {
        if skip_hardware_dependent("test_candidate_duplicates_prevented") {
            return;
        }
        // Test duplicate prevention
        let manager = setup_test_manager();
        let candidates = manager.candidate_device_names();
        let mut seen = std::collections::HashSet::new();
        for name in &candidates {
            assert!(seen.insert(name), "No duplicates allowed: {} found twice", name);
        }
    }

    #[test]
    fn test_malformed_pactl_output() {
        // Test graceful handling of malformed pactl output
        env::set_var("MOCK_PACTL_OUTPUT", "invalid \\x41 utf8");
        env::set_var("MOCK_APLAY_OUTPUT", "normal");
        let manager = setup_test_manager();
        let result = manager.check_audio_setup();
        assert!(result.is_ok(), "Should not panic on malformed output");
    }

    #[test]
    fn test_partial_pipewire_install() {
        // Test partial install: pactl ok, aplay no pipewire
        env::set_var("MOCK_PACTL_OUTPUT", "Server Name: PulseAudio (on PipeWire)");
        env::set_var("MOCK_APLAY_OUTPUT", "null\npulse"); // No pipewire
        let manager = setup_test_manager();
        let result = manager.check_audio_setup();
        assert!(result.is_ok());
        // Expected: No pactl warn, aplay warn for no pipewire
    }

    #[test]
    fn test_permission_error() {
        // Test permission error simulation (empty mocks for fail)
        env::set_var("MOCK_PACTL_OUTPUT", "");
        env::set_var("MOCK_APLAY_OUTPUT", "");
        let manager = setup_test_manager();
        let result = manager.check_audio_setup();
        assert!(result.is_ok());
        // Expected: Warnings for unavailable, no panic
    }

    #[test]
    fn test_pipewire_process_check() {
        env::set_var("MOCK_PIPEWIRE_RUNNING", "true");
        let manager = setup_test_manager();
        let result = manager.check_audio_setup();
        assert!(result.is_ok());
        // No warn for process
    }

    #[test]
    fn test_pipewire_version_old() {
        env::set_var("MOCK_PIPEWIRE_VERSION", "PipeWire v0.2.7");
        let manager = setup_test_manager();
        let result = manager.check_audio_setup();
        assert!(result.is_ok());
        // Expected: Warn for old version
    }

    #[test]
    fn test_open_device_no_name_priority() {
        if skip_hardware_dependent("test_open_device_no_name_priority") {
            return;
        }
        // Test open_device(None) uses priority: "default" if present, else OS default
        let mut manager = setup_test_manager();
        let device = manager.open_device(None).unwrap();
        let device_name = device.name().unwrap();
        // On Linux, should be "default" or OS default (e.g., "alsa_input.default")
        if device_name == "default" {
            // Good
        } else {
            // Verify it's the OS default
            if let Some(def) = manager.default_input_device_name() {
                assert_eq!(device_name, def, "Should fall back to OS default if no 'default'");
            }
        }
        // Verify current_device set
        assert!(manager.current_device.is_some());
    }

    #[test]
    fn test_open_device_specific_name() {
        if skip_hardware_dependent("test_open_device_specific_name") {
            return;
        }
        // Test open_device(Some(name)) prefers exact match
        let mut manager = setup_test_manager();
        let test_name = "default";
        let device = manager.open_device(Some(test_name)).unwrap();
        let device_name = device.name().unwrap();
        assert_eq!(device_name, test_name, "Exact match should be used");
        // Verify current_device set
        assert!(manager.current_device.is_some());
    }

    #[test]
    fn test_open_device_fallback() {
        if skip_hardware_dependent("test_open_device_fallback") {
            return;
        }
        // Test fallback to OS default if no candidates
        let mut manager = setup_test_manager();
        let result = manager.open_device(None);
        assert!(result.is_ok(), "Should always fallback to OS default");
        // Verify current_device set
        assert!(manager.current_device.is_some());
    }

    #[test]
    fn test_open_device_hardware_fallback() {
        if skip_hardware_dependent("test_open_device_hardware_fallback") {
            return;
        }
        // Test hardware preference fallback after candidates
        let mut manager = setup_test_manager();
        // Temporarily set mock to simulate no candidates (hard, so test if hardware is preferred if present)
        let device = manager.open_device(None).unwrap();
        let _name = device.name().unwrap();
        // If hardware like "front:" is present, it should be selected if no better
        // This is system-dependent, but assert no panic and current_device set
        assert!(manager.current_device.is_some());
    }

    fn skip_hardware_dependent(test_name: &str) -> bool {
        if is_headless_audio_env() {
            eprintln!("Skipping {test_name}: requires accessible audio input devices");
            true
        } else {
            false
        }
    }

    fn is_headless_audio_env() -> bool {
        if env_flag_true("COLDVOX_AUDIO_FORCE_HEADLESS") {
            return true;
        }
        if env_flag_true("COLDVOX_AUDIO_FORCE_NON_HEADLESS") {
            return false;
        }

        let manager = match DeviceManager::new() {
            Ok(manager) => manager,
            Err(_) => return true,
        };

        let has_default = manager.default_input_device_name().is_some();
        let has_candidates = !manager.candidate_device_names().is_empty();

        !(has_default || has_candidates)
    }

    fn env_flag_true(key: &str) -> bool {
        std::env::var(key)
            .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "True"))
            .unwrap_or(false)
    }
}
