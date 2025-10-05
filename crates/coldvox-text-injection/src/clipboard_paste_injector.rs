use crate::clipboard_injector::ClipboardInjector;
use crate::types::{InjectionConfig, InjectionResult};
use crate::TextInjector;
use async_trait::async_trait;
use std::time::{Duration, Instant};
use tokio::process::Command;
use tokio::time::timeout;
use tracing::{debug, trace};

#[cfg(feature = "atspi")]
use atspi::{
    connection::AccessibilityConnection, proxy::action::ActionProxy,
    proxy::collection::CollectionProxy, Interface, MatchType, ObjectMatchRule, SortOrder, State,
};

#[cfg(feature = "wl_clipboard")]
use wl_clipboard_rs::{
    copy::{MimeType as CopyMime, Options as CopyOptions, Source as CopySource},
    paste::{get_contents, ClipboardType, MimeType as PasteMime, Seat},
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

    async fn inject_text(&self, text: &str) -> InjectionResult<()> {
        let start = Instant::now();
        trace!(
            "ClipboardPasteInjector starting injection of {} chars",
            text.len()
        );

        #[allow(unused_mut)]
        let mut saved_clipboard: Option<String> = None;
        #[cfg(feature = "wl_clipboard")]
        {
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

        let clipboard_start = Instant::now();
        self.clipboard_injector.inject_text(text).await?;
        debug!(
            "Clipboard set with {} chars in {}ms",
            text.len(),
            clipboard_start.elapsed().as_millis()
        );

        trace!("Waiting 20ms for clipboard to stabilize");
        tokio::time::sleep(Duration::from_millis(20)).await;

        #[cfg(feature = "atspi")]
        {
            match timeout(
                Duration::from_millis(self.config.paste_action_timeout_ms),
                self.try_atspi_paste(),
            )
            .await
            {
                Ok(Ok(())) => {
                    #[cfg(feature = "wl_clipboard")]
                    if let Some(content) = saved_clipboard.clone() {
                        let delay_ms = self.config.clipboard_restore_delay_ms.unwrap_or(500);
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
                    let elapsed = start.elapsed();
                    debug!(
                        "AT-SPI paste succeeded; clipboard injection completed in {}ms",
                        elapsed.as_millis()
                    );
                    return Ok(());
                }
                Ok(Err(e)) => {
                    debug!("AT-SPI paste failed; evaluating ydotool fallback: {}", e);
                }
                Err(_) => {
                    debug!("AT-SPI paste timed out; evaluating ydotool fallback");
                }
            }
        }

        // ydotool fallback (optional)
        if Self::ydotool_available().await {
            let paste_start = Instant::now();
            let output = timeout(
                Duration::from_millis(self.config.paste_action_timeout_ms),
                Command::new("ydotool").args(["key", "ctrl+v"]).output(),
            )
            .await
            .map_err(|_| {
                crate::types::InjectionError::Timeout(self.config.paste_action_timeout_ms)
            })?
            .map_err(|e| crate::types::InjectionError::Process(format!("ydotool failed: {e}")))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(crate::types::InjectionError::MethodFailed(format!(
                    "ydotool paste failed: {stderr}"
                )));
            }

            debug!(
                "Paste triggered via ydotool in {}ms",
                paste_start.elapsed().as_millis()
            );
        } else {
            return Err(crate::types::InjectionError::MethodFailed(
                "AT-SPI paste failed and ydotool is unavailable".to_string(),
            ));
        }

        #[cfg(feature = "wl_clipboard")]
        if let Some(content) = saved_clipboard {
            let delay_ms = self.config.clipboard_restore_delay_ms.unwrap_or(500);
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

        let elapsed = start.elapsed();
        debug!(
            "ClipboardPasteInjector completed in {}ms",
            elapsed.as_millis()
        );

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

impl ClipboardPasteInjector {
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
