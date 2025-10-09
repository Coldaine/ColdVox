//! # AT-SPI Event Confirmation Module
//!
//! This module provides event-driven confirmation of text injection using AT-SPI events.
//! It subscribes to text-changed events and performs prefix matching to verify
//! that injected text has been properly received by the target application.

use crate::types::{InjectionConfig, InjectionError, InjectionResult};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{debug, error, info, trace, warn};
use unicode_segmentation::UnicodeSegmentation;

/// Confirmation result for text injection
#[derive(Debug, Clone, PartialEq)]
pub enum ConfirmationResult {
    /// Text was successfully confirmed
    Success,
    /// Confirmation timed out
    Timeout,
    /// No matching event was received
    NoMatch,
    /// Error during confirmation
    Error(String),
}

/// AT-SPI event listener for text change confirmation
pub struct TextChangeListener {
    config: InjectionConfig,
    is_listening: Arc<Mutex<bool>>,
}

impl TextChangeListener {
    /// Create a new text change listener
    pub fn new(config: InjectionConfig) -> Self {
        Self {
            config,
            is_listening: Arc::new(Mutex::new(false)),
        }
    }

    /// Extract the first 3-6 visible characters from text for prefix matching
    /// Handles Unicode grapheme clusters properly
    pub fn extract_prefix(text: &str) -> String {
        // Use Unicode grapheme clusters to handle multi-byte characters
        let graphemes: Vec<&str> = text.graphemes(true).collect();
        
        // Take between 3-6 graphemes, preferring shorter for common cases
        let prefix_len = match graphemes.len() {
            0 => return String::new(),
            1..=3 => graphemes.len(),
            4..=6 => 3, // Use 3 for 4-6 char inputs to avoid false positives
            _ => 4, // Use 4 for longer inputs
        };
        
        graphemes.iter().take(prefix_len).cloned().collect()
    }

    /// Check if a text change event matches our expected prefix
    pub fn matches_prefix(event_text: &str, expected_prefix: &str) -> bool {
        if expected_prefix.is_empty() {
            return false;
        }
        
        // Extract prefix from the event text using the same logic
        let event_prefix = Self::extract_prefix(event_text);
        
        // Compare the prefixes
        event_prefix == expected_prefix
    }
}

/// Confirm text injection using AT-SPI events
/// 
/// # Arguments
/// * `target` - Target application identifier
/// * `prefix` - Expected prefix of the injected text (3-6 characters)
/// * `window` - Window identifier for the target
/// 
/// # Returns
/// * `Ok(ConfirmationResult)` - Confirmation result
/// * `Err(InjectionError)` - Error during confirmation
pub async fn text_changed(
    target: &str,
    prefix: &str,
    window: &str,
) -> InjectionResult<ConfirmationResult> {
    let start_time = Instant::now();
    let timeout_duration = Duration::from_millis(75);
    
    info!(
        target = %target,
        window = %window,
        prefix = %prefix,
        "Starting AT-SPI text change confirmation with 75ms timeout"
    );
    
    #[cfg(feature = "atspi")]
    {
        use atspi::{
            connection::AccessibilityConnection,
            proxy::collection::CollectionProxy,
            proxy::text::TextProxy,
            Interface, MatchType, State,
        };
        use tokio::time;
        
        // Extract the prefix we'll be looking for
        let expected_prefix = if prefix.is_empty() {
            warn!("Empty prefix provided for confirmation");
            return Ok(ConfirmationResult::Error("Empty prefix".to_string()));
        } else {
            TextChangeListener::extract_prefix(prefix)
        };
        
        if expected_prefix.is_empty() {
            return Ok(ConfirmationResult::Error("Invalid prefix after extraction".to_string()));
        }
        
        debug!(
            expected_prefix = %expected_prefix,
            original_prefix = %prefix,
            "Extracted prefix for matching"
        );
        
        // Connect to AT-SPI
        let conn = time::timeout(timeout_duration, AccessibilityConnection::new())
            .await
            .map_err(|_| InjectionError::Timeout(75))?
            .map_err(|e| {
                error!("Failed to establish AT-SPI connection: {}", e);
                InjectionError::Other(format!("AT-SPI connect failed: {e}"))
            })?;
        
        let zbus_conn = conn.connection();
        
        // Get the focused editable text element
        let collection_fut = CollectionProxy::builder(zbus_conn)
            .destination("org.a11y.atspi.Registry")
            .map_err(|e| InjectionError::Other(format!("CollectionProxy destination failed: {e}")))?
            .path("/org/a11y/atspi/accessible/root")
            .map_err(|e| InjectionError::Other(format!("CollectionProxy path failed: {e}")))?
            .build();
        
        let collection = time::timeout(timeout_duration, collection_fut)
            .await
            .map_err(|_| InjectionError::Timeout(75))?
            .map_err(|e| InjectionError::Other(format!("CollectionProxy build failed: {e}")))?;
        
        let mut rule = atspi::ObjectMatchRule::default();
        rule.states = State::Focused.into();
        rule.states_mt = MatchType::All;
        rule.ifaces = Interface::EditableText.into();
        rule.ifaces_mt = MatchType::All;
        
        // Get initial text content
        let get_matches = collection.get_matches(rule.clone(), atspi::SortOrder::Canonical, 1, false);
        let matches = time::timeout(Duration::from_millis(25), get_matches)
            .await
            .map_err(|_| InjectionError::Timeout(75))?
            .map_err(|e| {
                trace!("Failed to get focused element: {}", e);
                InjectionError::Other(format!("Get matches failed: {e}"))
            })?;
        
        let mut last_text = String::new();
        
        if let Some(obj_ref) = matches.first() {
            // Get the initial text content
            let text_fut = TextProxy::builder(zbus_conn)
                .destination(obj_ref.name.clone())
                .map_err(|e| InjectionError::Other(format!("TextProxy destination failed: {e}")))?
                .path(obj_ref.path.clone())
                .map_err(|e| InjectionError::Other(format!("TextProxy path failed: {e}")))?
                .build();
            
            if let Ok(text_proxy) = time::timeout(Duration::from_millis(25), text_fut).await {
                if let Ok(text_proxy) = text_proxy {
                    let get_text_fut = text_proxy.get_text(0, -1);
                    if let Ok(current_text) = time::timeout(Duration::from_millis(25), get_text_fut).await {
                        if let Ok(current_text) = current_text {
                            last_text = current_text;
                        }
                    }
                }
            }
        }
        
        // Poll for text changes with small intervals
        let poll_interval = Duration::from_millis(10);
        let mut poll_count = 0;
        let max_polls = 7; // 70ms total (7 * 10ms)
        
        while start_time.elapsed() < timeout_duration && poll_count < max_polls {
            poll_count += 1;
            
            // Get the focused element
            let get_matches = collection.get_matches(rule.clone(), atspi::SortOrder::Canonical, 1, false);
            let matches = time::timeout(poll_interval, get_matches)
                .await
                .map_err(|_| InjectionError::Timeout(75))?
                .map_err(|e| {
                    trace!("Failed to get focused element during polling: {}", e);
                    // Continue polling even if this fails
                })
                .unwrap_or_default();
            
            if let Some(obj_ref) = matches.first() {
                // Get the text content
                let text_fut = TextProxy::builder(zbus_conn)
                    .destination(obj_ref.name.clone())
                    .map_err(|e| InjectionError::Other(format!("TextProxy destination failed: {e}")))?
                    .path(obj_ref.path.clone())
                    .map_err(|e| InjectionError::Other(format!("TextProxy path failed: {e}")))?
                    .build();
                
                if let Ok(text_proxy) = time::timeout(poll_interval, text_fut).await {
                    if let Ok(text_proxy) = text_proxy {
                        let get_text_fut = text_proxy.get_text(0, -1);
                        if let Ok(current_text) = time::timeout(poll_interval, get_text_fut).await {
                            if let Ok(current_text) = current_text {
                                // Check if text has changed and matches our prefix
                                if current_text != last_text {
                                    trace!(
                                        old_text = %last_text,
                                        new_text = %current_text,
                                        "Text content changed during polling"
                                    );
                                    
                                    // Extract the new portion (last few characters)
                                    if current_text.len() > last_text.len() {
                                        let new_chars = &current_text[last_text.len()..];
                                        
                                        debug!(
                                            new_chars = %new_chars,
                                            expected_prefix = %expected_prefix,
                                            "Checking if new text matches expected prefix"
                                        );
                                        
                                        if TextChangeListener::matches_prefix(new_chars, &expected_prefix) {
                                            info!(
                                                new_chars = %new_chars,
                                                expected_prefix = %expected_prefix,
                                                elapsed_ms = %start_time.elapsed().as_millis(),
                                                poll_count = %poll_count,
                                                "Text change confirmed via AT-SPI polling"
                                            );
                                            return Ok(ConfirmationResult::Success);
                                        }
                                    }
                                    
                                    last_text = current_text;
                                }
                            }
                        }
                    }
                }
            }
            
            // Small delay between polls
            tokio::time::sleep(poll_interval).await;
        }
        
        warn!(
            elapsed_ms = %start_time.elapsed().as_millis(),
            poll_count = %poll_count,
            "AT-SPI text change confirmation timed out after 75ms (polling approach)"
        );
        
        Ok(ConfirmationResult::Timeout)
    }
    
    #[cfg(not(feature = "atspi"))]
    {
        warn!("AT-SPI feature disabled; confirmation not available");
        Ok(ConfirmationResult::Error("AT-SPI feature is disabled".to_string()))
    }
}

/// Utility function to create a confirmation context for injection operations
pub fn create_confirmation_context(config: InjectionConfig) -> ConfirmationContext {
    ConfirmationContext {
        listener: TextChangeListener::new(config.clone()),
    }
}

/// Context for managing confirmation operations
pub struct ConfirmationContext {
    listener: TextChangeListener,
}

impl ConfirmationContext {
    /// Confirm text injection using the context's configuration
    pub async fn confirm_injection(
        &self,
        target: &str,
        text: &str,
        window: &str,
    ) -> InjectionResult<ConfirmationResult> {
        let start_time = Instant::now();
        let result = text_changed(target, text, window).await;
        let elapsed = start_time.elapsed();
        
        // Log the result with basic info
        match &result {
            Ok(ConfirmationResult::Success) => {
                info!(
                    target = %target,
                    text_length = %text.len(),
                    duration_ms = %elapsed.as_millis(),
                    "Text injection confirmed successfully"
                );
            }
            Ok(other) => {
                warn!(
                    target = %target,
                    text_length = %text.len(),
                    duration_ms = %elapsed.as_millis(),
                    result = ?other,
                    "Text injection confirmation failed"
                );
            }
            Err(e) => {
                warn!(
                    target = %target,
                    text_length = %text.len(),
                    duration_ms = %elapsed.as_millis(),
                    error = %e.to_string(),
                    "Text injection confirmation error"
                );
            }
        }
        
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_extract_prefix() {
        // Test with ASCII
        assert_eq!(TextChangeListener::extract_prefix("hello"), "hel");
        assert_eq!(TextChangeListener::extract_prefix("hi"), "hi");
        assert_eq!(TextChangeListener::extract_prefix("a"), "a");
        
        // Test with Unicode
        assert_eq!(TextChangeListener::extract_prefix("café"), "caf");
        assert_eq!(TextChangeListener::extract_prefix("👍🏽test"), "👍🏽te");
        
        // Test with longer text
        assert_eq!(TextChangeListener::extract_prefix("this is a long text"), "this");
    }
    
    #[test]
    fn test_matches_prefix() {
        // Test matching
        assert!(TextChangeListener::matches_prefix("hello world", "hel"));
        assert!(TextChangeListener::matches_prefix("café", "caf"));
        
        // Test non-matching
        assert!(!TextChangeListener::matches_prefix("hello world", "wor"));
        assert!(!TextChangeListener::matches_prefix("test", "xyz"));
        
        // Test empty prefix
        assert!(!TextChangeListener::matches_prefix("anything", ""));
    }
}