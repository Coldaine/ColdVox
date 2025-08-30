#![cfg(feature = "text-injection-ydotool")]

use crate::text_injection::{InjectionError, TextInjector};
use anyhow::{anyhow, Result};
use std::process::Command;

/// Demo of ydotool integration (last resort; opt-in).
/// NOTE: Requires `ydotoold` running and uinput permissions. We shell out.
#[derive(Debug, Default)]
pub struct YdotoolInjector;

impl YdotoolInjector {
    pub fn new() -> Self { Self }

    fn ydotool_present() -> bool {
        // A quick probe; not perfect but fine for demo.
        Command::new("ydotool").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
    }
}

impl TextInjector for YdotoolInjector {
    fn name(&self) -> &'static str { "ydotool" }

    fn is_available(&self) -> bool { Self::ydotool_present() }

    fn inject(&self, text: &str) -> Result<()> {
        // On real use we’d escape special chars. For demo purposes only:
        let status = Command::new("ydotool")
            .arg("type")
            .arg("--")
            .arg(text)
            .status();

        match status {
            Ok(s) if s.success() => Ok(()),
            Ok(s) => Err(anyhow!(InjectionError::PermissionDenied(format!(
                "ydotool exited with status {s}"
            )))),
            Err(e) => Err(anyhow!(InjectionError::MethodNotAvailable(format!(
                "ydotool not runnable: {e}"
            )))),
        }
    }
}
