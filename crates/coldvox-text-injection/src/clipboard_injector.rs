//! # Clipboard-based Text Injector
//!
//! This module provides a text injection strategy that works by setting the
//! system clipboard content. It is designed to be robust, using timeouts for all
//! external tool interactions to prevent hangs. It supports both Wayland and X11
//! environments by wrapping their respective command-line clipboard utilities.

use crate::constants::CLIPBOARD_TOOL_TIMEOUT_MS;
use crate::error::{ClipboardError, InjectionError, UnavailableCause};
use crate::outcome::InjectionOutcome;
use crate::probe::BackendId;
use crate::subprocess::{run_tool_with_stdin_timeout, run_tool_with_timeout};
use crate::{async_trait, InjectionConfig, TextInjector};
use std::env;
use std::time::Instant;

/// An internal trait for abstracting over different clipboard backends.
#[async_trait]
trait Clipboard: Send + Sync {
    async fn get(&self) -> Result<String, ClipboardError>;
    async fn set(&self, text: &str) -> Result<(), ClipboardError>;
}

/// A clipboard implementation for Wayland using `wl-copy` and `wl-paste`.
struct WlClipboard;

#[async_trait]
impl Clipboard for WlClipboard {
    async fn get(&self) -> Result<String, ClipboardError> {
        run_tool_with_timeout("wl-paste", &["--no-newline"], CLIPBOARD_TOOL_TIMEOUT_MS).await
    }

    async fn set(&self, text: &str) -> Result<(), ClipboardError> {
        run_tool_with_stdin_timeout("wl-copy", &[], text.as_bytes(), CLIPBOARD_TOOL_TIMEOUT_MS)
            .await
    }
}

/// A clipboard implementation for X11 using `xclip`.
struct X11Clipboard;

#[async_trait]
impl Clipboard for X11Clipboard {
    async fn get(&self) -> Result<String, ClipboardError> {
        run_tool_with_timeout("xclip", &["-selection", "clipboard", "-o"], CLIPBOARD_TOOL_TIMEOUT_MS)
            .await
    }

    async fn set(&self, text: &str) -> Result<(), ClipboardError> {
        run_tool_with_stdin_timeout(
            "xclip",
            &["-selection", "clipboard", "-i"],
            text.as_bytes(),
            CLIPBOARD_TOOL_TIMEOUT_MS,
        )
        .await
    }
}

/// The main injector struct that orchestrates clipboard operations.
pub struct ClipboardInjector {
    config: InjectionConfig,
    backend_id: BackendId,
    clipboard_impl: Box<dyn Clipboard>,
}

impl ClipboardInjector {
    /// Creates a new `ClipboardInjector`.
    /// It detects the environment (Wayland/X11) and selects the appropriate backend.
    /// Returns `None` if no suitable clipboard environment is detected.
    pub fn new(config: InjectionConfig) -> Option<Self> {
        let is_wayland = env::var("WAYLAND_DISPLAY").is_ok();
        let is_x11 = env::var("DISPLAY").is_ok();

        if is_wayland {
            Some(Self {
                config,
                backend_id: BackendId::ClipboardWayland,
                clipboard_impl: Box::new(WlClipboard),
            })
        } else if is_x11 {
            Some(Self {
                config,
                backend_id: BackendId::ClipboardX11,
                clipboard_impl: Box::new(X11Clipboard),
            })
        } else {
            None
        }
    }

    /// Saves the current clipboard content.
    async fn save_clipboard_content(&self) -> Result<String, ClipboardError> {
        self.clipboard_impl.get().await
    }

    /// Restores the given content to the clipboard.
    async fn restore_clipboard_content(&self, content: &str) -> Result<(), ClipboardError> {
        self.clipboard_impl.set(content).await
    }
}

#[async_trait]
impl TextInjector for ClipboardInjector {
    fn backend_id(&self) -> BackendId {
        self.backend_id
    }

    async fn is_available(&self) -> bool {
        // The existence of the injector implies the display variable was set.
        // A full check is done by the environment probe. This is a quick sanity check.
        match self.backend_id {
            BackendId::ClipboardWayland => self.clipboard_impl.set("").await.is_ok(),
            BackendId::ClipboardX11 => self.clipboard_impl.set("").await.is_ok(),
            _ => false,
        }
    }

    async fn inject_text(&self, text: &str) -> Result<InjectionOutcome, InjectionError> {
        let start_time = Instant::now();
        let original_clipboard = if self.config.restore_clipboard {
            self.save_clipboard_content().await.ok()
        } else {
            None
        };

        // Set the new clipboard content.
        self.clipboard_impl
            .set(text)
            .await
            .map_err(|e| InjectionError::Io {
                backend: self.backend_id,
                msg: e.to_string(),
            })?;

        // If we need to restore, check if it worked.
        let mut degraded = false;
        if let Some(original) = original_clipboard {
            // NOTE: In a real scenario, the paste action happens here, initiated
            // by another injector (e.g., a ydotool or atspi action injector).
            // For now, we simulate the paste delay before restoring.
            // A proper implementation would have the StrategyManager coordinate this.
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;

            if let Err(e) = self.restore_clipboard_content(&original).await {
                degraded = true;
                let error_msg = format!("Failed to restore clipboard: {}", e);
                // Unless enforced, this is just a warning (degraded outcome).
                if self.config.enforce_clipboard_restore {
                    return Err(InjectionError::ClipboardRestoreMismatch {
                        details: error_msg,
                    });
                }
            } else {
                // Final check to see if the content matches.
                let current_clipboard = self.save_clipboard_content().await.unwrap_or_default();
                if current_clipboard != original {
                    degraded = true;
                    if self.config.enforce_clipboard_restore {
                        return Err(InjectionError::ClipboardRestoreMismatch {
                            details: "Clipboard content mismatch after restore.".to_string(),
                        });
                    }
                }
            }
        }

        let latency_ms = start_time.elapsed().as_millis() as u32;
        Ok(InjectionOutcome {
            backend: self.backend_id,
            latency_ms,
            degraded,
        })
    }
}
