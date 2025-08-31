#![cfg(feature = "text-injection-clipboard-x11")]

use crate::text_injection::{InjectionError, TextInjector};
use crate::text_injection::types::InjectionMetrics;
use anyhow::{anyhow, Result};
use std::env;
use async_trait::async_trait;

/// X11/XWayland clipboard injector using arboard.
/// This is functional for X11; on Wayland we keep it out of the default order.
#[derive(Debug)]
pub struct X11ClipboardInjector {
    metrics: InjectionMetrics,
}

impl X11ClipboardInjector {
    pub fn new() -> Self {
        Self::default()
    }

    fn on_x11() -> bool {
        env::var_os("DISPLAY").is_some() && env::var_os("WAYLAND_DISPLAY").is_none()
    }
}

impl Default for X11ClipboardInjector {
    fn default() -> Self {
        Self {
            metrics: InjectionMetrics::default(),
        }
    }
}

#[async_trait]
impl TextInjector for X11ClipboardInjector {
    fn name(&self) -> &'static str {
        "Clipboard-X11"
    }

    fn is_available(&self) -> bool {
        Self::on_x11()
    }

    async fn inject(&mut self, text: &str) -> Result<(), InjectionError> {
        if !self.is_available() {
            return Err(InjectionError::MethodNotAvailable(
                "Not an X11 session (DISPLAY missing or Wayland present)".into()
            ));
        }

        // Functional demo (safe to run): set clipboard text.
        let mut cb = arboard::Clipboard::new().map_err(|e| {
            InjectionError::PermissionDenied(format!("clipboard init failed: {e}"))
        })?;

        cb.set_text(text.to_string()).map_err(|e| {
            InjectionError::PermissionDenied(format!("clipboard set failed: {e}"))
        })?;

        Ok(())
    }

    fn metrics(&self) -> &InjectionMetrics {
        &self.metrics
    }
}
