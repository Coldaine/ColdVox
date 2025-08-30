#![cfg(feature = "text-injection-clipboard")]

use super::common::{InjectionError, TextInjector};
use anyhow::{anyhow, Result};
use wl_clipboard_rs::copy::{self, Options};

#[derive(Debug, Default)]
pub struct ClipboardInjector;

impl ClipboardInjector {
    pub fn new() -> Self {
        Self
    }
}

impl TextInjector for ClipboardInjector {
    fn name(&self) -> &'static str {
        "Wayland-Clipboard"
    }

    fn is_available(&self) -> bool {
        std::env::var("WAYLAND_DISPLAY").is_ok()
    }

    fn inject(&self, text: &str) -> Result<()> {
        let opts = Options::new();
        opts.copy(
            copy::Source::Bytes(text.as_bytes().to_vec().into()),
            copy::MimeType::Autodetect,
        )
        .map_err(|e| anyhow!(InjectionError::InjectionFailed(e.to_string())))
    }
}
