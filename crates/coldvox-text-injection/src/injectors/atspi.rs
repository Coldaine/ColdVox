//! AT-SPI Text Injector Implementation
//!
//! This module provides an AT-SPI-based text injector that supports both direct text insertion
//! and clipboard-based paste operations. It integrates with the existing AT-SPI infrastructure
//! while providing the new TextInjector trait interface.

use crate::confirm::{create_confirmation_context, ConfirmationContext};
use crate::log_throttle::log_atspi_connection_failure;
use crate::logging::utils;
use crate::types::{
    InjectionConfig, InjectionContext, InjectionMethod, InjectionMode, InjectionResult,
};
use crate::TextInjector;
use async_trait::async_trait;
use coldvox_foundation::error::InjectionError;
use std::time::Instant;
use tracing::{debug, trace, warn};

/// AT-SPI Text Injector with support for both insert and paste operations
pub struct AtspiInjector {
    /// Configuration for injection
    config: InjectionConfig,
    /// Confirmation context for injection verification
    #[allow(dead_code)]
    confirmation_context: ConfirmationContext,
}

impl AtspiInjector {
    /// Create a new AT-SPI injector with the given configuration
    pub fn new(config: InjectionConfig) -> Self {
        let confirmation_context = create_confirmation_context(config.clone());
        Self {
            config,
            confirmation_context,
        }
    }

    /// Insert text directly using AT-SPI EditableText interface
    pub async fn insert_text(&self, text: &str, context: &InjectionContext) -> InjectionResult<()> {
        let start_time = Instant::now();

        trace!("AT-SPI insert_text starting for {} chars", text.len());
        // Fast-path: nothing to do for empty text. This keeps tests simple and
        // avoids attempting AT-SPI operations when callers simply want a no-op.
        if text.is_empty() {
            debug!("insert_text called with empty text; nothing to do");
            return Ok(());
        }

        #[cfg(feature = "atspi")]
        {
            use atspi::{
                connection::AccessibilityConnection, proxy::collection::CollectionProxy,
                proxy::editable_text::EditableTextProxy, proxy::text::TextProxy, Interface,
                MatchType, ObjectMatchRule, SortOrder, State,
            };
            use tokio::time;

            let per_method_timeout = self.config.per_method_timeout();

            // Connect to AT-SPI
            let conn = time::timeout(per_method_timeout, AccessibilityConnection::new())
                .await
                .map_err(|_| InjectionError::Timeout(per_method_timeout.as_millis() as u64))?
                .map_err(|e| {
                    log_atspi_connection_failure(&e.to_string());
                    InjectionError::Other(format!("AT-SPI connect failed: {e}"))
                })?;

            let zbus_conn = conn.connection();
            trace!("AT-SPI connection established for insert_text");

            // Find focused element (pre-warming not currently implemented for AT-SPI objects)
            let obj_ref = {
                // Find focused element
                let collection_fut = CollectionProxy::builder(zbus_conn)
                    .destination("org.a11y.atspi.Registry")
                    .map_err(|e| {
                        InjectionError::Other(format!("CollectionProxy destination failed: {e}"))
                    })?
                    .path("/org/a11y/atspi/accessible/root")
                    .map_err(|e| {
                        InjectionError::Other(format!("CollectionProxy path failed: {e}"))
                    })?
                    .build();

                let collection = time::timeout(per_method_timeout, collection_fut)
                    .await
                    .map_err(|_| InjectionError::Timeout(per_method_timeout.as_millis() as u64))?
                    .map_err(|e| {
                        InjectionError::Other(format!("CollectionProxy build failed: {e}"))
                    })?;

                let mut rule = ObjectMatchRule::default();
                rule.states = State::Focused.into();
                rule.states_mt = MatchType::All;
                rule.ifaces = Interface::EditableText.into();
                rule.ifaces_mt = MatchType::All;

                let get_matches = collection.get_matches(rule, SortOrder::Canonical, 1, false);
                let mut matches = time::timeout(per_method_timeout, get_matches)
                    .await
                    .map_err(|_| InjectionError::Timeout(per_method_timeout.as_millis() as u64))?
                    .map_err(|e| {
                        InjectionError::Other(format!("Collection.get_matches failed: {e}"))
                    })?;

                if matches.is_empty() {
                    debug!("No focused EditableText found for insert_text");
                    return Err(InjectionError::NoEditableFocus);
                }

                matches.pop().unwrap()
            };

            debug!(
                "Found editable element at path: {:?} in app: {:?}",
                obj_ref.path(), obj_ref.name()
            );

            // Get EditableText proxy
            let editable_fut = EditableTextProxy::builder(zbus_conn)
                .destination(obj_ref.name().ok_or_else(|| InjectionError::Other("No bus name".into()))?.clone())
                .map_err(|e| {
                    InjectionError::Other(format!("EditableTextProxy destination failed: {e}"))
                })?
                .path(obj_ref.path().clone())
                .map_err(|e| InjectionError::Other(format!("EditableTextProxy path failed: {e}")))?
                .build();

            let editable = time::timeout(per_method_timeout, editable_fut)
                .await
                .map_err(|_| InjectionError::Timeout(per_method_timeout.as_millis() as u64))?
                .map_err(|e| {
                    InjectionError::Other(format!("EditableTextProxy build failed: {e}"))
                })?;

            // Get Text proxy to determine caret position
            let text_iface_fut = TextProxy::builder(zbus_conn)
                .destination(obj_ref.name().ok_or_else(|| InjectionError::Other("No bus name".into()))?.clone())
                .map_err(|e| InjectionError::Other(format!("TextProxy destination failed: {e}")))?
                .path(obj_ref.path().clone())
                .map_err(|e| InjectionError::Other(format!("TextProxy path failed: {e}")))?
                .build();

            let text_iface = time::timeout(per_method_timeout, text_iface_fut)
                .await
                .map_err(|_| InjectionError::Timeout(per_method_timeout.as_millis() as u64))?
                .map_err(|e| InjectionError::Other(format!("TextProxy build failed: {e}")))?;

            // Get current caret position
            let caret_fut = text_iface.caret_offset();
            let caret = time::timeout(per_method_timeout, caret_fut)
                .await
                .map_err(|_| InjectionError::Timeout(per_method_timeout.as_millis() as u64))?
                .map_err(|e| {
                    warn!("Failed to get caret offset from {:?}: {}", obj_ref.path(), e);
                    InjectionError::Other(format!("Text.caret_offset failed: {e}"))
                })?;

            trace!("Current caret position: {}", caret);

            // Insert text at caret position
            let insert_fut = editable.insert_text(caret, text, text.chars().count() as i32);
            time::timeout(per_method_timeout, insert_fut)
                .await
                .map_err(|_| InjectionError::Timeout(per_method_timeout.as_millis() as u64))?
                .map_err(|e| {
                    warn!(
                        "Failed to insert text at position {} in {:?}: {}",
                        caret, obj_ref.path(), e
                    );
                    InjectionError::Other(format!("EditableText.insert_text failed: {e}"))
                })?;

            let elapsed = start_time.elapsed();

            // Log successful insertion
            utils::log_injection_success(
                InjectionMethod::AtspiInsert,
                text,
                elapsed,
                self.config.redact_logs,
            );

            debug!(
                "Successfully inserted {} chars via AT-SPI to {:?} in {}ms",
                text.len(),
                obj_ref.name(),
                elapsed.as_millis()
            );

            // Confirm insertion if needed
            if let Some(ref target) = context.target_app {
                let window = context.window_id.as_deref().unwrap_or("unknown");
                if let Ok(result) = self
                    .confirmation_context
                    .confirm_injection(target, text, window)
                    .await
                {
                    match result {
                        crate::confirm::ConfirmationResult::Success => {
                            debug!("AT-SPI insertion confirmed via text change event");
                        }
                        _ => {
                            debug!("AT-SPI insertion confirmation failed or timed out");
                        }
                    }
                }
            }

            Ok(())
        }

        #[cfg(not(feature = "atspi"))]
        {
            warn!("AT-SPI injector compiled without 'atspi' feature");
            Err(InjectionError::Other(
                "AT-SPI feature is disabled at compile time".to_string(),
            ))
        }
    }

    /// Paste text using AT-SPI clipboard operations
    pub async fn paste_text(&self, text: &str, context: &InjectionContext) -> InjectionResult<()> {
        let start_time = Instant::now();

        trace!("AT-SPI paste_text starting for {} chars", text.len());
        // Fast-path: nothing to do for empty text. Matches insert_text behavior.
        if text.is_empty() {
            debug!("paste_text called with empty text; nothing to do");
            return Ok(());
        }

        #[cfg(feature = "atspi")]
        {
            use atspi::{
                connection::AccessibilityConnection, proxy::action::ActionProxy,
                proxy::collection::CollectionProxy, Interface, MatchType, ObjectMatchRule,
                SortOrder, State,
            };
            use tokio::time;

            let per_method_timeout = self.config.per_method_timeout();

            // First, set the clipboard content
            self.set_clipboard_content(text).await?;

            // Connect to AT-SPI
            let conn = time::timeout(per_method_timeout, AccessibilityConnection::new())
                .await
                .map_err(|_| InjectionError::Timeout(per_method_timeout.as_millis() as u64))?
                .map_err(|e| {
                    log_atspi_connection_failure(&e.to_string());
                    InjectionError::Other(format!("AT-SPI connect failed: {e}"))
                })?;

            let zbus_conn = conn.connection();
            trace!("AT-SPI connection established for paste_text");

            // Find focused element (pre-warming not currently implemented for AT-SPI objects)
            let obj_ref = {
                // Find focused element
                let collection_fut = CollectionProxy::builder(zbus_conn)
                    .destination("org.a11y.atspi.Registry")
                    .map_err(|e| {
                        InjectionError::Other(format!("CollectionProxy destination failed: {e}"))
                    })?
                    .path("/org/a11y/atspi/accessible/root")
                    .map_err(|e| {
                        InjectionError::Other(format!("CollectionProxy path failed: {e}"))
                    })?
                    .build();

                let collection = time::timeout(per_method_timeout, collection_fut)
                    .await
                    .map_err(|_| InjectionError::Timeout(per_method_timeout.as_millis() as u64))?
                    .map_err(|e| {
                        InjectionError::Other(format!("CollectionProxy build failed: {e}"))
                    })?;

                let mut rule = ObjectMatchRule::default();
                rule.states = State::Focused.into();
                rule.states_mt = MatchType::All;
                rule.ifaces = Interface::EditableText.into();
                rule.ifaces_mt = MatchType::All;

                let get_matches = collection.get_matches(rule, SortOrder::Canonical, 1, false);
                let mut matches = time::timeout(per_method_timeout, get_matches)
                    .await
                    .map_err(|_| InjectionError::Timeout(per_method_timeout.as_millis() as u64))?
                    .map_err(|e| {
                        InjectionError::Other(format!("Collection.get_matches failed: {e}"))
                    })?;

                if matches.is_empty() {
                    debug!("No focused EditableText found for paste_text");
                    return Err(InjectionError::NoEditableFocus);
                }

                matches.pop().unwrap()
            };

            debug!(
                "Found editable element at path: {:?} in app: {:?}",
                obj_ref.path(), obj_ref.name()
            );

            // Get Action proxy to trigger paste action
            let action_fut = ActionProxy::builder(zbus_conn)
                .destination(obj_ref.name().ok_or_else(|| InjectionError::Other("No bus name".into()))?.clone())
                .map_err(|e| InjectionError::Other(format!("ActionProxy destination failed: {e}")))?
                .path(obj_ref.path().clone())
                .map_err(|e| InjectionError::Other(format!("ActionProxy path failed: {e}")))?
                .build();

            let action = time::timeout(per_method_timeout, action_fut)
                .await
                .map_err(|_| InjectionError::Timeout(per_method_timeout.as_millis() as u64))?
                .map_err(|e| InjectionError::Other(format!("ActionProxy build failed: {e}")))?;

            // Try to find and execute a paste action
            let actions_fut = action.get_actions();
            let actions = time::timeout(per_method_timeout, actions_fut)
                .await
                .map_err(|_| InjectionError::Timeout(per_method_timeout.as_millis() as u64))?
                .map_err(|e| {
                    warn!("Failed to get actions from {:?}: {}", obj_ref.path(), e);
                    InjectionError::Other(format!("Action.get_actions failed: {e}"))
                })?;

            // Look for a paste action
            let mut paste_found = false;
            for (i, action_obj) in actions.iter().enumerate() {
                // AT-SPI Action objects have a name field that contains the action name
                let action_name = &action_obj.name;
                if action_name.to_lowercase().contains("paste") {
                    debug!("Found paste action: {} at index {}", action_name, i);
                    let do_action_fut = action.do_action(i as i32);
                    time::timeout(per_method_timeout, do_action_fut)
                        .await
                        .map_err(
                            |_| InjectionError::Timeout(per_method_timeout.as_millis() as u64),
                        )?
                        .map_err(|e| {
                            warn!("Failed to execute paste action {}: {}", i, e);
                            InjectionError::Other(format!("Action.do_action failed: {e}"))
                        })?;
                    paste_found = true;
                    break;
                }
            }

            if !paste_found {
                // Fallback: try to trigger paste using key events if available
                debug!("No paste action found, trying key event fallback");
                self.trigger_paste_key_event().await?;
            }

            let elapsed = start_time.elapsed();

            // Log successful paste
            utils::log_injection_success(
                InjectionMethod::AtspiInsert,
                text,
                elapsed,
                self.config.redact_logs,
            );

            debug!(
                "Successfully pasted {} chars via AT-SPI to {:?} in {}ms",
                text.len(),
                obj_ref.name(),
                elapsed.as_millis()
            );

            // Confirm paste if needed
            if let Some(ref target) = context.target_app {
                let window = context.window_id.as_deref().unwrap_or("unknown");
                if let Ok(result) = self
                    .confirmation_context
                    .confirm_injection(target, text, window)
                    .await
                {
                    match result {
                        crate::confirm::ConfirmationResult::Success => {
                            debug!("AT-SPI paste confirmed via text change event");
                        }
                        _ => {
                            debug!("AT-SPI paste confirmation failed or timed out");
                        }
                    }
                }
            }

            Ok(())
        }

        #[cfg(not(feature = "atspi"))]
        {
            warn!("AT-SPI injector compiled without 'atspi' feature");
            Err(InjectionError::Other(
                "AT-SPI feature is disabled at compile time".to_string(),
            ))
        }
    }

    /// Set clipboard content for paste operations
    #[allow(dead_code)]
    async fn set_clipboard_content(&self, text: &str) -> InjectionResult<()> {
        #[cfg(feature = "wl_clipboard")]
        {
            use wl_clipboard_rs::copy::{MimeType, Options, Source};

            let source = Source::Bytes(text.as_bytes().to_vec().into());
            let opts = Options::new();

            opts.copy(source, MimeType::Text)
                .map_err(|e| InjectionError::Clipboard(e.to_string()))?;

            debug!(
                "Set clipboard content ({} chars) for AT-SPI paste",
                text.len()
            );
            Ok(())
        }

        #[cfg(not(feature = "wl_clipboard"))]
        {
            // Fallback to system clipboard if wl_clipboard is not available
            use std::process::Command;

            let output = Command::new("wl-copy").arg(text).output().map_err(|e| {
                InjectionError::Process(format!("Failed to execute wl-copy: {}", e))
            })?;

            if !output.status.success() {
                return Err(InjectionError::Process(
                    "wl-copy command failed".to_string(),
                ));
            }

            debug!(
                "Set clipboard content via wl-copy ({} chars) for AT-SPI paste",
                text.len()
            );
            Ok(())
        }
    }

    /// Trigger paste using key events as a fallback
    #[allow(dead_code)]
    async fn trigger_paste_key_event(&self) -> InjectionResult<()> {
        #[cfg(feature = "enigo")]
        {
            use enigo::{Direction, Enigo, Key, Keyboard, Settings};

            let result = tokio::task::spawn_blocking(move || {
                let mut enigo = Enigo::new(&Settings::default()).map_err(|e| {
                    InjectionError::MethodFailed(format!("Failed to create Enigo: {}", e))
                })?;

                // Press Ctrl+V for paste
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

                Ok(())
            })
            .await;

            match result {
                Ok(Ok(())) => {
                    debug!("Successfully triggered paste via key events");
                    Ok(())
                }
                Ok(Err(e)) => Err(e),
                Err(_) => Err(InjectionError::Timeout(0)), // Spawn failed
            }
        }

        #[cfg(not(feature = "enigo"))]
        {
            Err(InjectionError::MethodUnavailable(
                "No key event support available for paste fallback".to_string(),
            ))
        }
    }

    /// Main injection method that delegates to insert_text or paste_text
    /// Respects mode_override in context if present
    pub async fn inject(&self, text: &str, context: &InjectionContext) -> InjectionResult<()> {
        if text.is_empty() {
            return Ok(());
        }

        // Determine injection method based on context override or configuration
        let use_paste = if let Some(mode_override) = context.mode_override {
            match mode_override {
                InjectionMode::Paste => true,
                InjectionMode::Keystroke => false,
            }
        } else {
            // Fall back to config-based decision
            match self.config.injection_mode.as_str() {
                "paste" => true,
                "keystroke" => false,
                "auto" => text.len() > self.config.paste_chunk_chars as usize,
                _ => text.len() > self.config.paste_chunk_chars as usize, // Default to auto
            }
        };

        if use_paste {
            self.paste_text(text, context).await
        } else {
            self.insert_text(text, context).await
        }
    }

    /// Get the injection method used by this injector
    pub fn method(&self) -> InjectionMethod {
        InjectionMethod::AtspiInsert
    }
}

#[async_trait]
impl TextInjector for AtspiInjector {
    fn backend_name(&self) -> &'static str {
        "atspi-injector"
    }

    fn backend_info(&self) -> Vec<(&'static str, String)> {
        vec![
            ("type", "AT-SPI accessibility".to_string()),
            (
                "description",
                "Injects text directly or via paste using AT-SPI accessibility API".to_string(),
            ),
            ("platform", "Linux".to_string()),
            ("requires", "AT-SPI accessibility service".to_string()),
        ]
    }

    async fn is_available(&self) -> bool {
        #[cfg(feature = "atspi")]
        {
            use atspi::connection::AccessibilityConnection;
            use tokio::time;

            let timeout_duration = self.config.per_method_timeout();

            let availability_check = async { AccessibilityConnection::new().await.is_ok() };

            match time::timeout(timeout_duration, availability_check).await {
                Ok(is_ok) => {
                    if is_ok {
                        debug!("AT-SPI is available");
                    } else {
                        debug!("AT-SPI connection failed");
                    }
                    is_ok
                }
                Err(_) => {
                    warn!(
                        "AT-SPI availability check timed out after {}ms",
                        timeout_duration.as_millis()
                    );
                    false
                }
            }
        }
        #[cfg(not(feature = "atspi"))]
        {
            warn!("AT-SPI feature disabled; injector unavailable");
            false
        }
    }

    async fn inject_text(
        &self,
        text: &str,
        context: Option<&InjectionContext>,
    ) -> InjectionResult<()> {
        // Use provided context or create default
        let default_context = InjectionContext::default();
        let ctx = context.unwrap_or(&default_context);
        self.inject(text, ctx).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_atspi_injector_creation() {
        let config = InjectionConfig::default();
        let injector = AtspiInjector::new(config);

        assert_eq!(injector.backend_name(), "atspi-injector");
        assert_eq!(injector.method(), InjectionMethod::AtspiInsert);
    }

    #[tokio::test]
    async fn test_atspi_injector_availability() {
        let config = InjectionConfig::default();
        let injector = AtspiInjector::new(config);

        // Just ensure the method doesn't panic
        let _available = injector.is_available().await;
    }

    #[tokio::test]
    async fn test_context_default() {
        let context = InjectionContext::default();

        assert!(context.atspi_focused_node_path.is_none());
        assert!(context.target_app.is_none());
        assert!(context.window_id.is_none());
    }

    #[tokio::test]
    async fn test_empty_text_handling() {
        let config = InjectionConfig::default();
        let injector = AtspiInjector::new(config);
        let context = InjectionContext::default();

        // Empty text should succeed without error
        let result = injector.insert_text("", &context).await;
        assert!(result.is_ok());

        let result = injector.paste_text("", &context).await;
        assert!(result.is_ok());

        let result = injector.inject("", &context).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_legacy_inject_text() {
        let config = InjectionConfig::default();
        let injector = AtspiInjector::new(config);

        // Empty text should succeed without error
        let result = injector.inject_text("", None).await;
        assert!(result.is_ok());
    }
}
