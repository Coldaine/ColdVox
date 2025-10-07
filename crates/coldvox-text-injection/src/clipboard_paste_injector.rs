use crate::clipboard_injector::ClipboardInjector;
use crate::types::{InjectionConfig, InjectionResult, InjectionError};
use crate::TextInjector;
use async_trait::async_trait;
use tracing::{debug};

#[cfg(feature = "atspi")]
#[cfg(feature = "wl_clipboard")]
use wl_clipboard_rs::{
    copy::{MimeType, Options, Source},
    paste::{get_contents, ClipboardType, MimeType as PasteMimeType, Seat},
};

/// Clipboard injector that always issues a paste (AT-SPI first, then ydotool when available)
pub struct ClipboardPasteInjector {
    config: InjectionConfig,
    clipboard_injector: ClipboardInjector,
}

impl ClipboardPasteInjector {
    /// Create a new clipboard paste injector
    pub fn new(config: InjectionConfig) -> Self {
        Self {
            config: config.clone(),
            clipboard_injector: ClipboardInjector::new(config),
        }
    }

    /// Clipboard availability is enough to expose this injector; ydotool is optional.
    pub async fn is_available(&self) -> bool {
        self.clipboard_injector.is_available().await
    }
}

#[async_trait]
impl TextInjector for ClipboardPasteInjector {
    fn backend_name(&self) -> &'static str {
        "ClipboardPaste"
    }

    async fn is_available(&self) -> bool {
        self.is_available().await
    }

    async fn inject_text(&self, text: &str) -> InjectionResult<()> {
        use std::io::Read;
        use wl_clipboard_rs::copy::{MimeType, Options, Source};
        use wl_clipboard_rs::paste::{get_contents, ClipboardType, MimeType as PasteMimeType, Seat};
        use tokio::time::Duration;

        if text.is_empty() {
            return Ok(());
        }

        // Save current clipboard
        let saved_clipboard = match get_contents(ClipboardType::Regular, Seat::Unspecified, PasteMimeType::Text) {
            Ok((mut pipe, _mime)) => {
                let mut contents = String::new();
                if pipe.read_to_string(&mut contents).is_ok() {
                    Some(contents)
                } else {
                    None
                }
            }
            Err(_) => None,
        };

        // Set new clipboard content
        let source = Source::Bytes(text.as_bytes().to_vec().into());
        let opts = Options::new();
        match opts.copy(source, MimeType::Text) {
            Ok(_) => {
                debug!("ClipboardPasteInjector set clipboard ({} chars)", text.len());
            }
            Err(e) => return Err(InjectionError::Clipboard(e.to_string())),
        }

        // Schedule restoration after a delay
        if let Some(content) = saved_clipboard {
            let delay_ms = self.config.clipboard_restore_delay_ms.unwrap_or(500);
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                let src = Source::Bytes(content.as_bytes().to_vec().into());
                let opts = Options::new();
                let _ = opts.copy(src, MimeType::Text);
            });
        }

        Ok(())
    }

    fn backend_info(&self) -> Vec<(&'static str, String)> {
        vec![
            ("type", "clipboard+paste".to_string()),
            (
                "description",
                "Sets clipboard text, issues AT-SPI paste, falls back to ydotool when available"
                    .to_string(),
            ),
            ("platform", "Linux (Wayland/X11)".to_string()),
            (
                "status",
                "Active - requires clipboard access; ydotool optional".to_string(),
            ),
        ]
    }
}
