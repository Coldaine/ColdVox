use crate::clipboard_injector::ClipboardInjector;
use crate::types::{InjectionConfig, InjectionResult};
use crate::TextInjector;
use async_trait::async_trait;
use std::time::Duration;
use tracing::{debug, warn};

/// Combo injector that sets clipboard and then triggers AT-SPI paste action
/// NOTE: AT-SPI paste action not yet implemented for atspi 0.22
pub struct ComboClipboardAtspi {
    _config: InjectionConfig,
    clipboard_injector: ClipboardInjector,
}

impl ComboClipboardAtspi {
    /// Create a new combo clipboard+AT-SPI injector
    pub fn new(config: InjectionConfig) -> Self {
        Self {
            _config: config.clone(),
            clipboard_injector: ClipboardInjector::new(config),
        }
    }

    /// Check if this combo injector is available
    pub async fn is_available(&self) -> bool {
        // For now, just check if clipboard is available
        // AT-SPI paste action implementation pending
        self.clipboard_injector.is_available().await
    }
}

#[async_trait]
impl TextInjector for ComboClipboardAtspi {
    /// Get the name of this injector
    fn backend_name(&self) -> &'static str {
        "Clipboard+AT-SPI"
    }

    /// Check if this injector is available for use
    async fn is_available(&self) -> bool {
        self.is_available().await
    }

    /// Inject text using clipboard+AT-SPI paste
    async fn inject_text(&self, text: &str) -> InjectionResult<()> {
        // Step 1: Set clipboard content
        self.clipboard_injector.inject_text(text).await?;
        debug!("Clipboard set with {} chars", text.len());

        // Step 2: Wait a short time for clipboard to stabilize
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Step 3: Trigger paste action via AT-SPI
        // TODO: Implement AT-SPI paste action when atspi 0.22 API is clarified
        // For now, we can only set clipboard and rely on manual paste
        warn!("AT-SPI paste action not yet implemented for atspi 0.22");
        warn!("Text is in clipboard but automatic paste is not available");

        // Return success since clipboard was set successfully
        // User will need to manually paste (Ctrl+V) for now
        Ok(())
    }

    /// Get backend-specific configuration information
    fn backend_info(&self) -> Vec<(&'static str, String)> {
        vec![
            ("type", "combo clipboard+atspi".to_string()),
            (
                "description",
                "Sets clipboard content and triggers paste via AT-SPI".to_string(),
            ),
            ("platform", "Linux (Wayland + AT-SPI)".to_string()),
            (
                "status",
                "AT-SPI paste action not yet implemented".to_string(),
            ),
        ]
    }
}
