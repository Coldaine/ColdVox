use crate::types::{InjectionConfig, InjectionError, InjectionMetrics, TextInjector};
use async_trait::async_trait;
use tracing::debug;

#[cfg(feature = "mki")]
use mouse_keyboard_input::{Key, KeyboardControllable, VirtualDevice, VirtualKeyboard};
#[cfg(feature = "mki")]
use std::os::unix::fs::PermissionsExt;

/// Mouse-keyboard-input (MKI) injector for synthetic key events
pub struct MkiInjector {
    config: InjectionConfig,
    metrics: InjectionMetrics,
    /// Whether MKI is available and can be used
    is_available: bool,
}

impl MkiInjector {
    /// Create a new MKI injector
    pub fn new(config: InjectionConfig) -> Self {
        let is_available = Self::check_availability();

        Self {
            config,
            metrics: InjectionMetrics::default(),
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

    /// Type text using MKI  
    #[cfg(feature = "mki")]
    async fn type_text(&mut self, text: &str) -> Result<(), InjectionError> {
        let start = std::time::Instant::now();
        let text_clone = text.to_string();

        let result = tokio::task::spawn_blocking(move || {
            let mut keyboard = VirtualKeyboard::default().map_err(|e| {
                InjectionError::MethodFailed(format!("Failed to create keyboard: {}", e))
            })?;

            // Simple implementation - just send the text
            keyboard
                .key_sequence(&text_clone)
                .map_err(|e| InjectionError::MethodFailed(e.to_string()))?;

            Ok(())
        })
        .await;

        match result {
            Ok(Ok(())) => {
                let duration = start.elapsed().as_millis() as u64;
                // TODO: Fix metrics - self.metrics.record_success requires &mut self
                info!("Successfully typed text via MKI ({} chars)", text.len());
                Ok(())
            }
            Ok(Err(e)) => Err(e),
            Err(_) => Err(InjectionError::Timeout(0)), // Spawn failed
        }
    }

    /// Type text using MKI (feature disabled stub)
    #[cfg(not(feature = "mki"))]
    async fn type_text(&mut self, _text: &str) -> Result<(), InjectionError> {
        Err(InjectionError::MethodUnavailable(
            "MKI feature not enabled".to_string(),
        ))
    }

    /// Trigger paste action using MKI (Ctrl+V)
    #[cfg(feature = "mki")]
    async fn trigger_paste(&mut self) -> Result<(), InjectionError> {
        let start = std::time::Instant::now();

        let result = tokio::task::spawn_blocking(|| {
            let mut keyboard = VirtualKeyboard::default().map_err(|e| {
                InjectionError::MethodFailed(format!("Failed to create keyboard: {}", e))
            })?;

            // Press Ctrl+V - simplified for now
            keyboard
                .key_sequence("ctrl+v")
                .map_err(|e| InjectionError::MethodFailed(e.to_string()))?;

            Ok(())
        })
        .await;

        match result {
            Ok(Ok(())) => {
                let duration = start.elapsed().as_millis() as u64;
                // TODO: Fix metrics - self.metrics.record_success requires &mut self
                info!("Successfully triggered paste action via MKI");
                Ok(())
            }
            Ok(Err(e)) => Err(e),
            Err(_) => Err(InjectionError::Timeout(0)), // Spawn failed
        }
    }

    /// Trigger paste action using MKI (feature disabled stub)
    #[cfg(not(feature = "mki"))]
    async fn trigger_paste(&mut self) -> Result<(), InjectionError> {
        Err(InjectionError::MethodUnavailable(
            "MKI feature not enabled".to_string(),
        ))
    }
}

#[async_trait]
impl TextInjector for MkiInjector {
    fn name(&self) -> &'static str {
        "MKI"
    }

    fn is_available(&self) -> bool {
        self.is_available && self.config.allow_mki
    }

    async fn inject(&mut self, text: &str) -> Result<(), InjectionError> {
        if text.is_empty() {
            return Ok(());
        }

        // First try paste action (more reliable for batch text)
        // We need to set the clipboard first, but that's handled by the strategy manager
        // So we just trigger the paste
        match self.trigger_paste().await {
            Ok(()) => Ok(()),
            Err(e) => {
                debug!("Paste action failed: {}", e);
                // Fall back to direct typing
                self.type_text(text).await
            }
        }
    }

    fn metrics(&self) -> &InjectionMetrics {
        &self.metrics
    }
}
