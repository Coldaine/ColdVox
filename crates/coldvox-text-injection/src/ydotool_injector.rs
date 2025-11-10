use crate::types::{InjectionConfig, InjectionResult};
use crate::TextInjector;
use anyhow::Result;
use async_trait::async_trait;
use coldvox_foundation::error::InjectionError;
use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use tokio::process::Command as TokioCommand;
use tokio::time::timeout;
use tracing::{debug, info, trace, warn};

fn push_unique(paths: &mut Vec<PathBuf>, candidate: PathBuf) {
    if candidate.as_os_str().is_empty() {
        return;
    }
    if !paths.iter().any(|existing| existing == &candidate) {
        paths.push(candidate);
    }
}

fn candidate_socket_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Some(env_socket) = env::var_os("YDOTOOL_SOCKET") {
        push_unique(&mut paths, PathBuf::from(env_socket));
    }

    if let Some(home) = env::var_os("HOME") {
        push_unique(
            &mut paths,
            PathBuf::from(home).join(".ydotool").join("socket"),
        );
    }

    if let Some(runtime_dir) = env::var_os("XDG_RUNTIME_DIR") {
        push_unique(
            &mut paths,
            PathBuf::from(runtime_dir).join(".ydotool_socket"),
        );
    }

    if let Ok(uid) = env::var("UID") {
        push_unique(
            &mut paths,
            PathBuf::from(format!("/run/user/{uid}/.ydotool_socket")),
        );
    }

    paths
}

fn locate_existing_socket() -> Option<PathBuf> {
    #[allow(clippy::manual_find)]
    for candidate in candidate_socket_paths() {
        if Path::new(&candidate).exists() {
            return Some(candidate);
        }
    }
    None
}

fn preferred_socket_path() -> Option<PathBuf> {
    env::var_os("HOME").map(|home| PathBuf::from(home).join(".ydotool").join("socket"))
}

pub(crate) fn ydotool_daemon_socket() -> Option<PathBuf> {
    locate_existing_socket()
}

pub(crate) fn ydotool_socket_env_value() -> Option<OsString> {
    if let Some(value) = env::var_os("YDOTOOL_SOCKET") {
        if !value.is_empty() {
            return Some(value);
        }
    }

    if let Some(existing) = locate_existing_socket() {
        return Some(existing.into_os_string());
    }

    preferred_socket_path().map(|path| path.into_os_string())
}

pub(crate) fn apply_socket_env(command: &mut TokioCommand) {
    if let Some(socket) = ydotool_socket_env_value() {
        command.env("YDOTOOL_SOCKET", socket);
    }
}

pub(crate) fn ydotool_runtime_available() -> bool {
    if YdotoolInjector::check_binary_permissions("ydotool").is_err() {
        return false;
    }

    if let Some(socket) = ydotool_daemon_socket() {
        if env::var_os("YDOTOOL_SOCKET").is_none() {
            env::set_var("YDOTOOL_SOCKET", &socket);
            trace!("Set YDOTOOL_SOCKET to {}", socket.display());
        }
        true
    } else {
        false
    }
}

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
                if let Some(socket) = ydotool_daemon_socket() {
                    if env::var_os("YDOTOOL_SOCKET").is_none() {
                        env::set_var("YDOTOOL_SOCKET", &socket);
                        trace!("Configured YDOTOOL_SOCKET to {}", socket.display());
                    }
                    true
                } else {
                    let expected = preferred_socket_path()
                        .map(|path| path.display().to_string())
                        .unwrap_or_else(|| "<unknown>".to_string());
                    warn!(
                        "ydotool socket not found (expected {}). ydotoold daemon may not be running.",
                        expected
                    );
                    false
                }
            }
            Err(e) => {
                warn!("ydotool not available: {}", e);
                false
            }
        }
    }

    /// Check if a binary exists and has proper permissions
    pub(crate) fn check_binary_permissions(binary_name: &str) -> Result<(), InjectionError> {
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

    #[cfg(test)]
    pub(crate) fn check_binary_for_tests(binary_name: &str) -> Result<(), InjectionError> {
        Self::check_binary_permissions(binary_name)
    }

    /// Trigger paste action using ydotool (Ctrl+V)
    async fn trigger_paste(&self) -> Result<(), InjectionError> {
        let _start = std::time::Instant::now();

        let mut command = TokioCommand::new("ydotool");
        apply_socket_env(&mut command);
        command.args(["key", "ctrl+v"]);

        let output = timeout(
            Duration::from_millis(self.config.paste_action_timeout_ms),
            command.output(),
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

        let mut command = TokioCommand::new("ydotool");
        apply_socket_env(&mut command);
        command.args(["type", "--delay", "10", text]);

        let output = timeout(
            Duration::from_millis(self.config.per_method_timeout_ms),
            command.output(),
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
    async fn inject_text(
        &self,
        text: &str,
        _context: Option<&crate::types::InjectionContext>,
    ) -> InjectionResult<()> {
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
