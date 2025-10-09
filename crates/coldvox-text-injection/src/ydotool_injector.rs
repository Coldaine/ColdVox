use crate::types::{InjectionConfig, InjectionError, InjectionResult};
use crate::TextInjector;
use anyhow::Result;
use async_trait::async_trait;
use std::process::Command;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{debug, info, warn};

/// Ydotool injector for synthetic key events
pub struct YdotoolInjector {
    config: InjectionConfig,
    /// Whether ydotool is available on the system
    is_available: bool,
}

impl YdotoolInjector {
    /// Create a new ydotool injector
    pub fn new(config: InjectionConfig) -> Self {
        let is_available = Self::check_ydotool();

        Self {
            config,
            is_available,
        }
    }

    /// Check if ydotool is available on the system
    fn check_ydotool() -> bool {
        match Self::check_binary_permissions("ydotool") {
            Ok(()) => {
                // Check if the ydotool socket exists (most reliable check)
                let user_id = std::env::var("UID").unwrap_or_else(|_| "1000".to_string());
                let socket_path = format!("/run/user/{}/.ydotool_socket", user_id);
                if !std::path::Path::new(&socket_path).exists() {
                    warn!(
                        "ydotool socket not found at {}, daemon may not be running",
                        socket_path
                    );
                    return false;
                }
                true
            }
            Err(e) => {
                warn!("ydotool not available: {}", e);
                false
            }
        }
    }

    /// Check if a binary exists and has proper permissions
    fn check_binary_permissions(binary_name: &str) -> Result<(), InjectionError> {
        use std::os::unix::fs::PermissionsExt;

        // Check if binary exists in PATH
        let output = Command::new("which")
            .arg(binary_name)
            .output()
            .map_err(|e| {
                InjectionError::Process(format!("Failed to locate {}: {}", binary_name, e))
            })?;

        if !output.status.success() {
            return Err(InjectionError::MethodUnavailable(format!(
                "{} not found in PATH",
                binary_name
            )));
        }

        let binary_path = String::from_utf8_lossy(&output.stdout).trim().to_string();

        // Check if binary is executable
        let metadata = std::fs::metadata(&binary_path).map_err(InjectionError::Io)?;

        let permissions = metadata.permissions();
        if permissions.mode() & 0o111 == 0 {
            return Err(InjectionError::PermissionDenied(format!(
                "{} is not executable",
                binary_name
            )));
        }

        // For ydotool specifically, check uinput access
        if binary_name == "ydotool" {
            Self::check_uinput_access()?;
        }

        Ok(())
    }

    /// Check if we have access to /dev/uinput (required for ydotool)
    fn check_uinput_access() -> Result<(), InjectionError> {
        use std::fs::OpenOptions;

        // Check if we can open /dev/uinput
        match OpenOptions::new().write(true).open("/dev/uinput") {
            Ok(_) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                // Check if user is in input group
                let groups = Command::new("groups").output().map_err(|e| {
                    InjectionError::Process(format!("Failed to check groups: {}", e))
                })?;

                let groups_str = String::from_utf8_lossy(&groups.stdout);
                if !groups_str.contains("input") {
                    return Err(InjectionError::PermissionDenied(
                        "User not in 'input' group. Run: sudo usermod -a -G input $USER"
                            .to_string(),
                    ));
                }

                Err(InjectionError::PermissionDenied(
                    "/dev/uinput access denied. ydotool daemon may not be running".to_string(),
                ))
            }
            Err(e) => Err(InjectionError::Io(e)),
        }
    }

    /// Trigger paste action using ydotool (Ctrl+V)
    async fn trigger_paste(&self) -> Result<(), InjectionError> {
        let _start = std::time::Instant::now();

        // Use tokio to run the command with timeout
        let output = timeout(
            Duration::from_millis(self.config.paste_action_timeout_ms),
            tokio::process::Command::new("ydotool")
                .args(["key", "ctrl+v"])
                .output(),
        )
        .await
        .map_err(|_| InjectionError::Timeout(self.config.paste_action_timeout_ms))?
        .map_err(|e| InjectionError::Process(format!("{e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(InjectionError::MethodFailed(format!(
                "ydotool key failed: {}",
                stderr
            )));
        }

        info!("Successfully triggered paste action via ydotool");

        Ok(())
    }

    /// Type text directly using ydotool
    async fn _type_text(&self, text: &str) -> Result<(), InjectionError> {
        let _start = std::time::Instant::now();

        // Use tokio to run the command with timeout
        let output = timeout(
            Duration::from_millis(self.config.per_method_timeout_ms),
            tokio::process::Command::new("ydotool")
                .args(["type", "--delay", "10", text])
                .output(),
        )
        .await
        .map_err(|_| InjectionError::Timeout(self.config.per_method_timeout_ms))?
        .map_err(|e| InjectionError::Process(format!("{e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(InjectionError::MethodFailed(format!(
                "ydotool type failed: {}",
                stderr
            )));
        }

        info!("Successfully typed text via ydotool ({} chars)", text.len());

        Ok(())
    }
}

#[async_trait]
impl TextInjector for YdotoolInjector {
    async fn inject_text(&self, text: &str, _context: Option<&crate::types::InjectionContext>) -> InjectionResult<()> {
        if text.is_empty() {
            return Ok(());
        }

        // First try paste action (more reliable for batch text)
        match self.trigger_paste().await {
            Ok(()) => Ok(()),
            Err(e) => {
                debug!("Paste action failed: {}", e);
                // Fall back to direct typing
                self._type_text(text).await
            }
        }
    }

    async fn is_available(&self) -> bool {
        self.is_available
    }

    fn backend_name(&self) -> &'static str {
        "Ydotool"
    }

    fn backend_info(&self) -> Vec<(&'static str, String)> {
        vec![
            ("type", "uinput".to_string()),
            ("requires_daemon", "true".to_string()),
            (
                "description",
                "Ydotool uinput automation backend".to_string(),
            ),
        ]
    }
}
