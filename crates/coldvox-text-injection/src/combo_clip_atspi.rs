use crate::clipboard_injector::ClipboardInjector;
use crate::focus::{FocusStatus, FocusTracker};
use crate::types::{
    InjectionConfig, InjectionError, InjectionMethod, InjectionMetrics, TextInjector,
};
use async_trait::async_trait;
use atspi::action::Action;
use atspi::Accessible;
use std::time::Duration;
use tokio::time::{error::Elapsed, timeout};
use tracing::{debug, error, info, warn};

/// Combo injector that sets clipboard and then triggers AT-SPI paste action
pub struct ComboClipboardAtspiInjector {
    config: InjectionConfig,
    metrics: InjectionMetrics,
    clipboard_injector: ClipboardInjector,
    focus_tracker: FocusTracker,
}

impl ComboClipboardAtspiInjector {
    /// Create a new combo clipboard+AT-SPI injector
    pub fn new(config: InjectionConfig) -> Self {
        Self {
            config: config.clone(),
            metrics: InjectionMetrics::default(),
            clipboard_injector: ClipboardInjector::new(config.clone()),
            focus_tracker: FocusTracker::new(config),
        }
    }

    /// Trigger paste action on the focused element via AT-SPI2
    async fn trigger_paste_action(&self, accessible: &Accessible) -> Result<(), InjectionError> {
        let start = std::time::Instant::now();

        // Get Action interface
        let action = Action::new(accessible)
            .await
            .map_err(|e| InjectionError::Atspi(e))?;

        // Find paste action
        let n_actions = action
            .n_actions()
            .await
            .map_err(|e| InjectionError::Atspi(e))?;

        for i in 0..n_actions {
            let action_name = action
                .get_action_name(i)
                .await
                .map_err(|e| InjectionError::Atspi(e))?;

            let action_description = action
                .get_action_description(i)
                .await
                .map_err(|e| InjectionError::Atspi(e))?;

            // Check if this is a paste action (case-insensitive)
            if action_name.to_lowercase().contains("paste")
                || action_description.to_lowercase().contains("paste")
            {
                debug!(
                    "Found paste action: {} ({})",
                    action_name, action_description
                );

                // Execute the paste action
                action
                    .do_action(i)
                    .await
                    .map_err(|e| InjectionError::Atspi(e))?;

                let duration = start.elapsed().as_millis() as u64;
                // TODO: Fix metrics - self.metrics.record_success requires &mut self
                info!("Successfully triggered paste action via AT-SPI2");
                return Ok(());
            }
        }

        Err(InjectionError::MethodUnavailable(
            "No paste action found".to_string(),
        ))
    }
}

#[async_trait]
impl TextInjector for ComboClipboardAtspiInjector {
    fn name(&self) -> &'static str {
        "Clipboard+AT-SPI Paste"
    }

    fn is_available(&self) -> bool {
        // Available if both clipboard and AT-SPI are available
        self.clipboard_injector.is_available()
            && std::env::var("XDG_SESSION_TYPE")
                .map(|t| t == "wayland")
                .unwrap_or(false)
    }

    async fn inject(&mut self, text: &str) -> Result<(), InjectionError> {
        if text.is_empty() {
            return Ok(());
        }

        let start = std::time::Instant::now();

        // First, set the clipboard
        match self.clipboard_injector.inject(text) {
            Ok(()) => {
                debug!("Clipboard set successfully, proceeding to trigger paste action");
            }
            Err(e) => {
                let duration = start.elapsed().as_millis() as u64;
                self.metrics.record_failure(
                    InjectionMethod::ClipboardAndPaste,
                    duration,
                    e.to_string(),
                );
                return Err(InjectionError::MethodFailed(
                    "Failed to set clipboard".to_string(),
                ));
            }
        }

        // Small delay for clipboard to settle
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Get focus status
        let focus_status = match self.focus_tracker.get_focus_status().await {
            Ok(status) => status,
            Err(e) => {
                let duration = start.elapsed().as_millis() as u64;
                self.metrics.record_failure(
                    InjectionMethod::ClipboardAndPaste,
                    duration,
                    e.to_string(),
                );
                return Err(InjectionError::Other(e.to_string()));
            }
        };

        // Only proceed if we have a focused element
        if focus_status == FocusStatus::Unknown {
            debug!("Focus state unknown");
            return Err(InjectionError::Other("Unknown focus state".to_string()));
        }

        // Get focused element
        let focused = match self.focus_tracker.get_focused_element().await {
            Ok(Some(element)) => element,
            Ok(None) => {
                debug!("No focused element");
                return Err(InjectionError::Other("No focused element".to_string()));
            }
            Err(e) => {
                let duration = start.elapsed().as_millis() as u64;
                self.metrics.record_failure(
                    InjectionMethod::ClipboardAndPaste,
                    duration,
                    e.to_string(),
                );
                return Err(InjectionError::Other(e.to_string()));
            }
        };

        // Check if the element supports paste action
        if !self
            .focus_tracker
            .supports_paste_action(&focused)
            .await
            .unwrap_or(false)
        {
            debug!("Focused element does not support paste action");
            return Err(InjectionError::MethodUnavailable(
                "Focused element does not support paste action".to_string(),
            ));
        }

        // Trigger paste action
        let res = timeout(
            Duration::from_millis(self.config.paste_action_timeout_ms),
            self.trigger_paste_action(&focused),
        )
        .await;
        match res {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(_) => {
                let duration = start.elapsed().as_millis() as u64;
                self.metrics.record_failure(
                    InjectionMethod::ClipboardAndPaste,
                    duration,
                    format!("Timeout after {}ms", self.config.paste_action_timeout_ms),
                );
                Err(InjectionError::Timeout(self.config.paste_action_timeout_ms))
            }
        }
    }

    fn metrics(&self) -> &InjectionMetrics {
        &self.metrics
    }
}
