use crate::text_injection::focus::{FocusTracker, FocusStatus};
use crate::text_injection::types::{InjectionConfig, InjectionError, InjectionMethod, InjectionMetrics};
use atspi::action::Action;
use atspi::editable_text::EditableText;
use atspi::Accessible;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};
use async_trait::async_trait;

/// AT-SPI2 injector for direct text insertion
pub struct AtspiInjector {
    config: InjectionConfig,
    metrics: InjectionMetrics,
    focus_tracker: FocusTracker,
}

impl AtspiInjector {
    /// Create a new AT-SPI2 injector
    pub fn new(config: InjectionConfig) -> Self {
        Self {
            config: config.clone(),
            metrics: InjectionMetrics::default(),
            focus_tracker: FocusTracker::new(config),
        }
    }

    /// Insert text directly into the focused element using EditableText interface
    async fn insert_text_direct(&self, text: &str, accessible: &Accessible) -> Result<(), InjectionError> {
        let start = std::time::Instant::now();
        
        // Get EditableText interface
        let editable_text = EditableText::new(accessible).await
            .map_err(|e| InjectionError::Atspi(e))?;
        
        // Get current text length to insert at end
        let text_length = editable_text.get_text(0, -1).await
            .map_err(|e| InjectionError::Atspi(e))?
            .len() as i32;
        
        // Insert text at the end
        editable_text.insert_text(text_length, text).await
            .map_err(|e| InjectionError::Atspi(e))?;
        
        let duration = start.elapsed().as_millis() as u64;
        self.metrics.record_success(InjectionMethod::AtspiInsert, duration);
        info!("Successfully inserted text via AT-SPI2 EditableText ({} chars)", text.len());
        
        Ok(())
    }

    /// Trigger paste action on the focused element
    async fn trigger_paste_action(&self, accessible: &Accessible) -> Result<(), InjectionError> {
        let start = std::time::Instant::now();
        
        // Get Action interface
        let action = Action::new(accessible).await
            .map_err(|e| InjectionError::Atspi(e))?;
        
        // Find paste action
        let n_actions = action.n_actions().await
            .map_err(|e| InjectionError::Atspi(e))?;
        
        for i in 0..n_actions {
            let action_name = action.get_action_name(i).await
                .map_err(|e| InjectionError::Atspi(e))?;
            
            let action_description = action.get_action_description(i).await
                .map_err(|e| InjectionError::Atspi(e))?;
            
            // Check if this is a paste action (case-insensitive)
            if action_name.to_lowercase().contains("paste") || 
               action_description.to_lowercase().contains("paste") {
                debug!("Found paste action: {} ({})", action_name, action_description);
                
                // Execute the paste action
                action.do_action(i).await
                    .map_err(|e| InjectionError::Atspi(e))?;
                
                let duration = start.elapsed().as_millis() as u64;
                self.metrics.record_success(InjectionMethod::AtspiInsert, duration);
                info!("Successfully triggered paste action via AT-SPI2");
                return Ok(());
            }
        }
        
        Err(InjectionError::MethodUnavailable("No paste action found".to_string()))
    }
}

#[async_trait]
impl super::types::TextInjector for AtspiInjector {
    fn name(&self) -> &'static str {
        "AT-SPI2"
    }

    fn is_available(&self) -> bool {
        // AT-SPI2 should be available on KDE/Wayland
        std::env::var("XDG_SESSION_TYPE").map(|t| t == "wayland").unwrap_or(false)
    }

    async fn inject(&mut self, text: &str) -> Result<(), InjectionError> {
        if text.is_empty() {
            return Ok(());
        }

        let start = std::time::Instant::now();
        
        // Get focus status
        let focus_status = self.focus_tracker.get_focus_status().await.map_err(|e| {
            let duration = start.elapsed().as_millis() as u64;
            self.metrics.record_failure(InjectionMethod::AtspiInsert, duration, e.to_string());
            e
        })?;

        // Only proceed if we have a confirmed editable field or unknown focus (if allowed)
        if focus_status == FocusStatus::NonEditable {
            // We can't insert text directly, but might be able to paste
            debug!("Focused element is not editable, skipping direct insertion");
            return Err(InjectionError::MethodUnavailable("Focused element not editable".to_string()));
        }

        if focus_status == FocusStatus::Unknown && !self.config.inject_on_unknown_focus {
            debug!("Focus state unknown and injection on unknown focus disabled");
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
                self.metrics.record_failure(InjectionMethod::AtspiInsert, duration, e.to_string());
                return Err(InjectionError::Other(e.to_string()));
            }
        };

        // Try direct insertion first
        let direct_res = timeout(
            Duration::from_millis(self.config.per_method_timeout_ms),
            self.insert_text_direct(text, &focused),
        ).await;
        match direct_res {
            Ok(Ok(())) => return Ok(()),
            Ok(Err(e)) => {
                debug!("Direct insertion failed: {}", e);
            }
            Err(_) => {
                let duration = start.elapsed().as_millis() as u64;
                self.metrics.record_failure(
                    InjectionMethod::AtspiInsert,
                    duration,
                    format!("Timeout after {}ms", self.config.per_method_timeout_ms)
                );
                return Err(InjectionError::Timeout(self.config.per_method_timeout_ms));
            }
        }

        // If direct insertion failed, try paste action if the element supports it
        if self.focus_tracker.supports_paste_action(&focused).await.unwrap_or(false) {
            let paste_res = timeout(
                Duration::from_millis(self.config.paste_action_timeout_ms),
                self.trigger_paste_action(&focused),
            ).await;
            match paste_res {
                Ok(Ok(())) => return Ok(()),
                Ok(Err(e)) => {
                    debug!("Paste action failed: {}", e);
                }
                Err(_) => {
                    let duration = start.elapsed().as_millis() as u64;
                    self.metrics.record_failure(
                        InjectionMethod::AtspiInsert,
                        duration,
                        format!("Timeout after {}ms", self.config.paste_action_timeout_ms)
                    );
                    return Err(InjectionError::Timeout(self.config.paste_action_timeout_ms));
                }
            }
        }

        // If we get here, both methods failed
        let duration = start.elapsed().as_millis() as u64;
        self.metrics.record_failure(
            InjectionMethod::AtspiInsert,
            duration,
            "Both direct insertion and paste action failed".to_string()
        );
        Err(InjectionError::MethodFailed("AT-SPI2 injection failed".to_string()))
    }

    fn metrics(&self) -> &InjectionMetrics {
        &self.metrics
    }
}