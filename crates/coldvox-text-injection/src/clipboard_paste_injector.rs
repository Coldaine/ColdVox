#![allow(unused_imports)]

use crate::injectors::clipboard::ClipboardInjector;
use crate::types::{InjectionConfig, InjectionError, InjectionResult};
use crate::TextInjector;
use async_trait::async_trait;
use std::time::{Duration, Instant};
use tokio::process::Command;
use tokio::time::timeout;
use tracing::{debug, trace, warn};

#[cfg(feature = "atspi")]
use atspi::{
    connection::AccessibilityConnection, proxy::action::ActionProxy,
    proxy::collection::CollectionProxy, Interface, MatchType, ObjectMatchRule, SortOrder, State,
};

#[cfg(feature = "wl_clipboard")]
use wl_clipboard_rs::{
    copy::{MimeType, Options, Source},
    paste::{get_contents, ClipboardType, MimeType as PasteMimeType, Seat},
};

/// Clipboard injector that always issues a paste and returns failure if no paste action succeeds.
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

    /// Non-blocking detection of ydotool for optional fallback behaviour.
    async fn ydotool_available() -> bool {
        match Command::new("which").arg("ydotool").output().await {
            Ok(o) => o.status.success(),
            Err(_) => false,
        }
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

    async fn inject_text(&self, text: &str, _context: Option<&crate::types::InjectionContext>) -> InjectionResult<()> {
        if text.is_empty() {
            return Ok(());
        }

        let start = Instant::now();
        trace!(
            "ClipboardPasteInjector starting injection of {} chars",
            text.len()
        );

        // Step 1: Save original clipboard ONCE with timeout
        #[allow(unused_mut)]
        let mut saved_clipboard: Option<String> = None;
        #[cfg(feature = "wl_clipboard")]
        {
            // Wrap the blocking clipboard read in a timeout to prevent hangs. When the
            // compositor or a clipboard manager holds the selection, the blocking read can
            // otherwise stall this async task indefinitely.
            let clipboard_timeout = Duration::from_millis(500);

            let read_future = tokio::task::spawn_blocking(|| {
                use std::io::Read;

                get_contents(
                    ClipboardType::Regular,
                    Seat::Unspecified,
                    PasteMimeType::Text,
                )
                .map_err(|e| format!("get_contents failed: {}", e))
                .and_then(|(mut pipe, _)| {
                    let mut buf = String::new();
                    pipe.read_to_string(&mut buf)
                        .map_err(|e| format!("read_to_string failed: {}", e))
                        .map(|_| buf)
                })
            });

            match timeout(clipboard_timeout, read_future).await {
                Ok(Ok(Ok(buf))) => {
                    debug!("Saved original clipboard ({} chars)", buf.len());
                    saved_clipboard = Some(buf);
                }
                Ok(Ok(Err(e))) => debug!("Could not read original clipboard: {}", e),
                Ok(Err(join_err)) => warn!("Clipboard read task join error: {}", join_err),
                Err(_) => debug!(
                    "Clipboard read timed out after {}ms",
                    clipboard_timeout.as_millis()
                ),
            }
        }

        // Step 2: Set clipboard to new text (delegate to ClipboardInjector)
        let clipboard_start = Instant::now();
        self.clipboard_injector.inject_text(text, _context).await?;
        debug!(
            "Clipboard set with {} chars in {}ms",
            text.len(),
            clipboard_start.elapsed().as_millis()
        );

        // Step 3: Brief stabilization delay
        trace!("Waiting 20ms for clipboard to stabilize");
        tokio::time::sleep(Duration::from_millis(20)).await;

        // Step 4: Try to paste (AT-SPI first, ydotool fallback)
        let paste_result = self.try_paste_action().await;

        // Step 5: Schedule restoration of ORIGINAL clipboard (whether paste succeeded or not)
        if let Some(content) = saved_clipboard {
            let delay_ms = self.config.clipboard_restore_delay_ms.unwrap_or(500);
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                #[cfg(feature = "wl_clipboard")]
                {
                    use wl_clipboard_rs::copy::{MimeType, Options, Source};
                    let src = Source::Bytes(content.as_bytes().to_vec().into());
                    let opts = Options::new();
                    let _ = opts.copy(src, MimeType::Text);
                    debug!("Restored original clipboard ({} chars)", content.len());
                }
            });
        }

        // Step 6: Require a successful paste. If it fails, propagate the error so callers know.
        match paste_result {
            Ok(method) => {
                debug!(
                    "Paste succeeded via {} in {}ms",
                    method,
                    start.elapsed().as_millis()
                );
                Ok(())
            }
            Err(e) => {
                warn!(
                    "ClipboardPasteInjector aborting: paste action failed after setting clipboard ({})",
                    e
                );
                Err(e)
            }
        }
    }

    fn backend_info(&self) -> Vec<(&'static str, String)> {
        vec![
            ("type", "clipboard+paste".to_string()),
            (
                "description",
                "Sets clipboard text and requires paste action success (AT-SPI first, ydotool fallback)"
                    .to_string(),
            ),
            ("platform", "Linux (Wayland/X11)".to_string()),
            (
                "status",
                "Active - requires clipboard access and fails when no paste action succeeds"
                    .to_string(),
            ),
        ]
    }
}

impl ClipboardPasteInjector {
    /// Helper: try AT-SPI paste first (when enabled), then ydotool fallback. Returning `Err`
    /// here propagates up so `inject_text` fails fast when nothing actually pastes.
    async fn try_paste_action(&self) -> InjectionResult<&'static str> {
        // Try AT-SPI paste first
        #[cfg(feature = "atspi")]
        {
            use tokio::time::timeout;
            match timeout(self.config.paste_action_timeout(), self.try_atspi_paste()).await {
                Ok(Ok(())) => return Ok("AT-SPI"),
                Ok(Err(e)) => debug!("AT-SPI paste failed: {}", e),
                Err(_) => debug!("AT-SPI paste timed out"),
            }
        }

        // Try ydotool fallback (only if available)
        if Self::ydotool_available().await {
            use tokio::time::timeout;
            let out = timeout(
                self.config.paste_action_timeout(),
                Command::new("ydotool").args(["key", "ctrl+v"]).output(),
            )
            .await
            .map_err(|_| InjectionError::Timeout(self.config.paste_action_timeout_ms))?
            .map_err(|e| InjectionError::Process(format!("ydotool failed: {}", e)))?;

            if out.status.success() {
                return Ok("ydotool");
            } else {
                let stderr = String::from_utf8_lossy(&out.stderr);
                debug!("ydotool paste failed: {}", stderr);
            }
        }

        Err(InjectionError::MethodUnavailable(
            "Neither AT-SPI nor ydotool available".to_string(),
        ))
    }
    #[cfg(feature = "atspi")]
    async fn try_atspi_paste(&self) -> InjectionResult<()> {
        use crate::types::InjectionError;

        let conn = AccessibilityConnection::new()
            .await
            .map_err(|e| InjectionError::Other(format!("AT-SPI connect failed: {e}")))?;
        let zbus_conn = conn.connection();

        let collection = CollectionProxy::builder(zbus_conn)
            .destination("org.a11y.atspi.Registry")
            .map_err(|e| InjectionError::Other(format!("CollectionProxy destination failed: {e}")))?
            .path("/org/a11y/atspi/accessible/root")
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
            .get_matches(rule.clone(), SortOrder::Canonical, 1, false)
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

        let Some(obj_ref) = matches.into_iter().next() else {
            return Err(InjectionError::MethodUnavailable(
                "No focused actionable element for AT-SPI paste".to_string(),
            ));
        };

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
                    "No AT-SPI paste action on focused element".to_string(),
                )
            })?;

        action
            .do_action(paste_index as i32)
            .await
            .map_err(|e| InjectionError::Other(format!("Action.do_action failed: {e}")))?;

        Ok(())
    }
}
