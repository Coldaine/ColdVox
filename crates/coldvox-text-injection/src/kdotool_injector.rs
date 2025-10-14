use crate::types::{InjectionConfig, InjectionError, InjectionResult};
use crate::TextInjector;
use async_trait::async_trait;
use serde::Deserialize;
use std::process::Command;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{debug, info, warn};

/// Details of the active window, retrieved via kdotool
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct WindowDetails {
    pub id: String,
    pub pid: i32,
    pub class: String,
}

/// Kdotool injector for KDE window activation/focus assistance
pub struct KdotoolInjector {
    config: InjectionConfig,
    /// Whether kdotool is available on the system
    is_available: bool,
}

impl KdotoolInjector {
    /// Create a new kdotool injector
    pub fn new(config: InjectionConfig) -> Self {
        let is_available = Self::check_kdotool();

        Self {
            config,
            is_available,
        }
    }

    /// Check if kdotool is available on the system
    fn check_kdotool() -> bool {
        Command::new("which")
            .arg("kdotool")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn binary_available(binary: &str) -> bool {
        Command::new("which")
            .arg(binary)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    #[cfg(test)]
    pub(crate) fn binary_available_for_tests(binary: &str) -> bool {
        Self::binary_available(binary)
    }

    /// Get the currently active window ID
    pub async fn get_active_window(&self) -> Result<String, InjectionError> {
        let output = timeout(
            Duration::from_millis(self.config.discovery_timeout_ms),
            tokio::process::Command::new("kdotool")
                .arg("getactivewindow")
                .output(),
        )
        .await
        .map_err(|_| InjectionError::Timeout(self.config.discovery_timeout_ms))?
        .map_err(|e| InjectionError::Process(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(InjectionError::MethodFailed(format!(
                "kdotool getactivewindow failed: {}",
                stderr
            )));
        }

        let window_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(window_id)
    }

    /// Get details (ID, PID, class) of the currently active window
    pub async fn get_active_window_details(&self) -> Result<WindowDetails, InjectionError> {
        // First, get the active window ID
        let window_id = self.get_active_window().await?;

        // Then, search for that window to get its details
        let search_output = timeout(
            Duration::from_millis(self.config.discovery_timeout_ms),
            tokio::process::Command::new("kdotool")
                .args(["search", "--class", ".*", &window_id])
                .output(),
        )
        .await
        .map_err(|_| InjectionError::Timeout(self.config.discovery_timeout_ms))?
        .map_err(|e| InjectionError::Process(e.to_string()))?;

        if !search_output.status.success() {
            let stderr = String::from_utf8_lossy(&search_output.stderr);
            warn!(
                "kdotool search failed for window id {}: {}",
                window_id, stderr
            );
            return Err(InjectionError::MethodFailed(format!(
                "kdotool search failed: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&search_output.stdout);
        debug!("kdotool search output: {}", stdout);

        // The output of `kdotool search` is expected to be a series of lines,
        // each being a JSON object representing a window. We need to find the
        // one that matches our active window ID.
        for line in stdout.lines() {
            if let Ok(mut details) = serde_json::from_str::<WindowDetails>(line) {
                // Ensure the ID is a plain string without extra quotes
                details.id = details.id.trim_matches('"').to_string();
                if details.id == window_id {
                    return Ok(details);
                }
            } else {
                warn!("Failed to parse kdotool search output line: {}", line);
            }
        }

        Err(InjectionError::MethodFailed(format!(
            "Could not find details for active window ID {}",
            window_id
        )))
    }

    /// Activate a window by ID
    async fn activate_window(&self, window_id: &str) -> Result<(), InjectionError> {
        let output = timeout(
            Duration::from_millis(self.config.per_method_timeout_ms),
            tokio::process::Command::new("kdotool")
                .args(["windowactivate", window_id])
                .output(),
        )
        .await
        .map_err(|_| InjectionError::Timeout(self.config.per_method_timeout_ms))?
        .map_err(|e| InjectionError::Process(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(InjectionError::MethodFailed(format!(
                "kdotool windowactivate failed: {}",
                stderr
            )));
        }

        info!("Successfully activated window {}", window_id);

        Ok(())
    }

    /// Focus a window by ID
    async fn focus_window(&self, window_id: &str) -> Result<(), InjectionError> {
        let output = timeout(
            Duration::from_millis(self.config.per_method_timeout_ms),
            tokio::process::Command::new("kdotool")
                .args(["windowfocus", window_id])
                .output(),
        )
        .await
        .map_err(|_| InjectionError::Timeout(self.config.per_method_timeout_ms))?
        .map_err(|e| InjectionError::Process(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(InjectionError::MethodFailed(format!(
                "kdotool windowfocus failed: {}",
                stderr
            )));
        }

        info!("Successfully focused window {}", window_id);

        Ok(())
    }
}

#[async_trait]
impl TextInjector for KdotoolInjector {
    fn backend_name(&self) -> &'static str {
        "Kdotool"
    }

    async fn is_available(&self) -> bool {
        self.is_available && self.config.allow_kdotool
    }

    async fn inject_text(
        &self,
        _text: &str,
        _context: Option<&crate::types::InjectionContext>,
    ) -> InjectionResult<()> {
        // Kdotool is only used for window activation/focus assistance
        // It doesn't actually inject text, so this method should not be called
        // directly for text injection
        Err(InjectionError::MethodUnavailable(
            "Kdotool is only for window activation/focus assistance".to_string(),
        ))
    }

    fn backend_info(&self) -> Vec<(&'static str, String)> {
        vec![
            ("type", "window management".to_string()),
            (
                "description",
                "Provides window activation and focus assistance using kdotool".to_string(),
            ),
            ("platform", "KDE/X11".to_string()),
            ("requires", "kdotool command line tool".to_string()),
        ]
    }
}

impl KdotoolInjector {
    /// Ensure the target window is active and focused
    pub async fn ensure_focus(&self, window_id: Option<&str>) -> Result<(), InjectionError> {
        let target_window = match window_id {
            Some(id) => id.to_string(),
            None => self.get_active_window().await?,
        };

        // First focus the window
        self.focus_window(&target_window).await?;

        // Then activate it
        self.activate_window(&target_window).await?;

        Ok(())
    }
}
