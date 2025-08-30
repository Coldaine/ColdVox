#![cfg(feature = "text-injection-clipboard-x11")]

use crate::text_injection::{InjectionError, TextInjector};
use anyhow::{anyhow, Result};
use std::env;

/// X11/XWayland clipboard injector using `arboard`.
/// This is functional for X11; on Wayland we keep it out of the default order.
#[derive(Debug, Default)]
pub struct X11ClipboardInjector;

impl X11ClipboardInjector {
    pub fn new() -> Self { Self }

    fn on_x11() -> bool {
        env::var_os("DISPLAY").is_some() && env::var_os("WAYLAND_DISPLAY").is_none()
    }
}

impl TextInjector for X11ClipboardInjector {
    fn name(&self) -> &'static str { "Clipboard-X11" }

    fn is_available(&self) -> bool { Self::on_x11() }

    fn inject(&self, text: &str) -> Result<()> {
        if !self.is_available() {
            return Err(anyhow!(InjectionError::MethodNotAvailable(
                "Not an X11 session (DISPLAY missing or Wayland present)".into()
            )));
        }

        // Functional demo (safe to run): set clipboard text.
        let mut cb = arboard::Clipboard::new().map_err(|e| {
            anyhow!(InjectionError::PermissionDenied(format!("clipboard init failed: {e}")))
        })?;

        cb.set_text(text.to_string()).map_err(|e| {
            anyhow!(InjectionError::PermissionDenied(format!("clipboard set failed: {e}")))
        })?;

        Ok(())
    }
}
