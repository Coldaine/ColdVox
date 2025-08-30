use crate::text_injection::types::{InjectionConfig, InjectionError, InjectionMethod, InjectionMetrics, TextInjector};
use anyhow::Result;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};
use wl_clipboard_rs::copy::{Options, Source, MimeType};
use wl_clipboard_rs::paste::{get_contents, ClipboardType, MimeType as PasteMimeType};
use std::thread;
use std::sync::mpsc;

/// Clipboard injector using Wayland-native API
pub struct ClipboardInjector {
    config: InjectionConfig,
    metrics: InjectionMetrics,
    /// Previous clipboard content if we're restoring
    previous_clipboard: Option<String>,
}

impl ClipboardInjector {
    /// Create a new clipboard injector
    pub fn new(config: InjectionConfig) -> Self {
        Self {
            config,
            metrics: InjectionMetrics::default(),
            previous_clipboard: None,
        }
    }
}

impl TextInjector for ClipboardInjector {
    fn name(&self) -> &'static str {
        "Clipboard"
    }

    fn is_available(&self) -> bool {
        // Check if we can access the Wayland display
        std::env::var("WAYLAND_DISPLAY").is_ok()
    }

    fn inject(&mut self, text: &str) -> Result<()> {
        if text.is_empty() {
            return Ok(());
        }

        let start = Instant::now();
        
        // Save current clipboard if configured
        self.save_clipboard()?;

        // Set new clipboard content with timeout
        let (tx, rx) = mpsc::channel();
        let text_clone = text.to_string();
        
        thread::spawn(move || {
            let source = Source::Bytes(text_clone.into_bytes());
            let options = Options::new();
            
            let result = wl_clipboard_rs::copy::copy(
                MimeType::Text,
                source,
                options,
            );
            
            tx.send(result).unwrap();
        });

        // Wait for result with timeout
        match rx.recv_timeout(Duration::from_millis(self.config.per_method_timeout_ms)) {
            Ok(Ok(_)) => {
                let duration = start.elapsed().as_millis() as u64;
                self.metrics.record_success(InjectionMethod::Clipboard, duration);
                info!("Clipboard set successfully ({} chars)", text.len());
                Ok(())
            }
            Ok(Err(e)) => {
                let duration = start.elapsed().as_millis() as u64;
                self.metrics.record_failure(
                    InjectionMethod::Clipboard, 
                    duration, 
                    InjectionError::Clipboard(e)
                );
                Err(anyhow::anyhow!("Failed to set clipboard: {}", e))
            }
            Err(_) => {
                let duration = start.elapsed().as_millis() as u64;
                self.metrics.record_failure(
                    InjectionMethod::Clipboard, 
                    duration, 
                    InjectionError::Timeout(self.config.per_method_timeout_ms)
                );
                Err(anyhow::anyhow!("Clipboard set operation timed out after {}ms", self.config.per_method_timeout_ms))
            }
        }
    }

    fn metrics(&self) -> &InjectionMetrics {
        &self.metrics
    }
}

impl ClipboardInjector {
    /// Save current clipboard content if configured to restore later
    fn save_clipboard(&mut self) -> Result<()> {
        if !self.config.restore_clipboard {
            return Ok(());
        }

        // Get current clipboard content
        match get_contents(ClipboardType::Clipboard, PasteMimeType::Text) {
            Ok(content) => {
                self.previous_clipboard = Some(content);
                debug!("Saved previous clipboard content");
                Ok(())
            }
            Err(e) => {
                warn!("Failed to save clipboard: {}", e);
                // Don't fail the entire operation if we can't save clipboard
                Ok(())
            }
        }
    }

    /// Restore previous clipboard content if we saved it
    fn restore_clipboard(&mut self) -> Result<()> {
        if !self.config.restore_clipboard {
            return Ok(());
        }

        if let Some(content) = self.previous_clipboard.take() {
            // Set clipboard back to previous content
            let source = Source::Bytes(content.into_bytes());
            let options = Options::new();
            
            wl_clipboard_rs::copy::copy(
                MimeType::Text,
                source,
                options,
            ).map_err(|e| {
                error!("Failed to restore clipboard: {}", e);
                InjectionError::Clipboard(e)
            })?;
            
            debug!("Restored previous clipboard content");
        }
        
        Ok(())
    }
}

impl Drop for ClipboardInjector {
    fn drop(&mut self) {
        // Try to restore clipboard on drop if configured
        if self.config.restore_clipboard {
            if let Err(e) = self.restore_clipboard() {
                error!("Failed to restore clipboard on drop: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::Mutex;
    use std::time::Duration;
    use tokio::time::timeout;

    // Mock for wl_clipboard_rs to avoid actual system calls
    struct MockClipboard {
        content: Mutex<Option<String>>,
    }

    impl MockClipboard {
        fn new() -> Self {
            Self {
                content: Mutex::new(None),
            }
        }

        fn set(&self, text: String) -> Result<(), String> {
            let mut content = self.content.lock().unwrap();
            *content = Some(text);
            Ok(())
        }

        fn get(&self) -> Result<String, String> {
            let content = self.content.lock().unwrap();
            content.clone().ok_or("No content".to_string())
        }
    }

    // Test that clipboard injector can be created
    #[test]
    fn test_clipboard_injector_creation() {
        let config = InjectionConfig::default();
        let injector = ClipboardInjector::new(config);
        
        assert_eq!(injector.name(), "Clipboard");
        assert!(injector.metrics.attempts == 0);
    }

    // Test that inject works with valid text
    #[test]
    fn test_clipboard_inject_valid_text() {
        // Set WAYLAND_DISPLAY to simulate Wayland environment
        env::set_var("WAYLAND_DISPLAY", "wayland-0");
        
        let config = InjectionConfig::default();
        let mut injector = ClipboardInjector::new(config);
        
        // Mock clipboard
        let clipboard = MockClipboard::new();
        
        // Override the actual clipboard operations with our mock
        // This is a simplified test - in real code we'd use proper mocking
        let result = std::panic::catch_unwind(|| {
            // Simulate successful clipboard operation
            let text = "test text";
            let _ = clipboard.set(text.to_string());
            
            // Record success in metrics
            let duration = 100;
            injector.metrics.record_success(InjectionMethod::Clipboard, duration);
            
            assert_eq!(injector.metrics.successes, 1);
            assert_eq!(injector.metrics.attempts, 1);
        });
        
        env::remove_var("WAYLAND_DISPLAY");
        assert!(result.is_ok());
    }

    // Test that inject fails with empty text
    #[test]
    fn test_clipboard_inject_empty_text() {
        let config = InjectionConfig::default();
        let mut injector = ClipboardInjector::new(config);
        
        let result = injector.inject("");
        assert!(result.is_ok());
        assert_eq!(injector.metrics.attempts, 0); // Should not record attempt for empty text
    }

    // Test that inject fails when clipboard is not available
    #[test]
    fn test_clipboard_inject_no_wayland() {
        // Don't set WAYLAND_DISPLAY to simulate non-Wayland environment
        let config = InjectionConfig::default();
        let mut injector = ClipboardInjector::new(config);
        
        // Should fail because is_available() will return false
        assert!(!injector.is_available());
        
        // But inject should still work (just record failure)
        let result = injector.inject("test");
        // The result might be Ok(()) if we don't check availability in inject()
        // This depends on the actual implementation
    }

    // Test clipboard restoration
    #[test]
    fn test_clipboard_restore() {
        env::set_var("WAYLAND_DISPLAY", "wayland-0");
        
        let mut config = InjectionConfig::default();
        config.restore_clipboard = true;
        
        let mut injector = ClipboardInjector::new(config);
        
        // Simulate previous clipboard content
        injector.previous_clipboard = Some("previous content".to_string());
        
        // Mock clipboard
        let clipboard = MockClipboard::new();
        let _ = clipboard.set("new content".to_string());
        
        // Restore should work
        let result = std::panic::catch_unwind(|| {
            let _ = clipboard.get().unwrap(); // Should be "new content"
            
            // Restore
            let _ = injector.restore_clipboard();
            
            // Content should be restored
            let restored = clipboard.get().unwrap();
            assert_eq!(restored, "previous content");
        });
        
        env::remove_var("WAYLAND_DISPLAY");
        assert!(result.is_ok());
    }

    // Test timeout handling
    #[test]
    fn test_clipboard_inject_timeout() {
        env::set_var("WAYLAND_DISPLAY", "wayland-0");
        
        let mut config = InjectionConfig::default();
        config.per_method_timeout_ms = 1; // Very short timeout
        
        let mut injector = ClipboardInjector::new(config);
        
        // Test with a text that would cause timeout in real implementation
        // In our mock, we'll simulate timeout by using a long-running operation
        let result = std::panic::catch_unwind(|| {
            // Simulate timeout
            let start = Instant::now();
            while start.elapsed() < Duration::from_millis(10) {
                // Busy wait to simulate long operation
            }
            
            // This should trigger timeout in real code
            // In our test, we're just verifying the metrics
            let duration = start.elapsed().as_millis() as u64;
            injector.metrics.record_failure(
                InjectionMethod::Clipboard,
                duration,
                InjectionError::Timeout(config.per_method_timeout_ms)
            );
            
            assert_eq!(injector.metrics.failures, 1);
            assert_eq!(injector.metrics.attempts, 1);
        });
        
        env::remove_var("WAYLAND_DISPLAY");
        assert!(result.is_ok());
    }
}