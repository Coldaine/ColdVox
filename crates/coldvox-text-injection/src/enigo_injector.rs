use crate::types::{InjectionConfig, InjectionError, InjectionResult};
use crate::TextInjector;
use async_trait::async_trait;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use tracing::{debug, info};

/// Enigo injector for synthetic input
pub struct EnigoInjector {
    config: InjectionConfig,
    /// Whether enigo is available and can be used
    is_available: bool,
}

impl EnigoInjector {
    /// Create a new enigo injector
    pub fn new(config: InjectionConfig) -> Self {
        let is_available = Self::check_availability();

        Self {
            config,
            is_available,
        }
    }

    /// Check if enigo can be used (permissions, backend availability)
    fn check_availability() -> bool {
        // Check if we can create an Enigo instance
        // This will fail if we don't have the necessary permissions
        Enigo::new(&Settings::default()).is_ok()
    }

    /// Type text using enigo
    async fn type_text(&self, text: &str) -> Result<(), InjectionError> {
        let text_clone = text.to_string();

        let result = tokio::task::spawn_blocking(move || {
            let mut enigo = Enigo::new(&Settings::default()).map_err(|e| {
                InjectionError::MethodFailed(format!("Failed to create Enigo: {}", e))
            })?;

            // Type each character with a small delay
            for c in text_clone.chars() {
                match c {
                    ' ' => enigo.key(Key::Space, Direction::Click).map_err(|e| {
                        InjectionError::MethodFailed(format!("Failed to type space: {}", e))
                    })?,
                    '\n' => enigo.key(Key::Return, Direction::Click).map_err(|e| {
                        InjectionError::MethodFailed(format!("Failed to type enter: {}", e))
                    })?,
                    '\t' => enigo.key(Key::Tab, Direction::Click).map_err(|e| {
                        InjectionError::MethodFailed(format!("Failed to type tab: {}", e))
                    })?,
                    _ => {
                        // Use text method for all other characters
                        enigo.text(&c.to_string()).map_err(|e| {
                            InjectionError::MethodFailed(format!("Failed to type text: {}", e))
                        })?;
                    }
                }
            }

            Ok(())
        })
        .await;

        match result {
            Ok(Ok(())) => {
                info!("Successfully typed text via enigo ({} chars)", text.len());
                Ok(())
            }
            Ok(Err(e)) => Err(e),
            Err(_) => Err(InjectionError::Timeout(0)), // Spawn failed
        }
    }

    /// Trigger paste action using enigo (Ctrl+V)
    async fn trigger_paste(&self) -> Result<(), InjectionError> {
        let result = tokio::task::spawn_blocking(|| {
            let mut enigo = Enigo::new(&Settings::default()).map_err(|e| {
                InjectionError::MethodFailed(format!("Failed to create Enigo: {}", e))
            })?;

            // Press platform-appropriate paste shortcut
            #[cfg(target_os = "macos")]
            {
                enigo.key(Key::Meta, Direction::Press).map_err(|e| {
                    InjectionError::MethodFailed(format!("Failed to press Cmd: {}", e))
                })?;
                enigo
                    .key(Key::Unicode('v'), Direction::Click)
                    .map_err(|e| {
                        InjectionError::MethodFailed(format!("Failed to type 'v': {}", e))
                    })?;
                enigo.key(Key::Meta, Direction::Release).map_err(|e| {
                    InjectionError::MethodFailed(format!("Failed to release Cmd: {}", e))
                })?;
            }
            #[cfg(not(target_os = "macos"))]
            {
                enigo.key(Key::Control, Direction::Press).map_err(|e| {
                    InjectionError::MethodFailed(format!("Failed to press Ctrl: {}", e))
                })?;
                enigo
                    .key(Key::Unicode('v'), Direction::Click)
                    .map_err(|e| {
                        InjectionError::MethodFailed(format!("Failed to type 'v': {}", e))
                    })?;
                enigo.key(Key::Control, Direction::Release).map_err(|e| {
                    InjectionError::MethodFailed(format!("Failed to release Ctrl: {}", e))
                })?;
            }

            Ok(())
        })
        .await;

        match result {
            Ok(Ok(())) => {
                info!("Successfully triggered paste action via enigo");
                Ok(())
            }
            Ok(Err(e)) => Err(e),
            Err(_) => Err(InjectionError::Timeout(0)), // Spawn failed
        }
    }

    /// A test-only helper to directly call the private `type_text` method.
    #[cfg(test)]
    pub async fn type_text_directly(&self, text: &str) -> Result<(), InjectionError> {
        self.type_text(text).await
    }
}

#[async_trait]
impl TextInjector for EnigoInjector {
    fn backend_name(&self) -> &'static str {
        "Enigo"
    }

    async fn is_available(&self) -> bool {
        self.is_available && self.config.allow_enigo
    }

    async fn inject_text(&self, text: &str) -> InjectionResult<()> {
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

    fn backend_info(&self) -> Vec<(&'static str, String)> {
        vec![
            ("type", "synthetic input".to_string()),
            (
                "description",
                "Types text using system keyboard events via Enigo library".to_string(),
            ),
            ("platform", "Cross-platform".to_string()),
            (
                "requires",
                "System permissions for synthetic input".to_string(),
            ),
        ]
    }
}
