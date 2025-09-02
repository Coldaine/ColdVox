use crate::text_injection::types::{
    InjectionConfig, InjectionError, InjectionMethod, InjectionMetrics, TextInjector,
};
use async_trait::async_trait;
use mouse_keyboard_input::{Key, Keyboard, KeyboardControllable};
use std::time::Duration;
use tokio::time::{error::Elapsed, timeout};
use tracing::{debug, error, info, warn};

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
                let mode = metadata.permissions().mode();
                (mode & 0o060) == 0o060 || (mode & 0o006) == 0o006
            })
            .unwrap_or(false);

        in_input_group && uinput_accessible
    }

    /// Type text using MKI
    async fn type_text(&mut self, text: &str) -> Result<(), InjectionError> {
        let start = std::time::Instant::now();
        let text_clone = text.to_string();

        let result = tokio::task::spawn_blocking(move || {
            let mut keyboard = Keyboard::new().map_err(|e| {
                InjectionError::MethodFailed(format!("Failed to create keyboard: {}", e))
            })?;

            // Type each character with a small delay
            for c in text_clone.chars() {
                match c {
                    ' ' => keyboard
                        .key_click(Key::Space)
                        .map_err(|e| InjectionError::MethodFailed(e.to_string()))?,
                    '\n' => keyboard
                        .key_click(Key::Enter)
                        .map_err(|e| InjectionError::MethodFailed(e.to_string()))?,
                    '\t' => keyboard
                        .key_click(Key::Tab)
                        .map_err(|e| InjectionError::MethodFailed(e.to_string()))?,
                    _ => {
                        if c.is_ascii() {
                            keyboard
                                .key_sequence(&c.to_string())
                                .map_err(|e| InjectionError::MethodFailed(e.to_string()))?;
                        } else {
                            // For non-ASCII characters, we might need to use clipboard
                            return Err(InjectionError::MethodFailed(
                                "MKI doesn't support non-ASCII characters directly".to_string(),
                            ));
                        }
                    }
                }

                // Small delay between characters
                std::thread::sleep(Duration::from_millis(10));
            }

            Ok(())
        })
        .await;

        match result {
            Ok(Ok(())) => {
                let duration = start.elapsed().as_millis() as u64;
                self.metrics
                    .record_success(InjectionMethod::UinputKeys, duration);
                info!("Successfully typed text via MKI ({} chars)", text.len());
                Ok(())
            }
            Ok(Err(e)) => Err(e),
            Err(_) => Err(InjectionError::Timeout(0)), // Spawn failed
        }
    }

    /// Trigger paste action using MKI (Ctrl+V)
    async fn trigger_paste(&mut self) -> Result<(), InjectionError> {
        let start = std::time::Instant::now();

        let result = tokio::task::spawn_blocking(|| {
            let mut keyboard = Keyboard::new().map_err(|e| {
                InjectionError::MethodFailed(format!("Failed to create keyboard: {}", e))
            })?;

            // Press Ctrl+V
            keyboard
                .key_down(Key::Control)
                .map_err(|e| InjectionError::MethodFailed(e.to_string()))?;
            keyboard
                .key_click(Key::V)
                .map_err(|e| InjectionError::MethodFailed(e.to_string()))?;
            keyboard
                .key_up(Key::Control)
                .map_err(|e| InjectionError::MethodFailed(e.to_string()))?;

            Ok(())
        })
        .await;

        match result {
            Ok(Ok(())) => {
                let duration = start.elapsed().as_millis() as u64;
                self.metrics
                    .record_success(InjectionMethod::UinputKeys, duration);
                info!("Successfully triggered paste action via MKI");
                Ok(())
            }
            Ok(Err(e)) => Err(e),
            Err(_) => Err(InjectionError::Timeout(0)), // Spawn failed
        }
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
