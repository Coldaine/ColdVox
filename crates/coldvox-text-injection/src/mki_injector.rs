use crate::types::{InjectionConfig, InjectionError, InjectionResult};
use crate::TextInjector;
use async_trait::async_trait;
// Note: debug was used for MKI logging but since MKI is disabled, removing unused import

// Note: mouse_keyboard_input::VirtualKeyboard doesn't exist in the current version
// Disabling MKI functionality until we can find a working alternative

/// Mouse-keyboard-input (MKI) injector for synthetic key events
pub struct MkiInjector {
    config: InjectionConfig,
    /// Whether MKI is available and can be used
    is_available: bool,
}

impl MkiInjector {
    /// Create a new MKI injector
    pub fn new(config: InjectionConfig) -> Self {
        let is_available = Self::check_availability();

        Self {
            config,
            is_available,
        }
    }

    /// Check if MKI can be used (permissions, backend availability)
    fn check_availability() -> bool {
        // Check if user is in input group
        let in_input_group = std::process::Command::new("groups")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains("input"))
            .unwrap_or(false);

        // Check if /dev/uinput is accessible
        let uinput_accessible = std::fs::metadata("/dev/uinput")
            .map(|metadata| {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mode = metadata.permissions().mode();
                    (mode & 0o060) == 0o060 || (mode & 0o006) == 0o006
                }
                #[cfg(not(unix))]
                false
            })
            .unwrap_or(false);

        in_input_group && uinput_accessible
    }

    /// Trigger paste action using MKI (Ctrl+V)
    #[cfg(feature = "mki")]
    pub async fn trigger_paste(&self) -> Result<(), InjectionError> {
        // MKI functionality disabled due to missing VirtualKeyboard in mouse_keyboard_input crate
        Err(InjectionError::MethodUnavailable(
            "MKI VirtualKeyboard not available in current mouse-keyboard-input version".to_string(),
        ))
    }

    /// Trigger paste action using MKI (feature disabled stub)
    #[cfg(not(feature = "mki"))]
    pub async fn trigger_paste(&self) -> Result<(), InjectionError> {
        Err(InjectionError::MethodUnavailable(
            "MKI feature not enabled".to_string(),
        ))
    }
}

#[async_trait]
impl TextInjector for MkiInjector {
    async fn inject_text(&self, text: &str) -> InjectionResult<()> {
        if text.is_empty() {
            return Ok(());
        }

        if !self.config.allow_mki {
            return Err(InjectionError::MethodNotAvailable(
                "MKI not allowed".to_string(),
            ));
        }

        #[cfg(feature = "mki")]
        {
            // MKI functionality disabled due to missing VirtualKeyboard in mouse_keyboard_input crate
            let _ = text; // Use the parameter to avoid unused warning
            Err(InjectionError::MethodUnavailable(
                "MKI VirtualKeyboard not available in current mouse-keyboard-input version"
                    .to_string(),
            ))
        }
        #[cfg(not(feature = "mki"))]
        {
            Err(InjectionError::MethodUnavailable(
                "MKI feature not enabled".to_string(),
            ))
        }
    }

    async fn is_available(&self) -> bool {
        self.is_available && self.config.allow_mki
    }

    fn backend_name(&self) -> &'static str {
        "MKI"
    }

    fn backend_info(&self) -> Vec<(&'static str, String)> {
        vec![
            ("type", "keyboard".to_string()),
            ("requires_permissions", "true".to_string()),
            (
                "description",
                "Mouse-keyboard-input uinput backend".to_string(),
            ),
            ("allowed", self.config.allow_mki.to_string()),
        ]
    }
}
