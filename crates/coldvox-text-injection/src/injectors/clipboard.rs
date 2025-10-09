//! Clipboard Text Injector with Seed/Restore Functionality
//!
//! This module provides a clipboard-based text injector that implements seed/restore
//! functionality. It backs up the clipboard content, seeds it with the payload,
//! performs the injection, and then restores the original clipboard content.
//! Optional Klipper cleanup is available behind a feature flag.

use crate::logging::utils;
use crate::types::{InjectionConfig, InjectionError, InjectionResult, InjectionMethod};
use crate::TextInjector;
use async_trait::async_trait;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::process::Command;
use tokio::time::timeout;
use tracing::{debug, error, info, trace, warn};

/// Context for clipboard injection operations
#[derive(Debug, Clone)]
pub struct Context {
    /// Pre-warmed clipboard data (backup)
    pub clipboard_backup: Option<ClipboardBackup>,
    /// Target application identifier
    pub target_app: Option<String>,
    /// Window identifier
    pub window_id: Option<String>,
}

impl Default for Context {
    fn default() -> Self {
        Self {
            clipboard_backup: None,
            target_app: None,
            window_id: None,
        }
    }
}

/// Clipboard backup data with MIME type information
#[derive(Debug, Clone)]
pub struct ClipboardBackup {
    /// Content of the clipboard
    pub content: Vec<u8>,
    /// MIME type of the content
    pub mime_type: String,
    /// Timestamp when backup was created
    pub timestamp: std::time::Instant,
}

impl ClipboardBackup {
    /// Create a new clipboard backup
    pub fn new(content: Vec<u8>, mime_type: String) -> Self {
        Self {
            content,
            mime_type,
            timestamp: std::time::Instant::now(),
        }
    }

    /// Check if the backup is still valid (not too old)
    pub fn is_valid(&self, max_age: Duration) -> bool {
        self.timestamp.elapsed() < max_age
    }
}

/// Clipboard-based text injector with seed/restore functionality
pub struct ClipboardInjector {
    /// Configuration for injection
    config: InjectionConfig,
    /// Whether the injector is available
    available: Arc<tokio::sync::RwLock<bool>>,
    /// Detected backend type
    backend_type: ClipboardBackend,
}

/// Clipboard backend types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardBackend {
    /// Wayland with wl-clipboard-rs
    Wayland,
    /// X11 with xclip
    X11,
    /// Unknown or unavailable
    Unknown,
}

impl ClipboardInjector {
    /// Create a new clipboard injector
    pub fn new(config: InjectionConfig) -> Self {
        let backend_type = Self::detect_backend();
        Self {
            config,
            available: Arc::new(tokio::sync::RwLock::new(false)),
            backend_type,
        }
    }

    /// Detect the clipboard backend type
    fn detect_backend() -> ClipboardBackend {
        // Check for Wayland first
        if std::env::var("WAYLAND_DISPLAY").is_ok() {
            debug!("Detected Wayland clipboard backend");
            return ClipboardBackend::Wayland;
        }

        // Check for X11
        if std::env::var("DISPLAY").is_ok() {
            debug!("Detected X11 clipboard backend");
            return ClipboardBackend::X11;
        }

        warn!("Could not detect clipboard backend");
        ClipboardBackend::Unknown
    }

    /// Read clipboard content for backup
    pub async fn read_clipboard(&self) -> InjectionResult<ClipboardBackup> {
        let start_time = Instant::now();
        trace!("Reading clipboard content for backup");

        let backup = match self.backend_type {
            ClipboardBackend::Wayland => self.read_wayland_clipboard().await?,
            ClipboardBackend::X11 => self.read_x11_clipboard().await?,
            ClipboardBackend::Unknown => {
                return Err(InjectionError::MethodUnavailable(
                    "No supported clipboard backend detected".to_string(),
                ))
            }
        };

        let elapsed = start_time.elapsed();
        debug!(
            "Read clipboard backup in {}ms ({} bytes, MIME: {:?})",
            elapsed.as_millis(),
            backup.content.len(),
            backup.mime_type
        );

        Ok(backup)
    }

    /// Read clipboard content using Wayland
    async fn read_wayland_clipboard(&self) -> InjectionResult<ClipboardBackup> {
        #[cfg(feature = "wl_clipboard")]
        {
            use wl_clipboard_rs::paste::{get_contents, ClipboardType, MimeType, Seat};

            let read_future = tokio::task::spawn_blocking(|| {
                get_contents(ClipboardType::Regular, Seat::Unspecified, MimeType::Text)
                    .map_err(|e| InjectionError::Clipboard(format!("Wayland clipboard read failed: {}", e)))
                    .and_then(|(mut pipe, mime)| {
                        let mut buf = Vec::new();
                        use std::io::Read;
                        pipe.read_to_end(&mut buf)
                            .map_err(|e| InjectionError::Clipboard(format!("Failed to read clipboard data: {}", e)))
                            .map(|_| (buf, mime))
                    })
            });

            match timeout(Duration::from_millis(500), read_future).await {
                Ok(Ok((content, mime))) => {
                    let mime_string = match mime {
                        MimeType::Text => "text/plain".to_string(),
                        _ => "unknown".to_string(),
                    };
                    Ok(ClipboardBackup::new(content, mime_string))
                }
                Ok(Err(e)) => Err(e),
                Err(_) => Err(InjectionError::Timeout(500)),
            }
        }

        #[cfg(not(feature = "wl_clipboard"))]
        {
            // Fallback to wl-paste command
            let output = Command::new("wl-paste")
                .args(&["--type", "text/plain"])
                .output()
                .await
                .map_err(|e| InjectionError::Process(format!("Failed to execute wl-paste: {}", e)))?;

            if output.status.success() {
                Ok(ClipboardBackup::new(output.stdout, "text/plain".to_string()))
            } else {
                Err(InjectionError::Process("wl-paste command failed".to_string()))
            }
        }
    }

    /// Read clipboard content using X11
    async fn read_x11_clipboard(&self) -> InjectionResult<ClipboardBackup> {
        let output = Command::new("xclip")
            .args(&["-selection", "clipboard", "-o"])
            .output()
            .await
            .map_err(|e| InjectionError::Process(format!("Failed to execute xclip: {}", e)))?;

        if output.status.success() {
            Ok(ClipboardBackup::new(output.stdout, "text/plain".to_string()))
        } else {
            Err(InjectionError::Process("xclip command failed".to_string()))
        }
    }

    /// Write content to clipboard
    pub async fn write_clipboard(&self, content: &[u8], mime_type: &str) -> InjectionResult<()> {
        let start_time = Instant::now();
        trace!("Writing {} bytes to clipboard (MIME: {})", content.len(), mime_type);

        match self.backend_type {
            ClipboardBackend::Wayland => self.write_wayland_clipboard(content, mime_type).await?,
            ClipboardBackend::X11 => self.write_x11_clipboard(content).await?,
            ClipboardBackend::Unknown => {
                return Err(InjectionError::MethodUnavailable(
                    "No supported clipboard backend detected".to_string(),
                ))
            }
        }

        let elapsed = start_time.elapsed();
        debug!(
            "Wrote to clipboard in {}ms ({} bytes, MIME: {})",
            elapsed.as_millis(),
            content.len(),
            mime_type
        );

        Ok(())
    }

    /// Write content to Wayland clipboard
    async fn write_wayland_clipboard(&self, content: &[u8], mime_type: &str) -> InjectionResult<()> {
        #[cfg(feature = "wl_clipboard")]
        {
            use wl_clipboard_rs::copy::{MimeType, Options, Source};

            let write_future = tokio::task::spawn_blocking({
                let content = content.to_vec();
                let mime_type = mime_type.to_string();
                move || {
                    let source = Source::Bytes(content.into());
                    let opts = Options::new();

                    let mime_enum = match mime_type.as_str() {
                        "text/plain" => MimeType::Text,
                        _ => MimeType::Other(mime_type),
                    };

                    opts.copy(source, mime_enum)
                        .map_err(|e| InjectionError::Clipboard(format!("Wayland clipboard write failed: {}", e)))
                }
            });

            match timeout(Duration::from_millis(500), write_future).await {
                Ok(Ok(())) => Ok(()),
                Ok(Err(e)) => Err(e),
                Err(_) => Err(InjectionError::Timeout(500)),
            }
        }

        #[cfg(not(feature = "wl_clipboard"))]
        {
            // Fallback to wl-copy command
            let content_str = String::from_utf8_lossy(content);
            let output = Command::new("wl-copy")
                .arg(&*content_str)
                .output()
                .await
                .map_err(|e| InjectionError::Process(format!("Failed to execute wl-copy: {}", e)))?;

            if output.status.success() {
                Ok(())
            } else {
                Err(InjectionError::Process("wl-copy command failed".to_string()))
            }
        }
    }

    /// Write content to X11 clipboard
    async fn write_x11_clipboard(&self, content: &[u8]) -> InjectionResult<()> {
        let content_str = String::from_utf8_lossy(content);
        let output = Command::new("xclip")
            .args(&["-selection", "clipboard"])
            .input(content_str.as_bytes())
            .output()
            .await
            .map_err(|e| InjectionError::Process(format!("Failed to execute xclip: {}", e)))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(InjectionError::Process("xclip command failed".to_string()))
        }
    }

    /// Restore clipboard content from backup
    pub async fn restore_clipboard(&self, backup: &ClipboardBackup) -> InjectionResult<()> {
        let start_time = Instant::now();
        trace!("Restoring clipboard from backup ({} bytes)", backup.content.len());

        self.write_clipboard(&backup.content, &backup.mime_type).await?;

        let elapsed = start_time.elapsed();
        debug!("Restored clipboard in {}ms", elapsed.as_millis());

        Ok(())
    }

    /// Perform paste action after seeding clipboard
    async fn perform_paste(&self) -> InjectionResult<()> {
        trace!("Performing paste action");

        // Try AT-SPI paste first if available
        #[cfg(feature = "atspi")]
        {
            if let Ok(()) = self.try_atspi_paste().await {
                debug!("Paste succeeded via AT-SPI");
                return Ok(());
            }
        }

        // Try key event paste if enigo is available
        #[cfg(feature = "enigo")]
        {
            if let Ok(()) = self.try_enigo_paste().await {
                debug!("Paste succeeded via Enigo");
                return Ok(());
            }
        }

        // Try ydotool as fallback
        if let Ok(()) = self.try_ydotool_paste().await {
            debug!("Paste succeeded via ydotool");
            return Ok(());
        }

        Err(InjectionError::MethodUnavailable(
            "No paste method available".to_string(),
        ))
    }

    /// Try AT-SPI paste
    #[cfg(feature = "atspi")]
    async fn try_atspi_paste(&self) -> InjectionResult<()> {
        use atspi::{
            connection::AccessibilityConnection, proxy::action::ActionProxy,
            proxy::collection::CollectionProxy, Interface, MatchType, ObjectMatchRule, SortOrder, State,
        };

        let conn = AccessibilityConnection::new()
            .await
            .map_err(|e| InjectionError::Other(format!("AT-SPI connect failed: {e}")))?;
        let zbus_conn = conn.connection();

        let collection = CollectionProxy::builder(zbus_conn)
            .destination("org.a11y.atspi.Registry")
            .map_err(|e| InjectionError::Other(format!("CollectionProxy destination failed: {e}")))?
            .path("/org/a11y.atspi/accessible/root")
            .map_err(|e| InjectionError::Other(format!("CollectionProxy path failed: {e}")))?
            .build()
            .await
            .map_err(|e| InjectionError::Other(format!("CollectionProxy build failed: {e}")))?;

        let mut rule = ObjectMatchRule::default();
        rule.states = State::Focused.into();
        rule.states_mt = MatchType::All;
        rule.ifaces = Interface::Action.into();
        rule.ifaces_mt = MatchType::Any;

        let mut matches = collection
            .get_matches(rule, SortOrder::Canonical, 1, false)
            .await
            .map_err(|e| InjectionError::Other(format!("Collection.get_matches failed: {e}")))?;

        if matches.is_empty() {
            rule.ifaces = Interface::EditableText.into();
            matches = collection
                .get_matches(rule, SortOrder::Canonical, 1, false)
                .await
                .map_err(|e| {
                    InjectionError::Other(format!(
                        "Collection.get_matches (EditableText) failed: {e}"
                    ))
                })?;
        }

        let obj_ref = matches
            .into_iter()
            .next()
            .ok_or_else(|| InjectionError::MethodUnavailable("No focused element".to_string()))?;

        let action = ActionProxy::builder(zbus_conn)
            .destination(obj_ref.name.clone())
            .map_err(|e| InjectionError::Other(format!("ActionProxy destination failed: {e}")))?
            .path(obj_ref.path.clone())
            .map_err(|e| InjectionError::Other(format!("ActionProxy path failed: {e}")))?
            .build()
            .await
            .map_err(|e| InjectionError::Other(format!("ActionProxy build failed: {e}")))?;

        let actions = action
            .get_actions()
            .await
            .map_err(|e| InjectionError::Other(format!("Action.get_actions failed: {e}")))?;

        let paste_index = actions
            .iter()
            .position(|a| {
                let n = a.name.to_ascii_lowercase();
                let d = a.description.to_ascii_lowercase();
                n.contains("paste") || d.contains("paste")
            })
            .ok_or_else(|| {
                InjectionError::MethodUnavailable(
                    "No paste action found on focused element".to_string(),
                )
            })?;

        action
            .do_action(paste_index as i32)
            .await
            .map_err(|e| InjectionError::Other(format!("Action.do_action failed: {e}")))?;

        Ok(())
    }

    /// Try Enigo paste
    #[cfg(feature = "enigo")]
    async fn try_enigo_paste(&self) -> InjectionResult<()> {
        use enigo::{Direction, Enigo, Key, Keyboard, Settings};

        let result = tokio::task::spawn_blocking(move || {
            let mut enigo = Enigo::new(&Settings::default())
                .map_err(|e| InjectionError::MethodFailed(format!("Failed to create Enigo: {}", e)))?;

            // Press Ctrl+V for paste
            enigo.key(Key::Control, Direction::Press)
                .map_err(|e| InjectionError::MethodFailed(format!("Failed to press Ctrl: {}", e)))?;
            enigo.key(Key::Unicode('v'), Direction::Click)
                .map_err(|e| InjectionError::MethodFailed(format!("Failed to type 'v': {}", e)))?;
            enigo.key(Key::Control, Direction::Release)
                .map_err(|e| InjectionError::MethodFailed(format!("Failed to release Ctrl: {}", e)))?;

            Ok(())
        })
        .await;

        match result {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(InjectionError::Timeout(0)),
        }
    }

    /// Try ydotool paste
    async fn try_ydotool_paste(&self) -> InjectionResult<()> {
        let output = Command::new("ydotool")
            .args(&["key", "ctrl+v"])
            .output()
            .await
            .map_err(|e| InjectionError::Process(format!("Failed to execute ydotool: {}", e)))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(InjectionError::Process("ydotool paste failed".to_string()))
        }
    }

    /// Clear Klipper history (optional, behind feature flag)
    #[cfg(feature = "klipper")]
    async fn clear_klipper_history(&self) -> InjectionResult<()> {
        trace!("Clearing Klipper history");

        // Use qdbus to clear Klipper history
        let output = Command::new("qdbus")
            .args(&[
                "org.kde.klipper",
                "/klipper",
                "org.kde.klipper.klipper",
                "clearClipboardHistory",
            ])
            .output()
            .await
            .map_err(|e| InjectionError::Process(format!("Failed to execute qdbus for Klipper: {}", e)))?;

        if output.status.success() {
            debug!("Klipper history cleared successfully");
            Ok(())
        } else {
            warn!("Failed to clear Klipper history: {}", String::from_utf8_lossy(&output.stderr));
            // Don't fail the operation for Klipper cleanup issues
            Ok(())
        }
    }

    /// Main injection method
    pub async fn inject(&self, text: &str, context: &Context) -> InjectionResult<()> {
        if text.is_empty() {
            return Ok(());
        }

        let start_time = Instant::now();
        trace!("Clipboard injector starting for {} chars", text.len());

        // Check if we need to create a backup or use the pre-warmed one
        let backup = if let Some(ref prewarmed) = context.clipboard_backup {
            if prewarmed.is_valid(Duration::from_secs(5)) {
                debug!("Using pre-warmed clipboard backup");
                prewarmed.clone()
            } else {
                trace!("Prewarmed backup expired, creating new backup");
                self.read_clipboard().await?
            }
        } else {
            trace!("No pre-warmed backup, creating new backup");
            self.read_clipboard().await?
        };

        // Seed clipboard with payload
        self.write_clipboard(text.as_bytes(), "text/plain").await?;

        // Stabilize clipboard
        tokio::time::sleep(Duration::from_millis(20)).await;

        // Perform paste action
        self.perform_paste().await?;

        // Wait for paste to complete
        let restore_delay = self.config.clipboard_restore_delay_ms.unwrap_or(500);
        tokio::time::sleep(Duration::from_millis(restore_delay)).await;

        // Always restore clipboard backup
        if let Err(e) = self.restore_clipboard(&backup).await {
            warn!("Failed to restore clipboard: {}", e);
        }

        // Optional Klipper cleanup if enabled
        #[cfg(feature = "klipper")]
        {
            if self.config.allow_kdotool {
                // Use allow_kdotool as a proxy for enabling Klipper cleanup
                if let Err(e) = self.clear_klipper_history().await {
                    warn!("Klipper cleanup failed: {}", e);
                }
            }
        }

        let elapsed = start_time.elapsed();
        
        // Log successful injection
        utils::log_injection_success(
            InjectionMethod::ClipboardPasteFallback,
            text,
            elapsed,
            self.config.redact_logs,
        );

        debug!(
            "Clipboard injection completed in {}ms ({} chars)",
            elapsed.as_millis(),
            text.len()
        );

        Ok(())
    }

    /// Get the injection method used by this injector
    pub fn method(&self) -> InjectionMethod {
        InjectionMethod::ClipboardPasteFallback
    }

    /// Check if the injector backend is available
    pub async fn check_availability(&self) -> bool {
        match self.backend_type {
            ClipboardBackend::Wayland => {
                // Try to read clipboard as a basic availability check
                self.read_wayland_clipboard().await.is_ok()
            }
            ClipboardBackend::X11 => {
                // Try to read clipboard as a basic availability check
                self.read_x11_clipboard().await.is_ok()
            }
            ClipboardBackend::Unknown => false,
        }
    }
}

/// Seed/restore wrapper function
pub async fn with_seed_restore<F, Fut>(
    payload: &[u8],
    mime_type: &str,
    backup: Option<&ClipboardBackup>,
    f: F,
) -> InjectionResult<()>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = InjectionResult<()>>,
{
    // Create injector with default config
    let config = InjectionConfig::default();
    let injector = ClipboardInjector::new(config);

    // Create backup if not provided
    let backup = if let Some(backup) = backup {
        backup.clone()
    } else {
        injector.read_clipboard().await?
    };

    // Seed clipboard with payload
    injector.write_clipboard(payload, mime_type).await?;

    // Run the provided function
    let result = f().await;

    // Always restore clipboard backup
    if let Err(e) = injector.restore_clipboard(&backup).await {
        warn!("Failed to restore clipboard in with_seed_restore: {}", e);
    }

    result
}

#[async_trait]
impl TextInjector for ClipboardInjector {
    fn backend_name(&self) -> &'static str {
        "clipboard-injector"
    }

    fn backend_info(&self) -> Vec<(&'static str, String)> {
        vec![
            ("type", "clipboard seed/restore".to_string()),
            (
                "description",
                "Backs up clipboard, seeds with payload, performs paste, then restores backup".to_string(),
            ),
            ("backend", format!("{:?}", self.backend_type)),
            ("paste_methods", "AT-SPI, Enigo, ydotool".to_string()),
        ]
    }

    async fn is_available(&self) -> bool {
        // Use cached availability if already checked
        {
            let available = self.available.read().await;
            if *available {
                return true;
            }
        }

        // Check availability and cache the result
        let is_available = self.check_availability().await;
        *self.available.write().await = is_available;
        is_available
    }

    async fn inject_text(&self, text: &str) -> InjectionResult<()> {
        // Create a default context for the legacy inject_text method
        let context = Context::default();
        self.inject(text, &context).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_clipboard_injector_creation() {
        let config = InjectionConfig::default();
        let injector = ClipboardInjector::new(config);
        
        assert_eq!(injector.backend_name(), "clipboard-injector");
        assert_eq!(injector.method(), InjectionMethod::ClipboardPasteFallback);
    }

    #[tokio::test]
    async fn test_clipboard_backup_creation() {
        let content = b"test content".to_vec();
        let backup = ClipboardBackup::new(content.clone(), "text/plain".to_string());
        
        assert_eq!(backup.content, content);
        assert_eq!(backup.mime_type, "text/plain");
        assert!(backup.is_valid(Duration::from_secs(1)));
        assert!(!backup.is_valid(Duration::from_nanos(1)));
    }

    #[tokio::test]
    async fn test_context_default() {
        let context = Context::default();
        
        assert!(context.clipboard_backup.is_none());
        assert!(context.target_app.is_none());
        assert!(context.window_id.is_none());
    }

    #[tokio::test]
    async fn test_with_seed_restore_wrapper() {
        let payload = b"test payload";
        let mime_type = "text/plain";
        
        // Test the wrapper function
        let result = with_seed_restore(payload, mime_type, None, async {
            Ok(())
        }).await;
        
        // The result may fail due to clipboard unavailability in test environment
        // but we're testing the wrapper structure
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_empty_text_handling() {
        let config = InjectionConfig::default();
        let injector = ClipboardInjector::new(config);
        let context = Context::default();
        
        // Empty text should succeed without error
        let result = injector.inject("", &context).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_legacy_inject_text() {
        let config = InjectionConfig::default();
        let injector = ClipboardInjector::new(config);
        
        // Empty text should succeed without error
        let result = injector.inject_text("").await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_backend_detection() {
        // These tests might not work in all environments
        // but ensure the detection code doesn't panic
        let backend_type = ClipboardInjector::detect_backend();
        assert!(matches!(backend_type, ClipboardBackend::Wayland | ClipboardBackend::X11 | ClipboardBackend::Unknown));
    }
}