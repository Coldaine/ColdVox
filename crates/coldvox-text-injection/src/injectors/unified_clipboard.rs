//! Unified Clipboard Text Injector
//!
//! This module provides a consolidated clipboard-based text injector that combines the best
//! features from ClipboardInjector, ClipboardPasteInjector, and ComboClipboardYdotool.
//! It supports both strict and best-effort injection modes with configurable behavior.

use crate::detection::{detect_display_protocol, DisplayProtocol};
use crate::logging::utils;
use crate::types::{InjectionConfig, InjectionContext, InjectionMethod, InjectionResult};
use crate::TextInjector;
use async_trait::async_trait;
use coldvox_foundation::error::InjectionError;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tokio::time::timeout;
use tracing::{debug, trace, warn};

/// Clipboard injection modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardInjectionMode {
    /// Require successful paste (strict mode like ClipboardPasteInjector)
    Strict,
    /// Best effort with fallbacks (like ClipboardInjector)
    BestEffort,
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

/// Clipboard backend types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardBackend {
    /// Wayland with wl-clipboard-rs (native)
    Wayland,
    /// X11 with xclip (including XWayland)
    X11,
    /// Unknown or unavailable
    Unknown,
}

/// Unified clipboard-based text injector with configurable injection modes
pub struct UnifiedClipboardInjector {
    /// Configuration for injection
    config: InjectionConfig,
    /// Whether the injector is available
    available: Arc<tokio::sync::RwLock<bool>>,
    /// Detected backend type
    backend_type: ClipboardBackend,
    /// Injection mode (strict vs best-effort)
    injection_mode: ClipboardInjectionMode,
}

impl UnifiedClipboardInjector {
    /// Create a new unified clipboard injector
    pub fn new(config: InjectionConfig) -> Self {
        let backend_type = Self::detect_backend();
        Self {
            config,
            available: Arc::new(tokio::sync::RwLock::new(false)),
            backend_type,
            injection_mode: ClipboardInjectionMode::BestEffort, // Default to best-effort
        }
    }

    /// Create a new unified clipboard injector with specific injection mode
    pub fn new_with_mode(config: InjectionConfig, mode: ClipboardInjectionMode) -> Self {
        let backend_type = Self::detect_backend();
        Self {
            config,
            available: Arc::new(tokio::sync::RwLock::new(false)),
            backend_type,
            injection_mode: mode,
        }
    }

    /// Detect the clipboard backend type using unified display protocol detection
    fn detect_backend() -> ClipboardBackend {
        let protocol = detect_display_protocol();

        match protocol {
            DisplayProtocol::Wayland => {
                debug!("Detected Wayland clipboard backend via unified detection");
                ClipboardBackend::Wayland
            }
            DisplayProtocol::X11 => {
                debug!("Detected X11 clipboard backend via unified detection");
                ClipboardBackend::X11
            }
            DisplayProtocol::Unknown => {
                warn!("Could not detect clipboard backend via unified detection");
                ClipboardBackend::Unknown
            }
        }
    }

    /// Helper for "native attempt + command fallback" pattern with consistent timeout/kill handling
    #[allow(dead_code)]
    async fn native_attempt_with_fallback<T, F, Fut, G, Gfut>(
        &self,
        native_attempt: F,
        fallback_name: &str,
        fallback_command: G,
    ) -> InjectionResult<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, InjectionError>>,
        G: FnOnce() -> Gfut,
        Gfut: std::future::Future<Output = Result<T, InjectionError>>,
    {
        // Try native implementation first
        match native_attempt().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                debug!(
                    "Native implementation failed, falling back to {}: {}",
                    fallback_name, e
                );
            }
        }

        // Fallback to command
        fallback_command().await
    }

    /// Execute a command with stdin, stdout, stderr and consistent timeout/kill handling
    async fn execute_command_with_stdin(
        &self,
        command_name: &str,
        command_args: &[&str],
        content: &[u8],
        timeout_duration: Duration,
    ) -> InjectionResult<()> {
        let mut child = Command::new(command_name)
            .args(command_args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                InjectionError::Process(format!("Failed to spawn {}: {}", command_name, e))
            })?;

        // Take stderr for later diagnostics if the process fails
        let mut stderr_pipe = child.stderr.take();

        if let Some(mut stdin) = child.stdin.take() {
            // Timebox the stdin write to avoid hangs
            let per_method = self.config.per_method_timeout();
            timeout(per_method, stdin.write_all(content))
                .await
                .map_err(|_| InjectionError::Timeout(self.config.per_method_timeout_ms))?
                .map_err(|e| {
                    InjectionError::Process(format!(
                        "Failed to write to {} stdin: {}",
                        command_name, e
                    ))
                })?;
            // Explicitly close stdin so the command knows input is finished
            drop(stdin);
        } else {
            return Err(InjectionError::Process(format!(
                "{} stdin unavailable",
                command_name
            )));
        }

        // Wait for command to exit within timeout budget
        match timeout(timeout_duration, child.wait()).await {
            Ok(Ok(status)) => {
                if status.success() {
                    Ok(())
                } else {
                    // Attempt to read stderr for context
                    let mut buf = Vec::new();
                    if let Some(mut s) = stderr_pipe.take() {
                        let _ = s.read_to_end(&mut buf).await;
                    }
                    let stderr = String::from_utf8_lossy(&buf);
                    Err(InjectionError::Process(format!(
                        "{} command failed (status {}): {}",
                        command_name, status, stderr
                    )))
                }
            }
            Ok(Err(e)) => Err(InjectionError::Process(format!(
                "Failed to wait for {}: {}",
                command_name, e
            ))),
            Err(_) => {
                // Timed out: best effort kill to avoid zombie process
                let _ = child.kill().await;
                Err(InjectionError::Timeout(self.config.paste_action_timeout_ms))
            }
        }
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

    /// Read clipboard content using native Wayland wl-clipboard-rs
    #[cfg(feature = "wl_clipboard")]
    async fn read_wayland_clipboard_native(&self) -> InjectionResult<ClipboardBackup> {
        use wl_clipboard_rs::paste::{get_contents, ClipboardType, MimeType, Seat};

        let result = tokio::task::spawn_blocking(move || {
            get_contents(ClipboardType::Regular, Seat::Unspecified, MimeType::Text)
        })
        .await
        .map_err(|e| InjectionError::Other(format!("Failed to spawn clipboard task: {}", e)))?;

        match result {
            Ok((data, _)) => {
                // Convert PipeReader to Vec<u8>
                let mut buf = Vec::new();
                use std::io::Read;
                let mut reader = data;
                reader.read_to_end(&mut buf).map_err(|e| {
                    InjectionError::Other(format!("Failed to read clipboard data: {}", e))
                })?;
                Ok(ClipboardBackup::new(buf, "text/plain".to_string()))
            }
            Err(e) => Err(InjectionError::Other(format!(
                "Failed to read Wayland clipboard: {}",
                e
            ))),
        }
    }

    /// Read clipboard content using Wayland
    async fn read_wayland_clipboard(&self) -> InjectionResult<ClipboardBackup> {
        #[cfg(feature = "wl_clipboard")]
        {
            self.native_attempt_with_fallback(
                || self.read_wayland_clipboard_native(),
                "wl-paste",
                || self.read_wayland_clipboard_fallback(),
            )
            .await
        }
        #[cfg(not(feature = "wl_clipboard"))]
        {
            self.read_wayland_clipboard_fallback().await
        }
    }

    /// Fallback implementation using wl-paste command
    async fn read_wayland_clipboard_fallback(&self) -> InjectionResult<ClipboardBackup> {
        let output = Command::new("wl-paste")
            .args(["--type", "text/plain"])
            .output()
            .await
            .map_err(|e| InjectionError::Process(format!("Failed to execute wl-paste: {}", e)))?;

        if output.status.success() {
            Ok(ClipboardBackup::new(
                output.stdout,
                "text/plain".to_string(),
            ))
        } else {
            Err(InjectionError::Process(
                "wl-paste command failed".to_string(),
            ))
        }
    }

    /// Read clipboard content using X11
    async fn read_x11_clipboard(&self) -> InjectionResult<ClipboardBackup> {
        let output = Command::new("xclip")
            .args(["-selection", "clipboard", "-o"])
            .output()
            .await
            .map_err(|e| InjectionError::Process(format!("Failed to execute xclip: {}", e)))?;

        if output.status.success() {
            Ok(ClipboardBackup::new(
                output.stdout,
                "text/plain".to_string(),
            ))
        } else {
            Err(InjectionError::Process("xclip command failed".to_string()))
        }
    }

    /// Write content to clipboard
    pub async fn write_clipboard(&self, content: &[u8], mime_type: &str) -> InjectionResult<()> {
        let start_time = Instant::now();
        trace!(
            "Writing {} bytes to clipboard (MIME: {})",
            content.len(),
            mime_type
        );

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

    /// Write content to Wayland clipboard using native wl-clipboard-rs
    #[cfg(feature = "wl_clipboard")]
    async fn write_wayland_clipboard_native(
        &self,
        content: &[u8],
        _mime_type: &str,
    ) -> InjectionResult<()> {
        use wl_clipboard_rs::copy::{Options, Source};

        // wl-clipboard-rs operations are blocking; run on a blocking thread
        let data = content.to_vec().into_boxed_slice();
        tokio::task::spawn_blocking(move || {
            let opts = Options::new();
            opts.copy(Source::Bytes(data), wl_clipboard_rs::copy::MimeType::Text)
                .map_err(|e| {
                    InjectionError::Other(format!("Failed to write Wayland clipboard: {}", e))
                })
        })
        .await
        .map_err(|e| InjectionError::Other(format!("Tokio spawn_blocking failed: {}", e)))??;

        Ok(())
    }

    /// Write content to Wayland clipboard
    async fn write_wayland_clipboard(
        &self,
        content: &[u8],
        _mime_type: &str,
    ) -> InjectionResult<()> {
        #[cfg(feature = "wl_clipboard")]
        {
            self.native_attempt_with_fallback(
                || self.write_wayland_clipboard_native(content, _mime_type),
                "wl-copy",
                || self.write_wayland_clipboard_fallback(content),
            )
            .await
        }
        #[cfg(not(feature = "wl_clipboard"))]
        {
            self.write_wayland_clipboard_fallback(content).await
        }
    }

    /// Fallback implementation using wl-copy command
    async fn write_wayland_clipboard_fallback(&self, content: &[u8]) -> InjectionResult<()> {
        let paste_timeout = self.config.paste_action_timeout();
        self.execute_command_with_stdin("wl-copy", &[], content, paste_timeout)
            .await
    }

    /// Write content to X11 clipboard
    async fn write_x11_clipboard(&self, content: &[u8]) -> InjectionResult<()> {
        let paste_timeout = self.config.paste_action_timeout();
        self.execute_command_with_stdin(
            "xclip",
            &["-selection", "clipboard"],
            content,
            paste_timeout,
        )
        .await
    }

    /// Restore clipboard content from backup
    pub async fn restore_clipboard(&self, backup: &ClipboardBackup) -> InjectionResult<()> {
        let start_time = Instant::now();
        trace!(
            "Restoring clipboard from backup ({} bytes)",
            backup.content.len()
        );

        self.write_clipboard(&backup.content, &backup.mime_type)
            .await?;

        let elapsed = start_time.elapsed();
        debug!("Restored clipboard in {}ms", elapsed.as_millis());

        Ok(())
    }

    /// Perform paste action after seeding clipboard
    async fn perform_paste(&self) -> InjectionResult<&'static str> {
        trace!("Performing paste action");

        // Try key event paste if enigo is available
        #[cfg(feature = "enigo")]
        {
            if let Ok(()) = self.try_enigo_paste().await {
                debug!("Paste succeeded via Enigo");
                return Ok("Enigo");
            }
        }

        // Try ydotool as fallback
        if let Ok(()) = self.try_ydotool_paste().await {
            debug!("Paste succeeded via ydotool");
            return Ok("ydotool");
        }

        Err(InjectionError::MethodUnavailable(
            "No paste method available".to_string(),
        ))
    }

    /// Try Enigo paste
    #[cfg(feature = "enigo")]
    async fn try_enigo_paste(&self) -> InjectionResult<()> {
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
                .map_err(|e| InjectionError::MethodFailed(format!("Failed to type 'v': {}", e)))?;
            enigo.key(Key::Control, Direction::Release).map_err(|e| {
                InjectionError::MethodFailed(format!("Failed to release Ctrl: {}", e))
            })?;

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
        let mut command = Command::new("ydotool");
        #[cfg(feature = "ydotool")]
        crate::ydotool_injector::apply_socket_env(&mut command);
        command.args(["key", "ctrl+v"]);

        let output = command
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
    #[cfg(feature = "kdotool")]
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
            .map_err(|e| {
                InjectionError::Process(format!("Failed to execute qdbus for Klipper: {}", e))
            })?;

        if output.status.success() {
            debug!("Klipper history cleared successfully");
            Ok(())
        } else {
            warn!(
                "Failed to clear Klipper history: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            // Don't fail the operation for Klipper cleanup issues
            Ok(())
        }
    }

    /// Schedule clipboard restoration with improved async handling
    async fn schedule_clipboard_restore(&self, backup: Option<ClipboardBackup>) {
        if let Some(backup) = backup {
            let delay_ms = self.config.clipboard_restore_delay_ms.unwrap_or(500);
            // Move only data needed into the task to avoid capturing &self
            let content = backup.content.clone();
            let content_len = content.len();

            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;

                // Restore clipboard content regardless of backend

                #[cfg(feature = "wl_clipboard")]
                {
                    use wl_clipboard_rs::copy::{MimeType, Options, Source};
                    let src = Source::Bytes(content.clone().into_boxed_slice());
                    let opts = Options::new();
                    let _ = opts.copy(src, MimeType::Text);
                    debug!(
                        "Restored original clipboard via wl-clipboard ({} chars)",
                        content_len
                    );
                }

                #[cfg(not(feature = "wl_clipboard"))]
                {
                    // Restore via command-line tools for X11/other backends without borrowing self
                    let restored = Self::restore_clipboard_direct(content.clone()).await;
                    match restored {
                        Ok(_) => debug!(
                            "Restored original clipboard via command-line ({} chars)",
                            content_len
                        ),
                        Err(e) => warn!("Failed to restore clipboard: {}", e),
                    }
                }
            });
        }
    }

    // ...existing code...

    /// Main injection method with configurable behavior
    pub async fn inject(&self, text: &str, _context: &InjectionContext) -> InjectionResult<()> {
        if text.is_empty() {
            return Ok(());
        }

        let start_time = Instant::now();
        trace!(
            "Unified clipboard injector starting for {} chars",
            text.len()
        );

        // Always read fresh clipboard for backup
        let backup = self.read_clipboard().await?;

        // Seed clipboard with payload
        self.write_clipboard(text.as_bytes(), "text/plain").await?;

        // Stabilize clipboard
        tokio::time::sleep(Duration::from_millis(20)).await;

        // Perform paste action
        let paste_result = self.perform_paste().await;

        // Schedule clipboard restoration (whether paste succeeded or not)
        self.schedule_clipboard_restore(Some(backup)).await;

        // Handle result based on injection mode
        match (self.injection_mode, paste_result) {
            (ClipboardInjectionMode::Strict, Err(e)) => {
                warn!(
                    "Strict mode: paste action failed after setting clipboard ({})",
                    e
                );
                return Err(e);
            }
            (ClipboardInjectionMode::BestEffort, Err(e)) => {
                debug!(
                    "Best effort mode: paste action failed but continuing ({})",
                    e
                );
                // Don't return error in best-effort mode
            }
            (_, Ok(method)) => {
                debug!("Paste succeeded via {}", method);
            }
        }

        // Optional Klipper cleanup if enabled
        #[cfg(feature = "kdotool")]
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
            "Unified clipboard injection completed in {}ms ({} chars)",
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

#[async_trait]
impl TextInjector for UnifiedClipboardInjector {
    fn backend_name(&self) -> &'static str {
        "unified-clipboard-injector"
    }

    fn backend_info(&self) -> Vec<(&'static str, String)> {
        vec![
            ("type", "unified clipboard seed/restore".to_string()),
            (
                "description",
                format!(
                    "Backs up clipboard, seeds with payload, performs paste ({:?}), then restores backup",
                    self.injection_mode
                ),
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
    async fn test_unified_clipboard_injector_creation() {
        let config = InjectionConfig::default();
        let injector = UnifiedClipboardInjector::new(config);

        assert_eq!(injector.backend_name(), "unified-clipboard-injector");
        assert_eq!(injector.method(), InjectionMethod::ClipboardPasteFallback);
        assert_eq!(injector.injection_mode, ClipboardInjectionMode::BestEffort);
    }

    #[tokio::test]
    async fn test_unified_clipboard_injector_strict_mode() {
        let config = InjectionConfig::default();
        let injector =
            UnifiedClipboardInjector::new_with_mode(config, ClipboardInjectionMode::Strict);

        assert_eq!(injector.injection_mode, ClipboardInjectionMode::Strict);
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
    async fn test_empty_text_handling() {
        let config = InjectionConfig::default();
        let injector = UnifiedClipboardInjector::new(config);
        let context = InjectionContext::default();

        // Empty text should succeed without error
        let result = injector.inject("", &context).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_backend_detection() {
        // These tests might not work in all environments
        // but ensure the detection code doesn't panic
        let backend_type = UnifiedClipboardInjector::detect_backend();
        assert!(matches!(
            backend_type,
            ClipboardBackend::Wayland | ClipboardBackend::X11 | ClipboardBackend::Unknown
        ));
    }
}
