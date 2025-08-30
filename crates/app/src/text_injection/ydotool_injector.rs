use crate::text_injection::types::{InjectionConfig, InjectionError, InjectionMethod, InjectionMetrics, TextInjector};
use anyhow::Result;
use std::process::Command;
use std::time::Duration;
use tokio::time::{timeout, error::Elapsed};
use tracing::{debug, error, info, warn};

/// Ydotool injector for synthetic key events
pub struct YdotoolInjector {
    config: InjectionConfig,
    metrics: InjectionMetrics,
    /// Whether ydotool is available on the system
    is_available: bool,
}

impl YdotoolInjector {
    /// Create a new ydotool injector
    pub fn new(config: InjectionConfig) -> Self {
        let is_available = Self::check_ydotool();
        
        Self {
            config,
            metrics: InjectionMetrics::default(),
            is_available,
        }
    }

    /// Check if ydotool is available on the system
    fn check_ydotool() -> bool {
        // Check if binary exists
        let binary_exists = Command::new("which")
            .arg("ydotool")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        
        if !binary_exists {
            return false;
        }
        
        // Check if the ydotool socket exists (most reliable check)
        let user_id = std::env::var("UID").unwrap_or_else(|_| "1000".to_string());
        let socket_path = format!("/run/user/{}/.ydotool_socket", user_id);
        std::path::Path::new(&socket_path).exists()
    }

    /// Trigger paste action using ydotool (Ctrl+V)
    async fn trigger_paste(&self) -> Result<(), InjectionError> {
        let start = std::time::Instant::now();
        
        // Use tokio to run the command with timeout
        let output = timeout(
            Duration::from_millis(self.config.paste_action_timeout_ms),
            tokio::process::Command::new("ydotool")
                .args(&["key", "ctrl+v"])
                .output(),
        )
        .await
        .map_err(|_| InjectionError::Timeout(self.config.paste_action_timeout_ms))?
        .map_err(|e| InjectionError::Process(e))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(InjectionError::MethodFailed(format!("ydotool key failed: {}", stderr)));
        }
        
        let duration = start.elapsed().as_millis() as u64;
        self.metrics.record_success(InjectionMethod::YdoToolPaste, duration);
        info!("Successfully triggered paste action via ydotool");
        
        Ok(())
    }

    /// Type text directly using ydotool
    async fn type_text(&self, text: &str) -> Result<(), InjectionError> {
        let start = std::time::Instant::now();
        
        // Use tokio to run the command with timeout
        let output = timeout(
            Duration::from_millis(self.config.per_method_timeout_ms),
            tokio::process::Command::new("ydotool")
                .args(&["type", "--delay", "10", text])
                .output(),
        )
        .await
        .map_err(|_| InjectionError::Timeout(self.config.per_method_timeout_ms))?
        .map_err(|e| InjectionError::Process(e))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(InjectionError::MethodFailed(format!("ydotool type failed: {}", stderr)));
        }
        
        let duration = start.elapsed().as_millis() as u64;
        self.metrics.record_success(InjectionMethod::YdoToolPaste, duration);
        info!("Successfully typed text via ydotool ({} chars)", text.len());
        
        Ok(())
    }
}

impl TextInjector for YdotoolInjector {
    fn name(&self) -> &'static str {
        "Ydotool"
    }

    fn is_available(&self) -> bool {
        self.is_available && self.config.allow_ydotool
    }

    fn inject(&mut self, text: &str) -> Result<(), InjectionError> {
        if text.is_empty() {
            return Ok(());
        }

        // First try paste action (more reliable for batch text)
        match self.trigger_paste() {
            Ok(()) => Ok(()),
            Err(e) => {
                debug!("Paste action failed: {}", e);
                // Fall back to direct typing
                self.type_text(text)
            }
        }
    }

    fn metrics(&self) -> &InjectionMetrics {
        &self.metrics
    }
}