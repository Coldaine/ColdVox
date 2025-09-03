use crate::types::{InjectionConfig, InjectionError, InjectionResult};
use crate::TextInjector;
use async_trait::async_trait;
use std::process::Command;
use std::time::Duration;
use tokio::time::timeout;
use tracing::info;

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

    /// Get the currently active window ID
    async fn get_active_window(&self) -> Result<String, InjectionError> {
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

    async fn inject_text(&self, _text: &str) -> InjectionResult<()> {
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
