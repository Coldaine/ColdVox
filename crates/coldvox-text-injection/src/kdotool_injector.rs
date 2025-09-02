use crate::types::{
    InjectionConfig, InjectionError, InjectionMethod, InjectionMetrics, TextInjector,
};
use anyhow::Result;
use async_trait::async_trait;
use std::process::Command;
use std::time::Duration;
use tokio::time::{error::Elapsed, timeout};
use tracing::{debug, error, info, warn};

/// Kdotool injector for KDE window activation/focus assistance
pub struct KdotoolInjector {
    config: InjectionConfig,
    metrics: InjectionMetrics,
    /// Whether kdotool is available on the system
    is_available: bool,
}

impl KdotoolInjector {
    /// Create a new kdotool injector
    pub fn new(config: InjectionConfig) -> Self {
        let is_available = Self::check_kdotool();

        Self {
            config,
            metrics: InjectionMetrics::default(),
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
        .map_err(|e| InjectionError::Process(e))?;

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
        let start = std::time::Instant::now();

        let output = timeout(
            Duration::from_millis(self.config.per_method_timeout_ms),
            tokio::process::Command::new("kdotool")
                .args(&["windowactivate", window_id])
                .output(),
        )
        .await
        .map_err(|_| InjectionError::Timeout(self.config.per_method_timeout_ms))?
        .map_err(|e| InjectionError::Process(e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(InjectionError::MethodFailed(format!(
                "kdotool windowactivate failed: {}",
                stderr
            )));
        }

        let duration = start.elapsed().as_millis() as u64;
        // TODO: Fix metrics - self.metrics.record_success requires &mut self
        info!("Successfully activated window {}", window_id);

        Ok(())
    }

    /// Focus a window by ID
    async fn focus_window(&self, window_id: &str) -> Result<(), InjectionError> {
        let start = std::time::Instant::now();

        let output = timeout(
            Duration::from_millis(self.config.per_method_timeout_ms),
            tokio::process::Command::new("kdotool")
                .args(&["windowfocus", window_id])
                .output(),
        )
        .await
        .map_err(|_| InjectionError::Timeout(self.config.per_method_timeout_ms))?
        .map_err(|e| InjectionError::Process(e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(InjectionError::MethodFailed(format!(
                "kdotool windowfocus failed: {}",
                stderr
            )));
        }

        let duration = start.elapsed().as_millis() as u64;
        // TODO: Fix metrics - self.metrics.record_success requires &mut self
        info!("Successfully focused window {}", window_id);

        Ok(())
    }
}

#[async_trait]
impl TextInjector for KdotoolInjector {
    fn name(&self) -> &'static str {
        "Kdotool"
    }

    fn is_available(&self) -> bool {
        self.is_available && self.config.allow_kdotool
    }

    async fn inject(&mut self, _text: &str) -> Result<(), InjectionError> {
        // Kdotool is only used for window activation/focus assistance
        // It doesn't actually inject text, so this method should not be called
        // directly for text injection
        Err(InjectionError::MethodUnavailable(
            "Kdotool is only for window activation/focus assistance".to_string(),
        ))
    }

    fn metrics(&self) -> &InjectionMetrics {
        &self.metrics
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
