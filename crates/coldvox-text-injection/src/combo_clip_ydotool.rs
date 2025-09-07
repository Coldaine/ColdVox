use crate::clipboard_injector::ClipboardInjector;
use crate::types::{InjectionConfig, InjectionResult};
use crate::TextInjector;
use async_trait::async_trait;
use std::time::{Duration, Instant};
use tokio::process::Command;
use tokio::time::timeout;
use tracing::{debug, trace};

#[cfg(feature = "wl_clipboard")]
use wl_clipboard_rs::{
    copy::{MimeType as CopyMime, Options as CopyOptions, Source as CopySource},
    paste::{get_contents, ClipboardType, MimeType as PasteMime, Seat},
};

/// Combo injector that sets clipboard and then triggers paste (AT-SPI action if available, else ydotool)
pub struct ComboClipboardYdotool {
    _config: InjectionConfig,
    clipboard_injector: ClipboardInjector,
}

impl ComboClipboardYdotool {
    /// Create a new combo clipboard+paste injector
    pub fn new(config: InjectionConfig) -> Self {
        Self {
            _config: config.clone(),
            clipboard_injector: ClipboardInjector::new(config),
        }
    }

    /// Check if this combo injector is available
    pub async fn is_available(&self) -> bool {
        // Requires clipboard to be available and ydotool present (for fallback)
        self.clipboard_injector.is_available().await && Self::check_ydotool().await
    }

    /// Check if ydotool is available in PATH and daemon is accessible
    async fn check_ydotool() -> bool {
        // First check if binary exists
        if let Ok(output) = Command::new("which").arg("ydotool").output().await {
            if !output.status.success() {
                return false;
            }
        } else {
            return false;
        }

        // Then test if daemon is accessible
        match Command::new("ydotool")
            .env("YDOTOOL_SOCKET", "/tmp/.ydotool_socket")
            .arg("--help")
            .output()
            .await
        {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }
}

#[async_trait]
impl TextInjector for ComboClipboardYdotool {
    /// Get the name of this injector
    fn backend_name(&self) -> &'static str {
        "Clipboard+ydotool"
    }

    /// Check if this injector is available for use
    async fn is_available(&self) -> bool {
        self.is_available().await
    }

    /// Inject text using clipboard+ydotool paste
    async fn inject_text(&self, text: &str) -> InjectionResult<()> {
        let start = Instant::now();
        trace!(
            "ComboClipboardYdotool starting injection of {} chars",
            text.len()
        );

        // Optional: save current clipboard for restoration
        #[allow(unused_mut)]
        let mut saved_clipboard: Option<String> = None;
        #[cfg(feature = "wl_clipboard")]
        if self._config.restore_clipboard {
            use std::io::Read;
            match get_contents(ClipboardType::Regular, Seat::Unspecified, PasteMime::Text) {
                Ok((mut pipe, _mime)) => {
                    let mut buf = String::new();
                    if pipe.read_to_string(&mut buf).is_ok() {
                        debug!("Saved prior clipboard ({} chars)", buf.len());
                        saved_clipboard = Some(buf);
                    }
                }
                Err(e) => debug!("Could not read prior clipboard: {}", e),
            }
        }

        // Step 1: Set clipboard content
        let clipboard_start = Instant::now();
        self.clipboard_injector.inject_text(text).await?;
        debug!(
            "Clipboard set with {} chars in {}ms",
            text.len(),
            clipboard_start.elapsed().as_millis()
        );

        // Step 2: Brief clipboard stabilize delay (keep small)
        trace!("Waiting 20ms for clipboard to stabilize");
        tokio::time::sleep(Duration::from_millis(20)).await;

                // Step 3: Trigger paste action via ydotool
        let paste_start = Instant::now();
        let output = timeout(
            Duration::from_millis(self._config.paste_action_timeout_ms),
            Command::new("ydotool")
                .env("YDOTOOL_SOCKET", "/tmp/.ydotool_socket")
                .args(["key", "ctrl+v"])
                .output(),
        )
        .await
        .map_err(|_| crate::types::InjectionError::Timeout(self._config.paste_action_timeout_ms))?
        .map_err(|e| crate::types::InjectionError::Process(format!("ydotool failed: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(crate::types::InjectionError::MethodFailed(format!(
                "ydotool paste failed: {}",
                stderr
            )));
        }

        debug!(
            "Paste triggered via ydotool in {}ms",
            paste_start.elapsed().as_millis()
        );

        // Schedule clipboard restore if configured
        #[cfg(feature = "wl_clipboard")]
        if self._config.restore_clipboard {
            if let Some(content) = saved_clipboard {
                let delay_ms = self._config.clipboard_restore_delay_ms.unwrap_or(500);
                tokio::spawn(async move {
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                    let _ = tokio::task::spawn_blocking(move || {
                        let src = CopySource::Bytes(content.into_bytes().into());
                        let opts = CopyOptions::new();
                        let _ = opts.copy(src, CopyMime::Text);
                    })
                    .await;
                });
            }
        }

        let elapsed = start.elapsed();
        debug!(
            "ComboClipboardYdotool completed in {}ms",
            elapsed.as_millis()
        );

        Ok(())
    }

    /// Get backend-specific configuration information
    fn backend_info(&self) -> Vec<(&'static str, String)> {
        vec![
            ("type", "clipboard+ydotool".to_string()),
            (
                "description",
                "Sets clipboard content and triggers paste via ydotool keyboard simulation".to_string(),
            ),
            ("platform", "Linux (Wayland/X11)".to_string()),
            (
                "status",
                "Active - uses ydotool for paste triggering".to_string(),
            ),
        ]
    }
}

impl ComboClipboardYdotool {
}
