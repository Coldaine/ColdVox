use crate::constants::PER_BACKEND_SOFT_TIMEOUT_MS;
use crate::error::InjectionError;
use crate::outcome::InjectionOutcome;
use crate::probe::BackendId;
use crate::types::InjectionConfig;
use crate::TextInjector;
use async_trait::async_trait;
use std::process::Command;
use std::time::{Duration, Instant};
use tokio::time::timeout;
use tracing::{debug, info, warn};

/// Ydotool injector for synthetic key events
pub struct YdotoolInjector {
    config: InjectionConfig,
}

impl YdotoolInjector {
    /// Create a new ydotool injector
    pub fn new(config: InjectionConfig) -> Self {
        Self { config }
    }

    /// Trigger paste action using ydotool (Ctrl+V)
    async fn trigger_paste(&self) -> Result<(), InjectionError> {
        let cmd_fut = tokio::process::Command::new("ydotool")
            .args(["key", "ctrl+v"])
            .output();

        let output = timeout(
            Duration::from_millis(PER_BACKEND_SOFT_TIMEOUT_MS / 2),
            cmd_fut,
        )
        .await
        .map_err(|_| InjectionError::Timeout {
            backend: BackendId::Ydotool,
            phase: "paste",
            elapsed_ms: (PER_BACKEND_SOFT_TIMEOUT_MS / 2) as u32,
        })?
        .map_err(|e| InjectionError::Io {
            backend: BackendId::Ydotool,
            msg: e.to_string(),
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(InjectionError::Transient {
                reason: "ydotool paste command failed",
                retryable: false,
            });
        }
        Ok(())
    }

    /// Type text directly using ydotool
    async fn type_text(&self, text: &str) -> Result<(), InjectionError> {
        let cmd_fut = tokio::process::Command::new("ydotool")
            .args(["type", "--delay", "10", text])
            .output();

        let output = timeout(
            Duration::from_millis(PER_BACKEND_SOFT_TIMEOUT_MS),
            cmd_fut,
        )
        .await
        .map_err(|_| InjectionError::Timeout {
            backend: BackendId::Ydotool,
            phase: "type",
            elapsed_ms: PER_BACKEND_SOFT_TIMEOUT_MS as u32,
        })?
        .map_err(|e| InjectionError::Io {
            backend: BackendId::Ydotool,
            msg: e.to_string(),
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(InjectionError::Transient {
                reason: "ydotool type command failed",
                retryable: false,
            });
        }
        Ok(())
    }
}

#[async_trait]
impl TextInjector for YdotoolInjector {
    fn backend_id(&self) -> BackendId {
        BackendId::Ydotool
    }

    async fn is_available(&self) -> bool {
        if !self.config.allow_ydotool {
            return false;
        }
        // A simple check. The full probe is more thorough.
        Command::new("which")
            .arg("ydotool")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    async fn inject_text(&self, text: &str) -> Result<InjectionOutcome, InjectionError> {
        if text.is_empty() {
            return Ok(InjectionOutcome {
                backend: self.backend_id(),
                latency_ms: 0,
                degraded: false,
            });
        }

        let start_time = Instant::now();

        // The primary method for ydotool is typing, as it's more direct.
        // A paste would require setting the clipboard first, which is another backend's job.
        self.type_text(text).await?;

        Ok(InjectionOutcome {
            backend: self.backend_id(),
            latency_ms: start_time.elapsed().as_millis() as u32,
            degraded: false,
        })
    }
}
