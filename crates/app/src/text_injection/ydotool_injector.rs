#![cfg(feature = "text-injection-ydotool")]

use crate::text_injection::{InjectionError, TextInjector};
use crate::text_injection::types::InjectionMetrics;
use anyhow::{anyhow, Result};
use std::process::Command;
use async_trait::async_trait;

/// Demo of ydotool integration (last resort; opt-in).
/// NOTE: Requires ydotoold running and uinput permissions. We shell out.
#[derive(Debug)]
pub struct YdotoolInjector {
    metrics: InjectionMetrics,
}

impl YdotoolInjector {
    pub fn new() -> Self {
        Self::default()
    }

    fn ydotool_present() -> bool {
        // A quick probe; not perfect but fine for demo.
        Command::new("ydotool")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

impl Default for YdotoolInjector {
    fn default() -> Self {
        Self {
            metrics: InjectionMetrics::default(),
        }
    }
}

#[async_trait]
impl TextInjector for YdotoolInjector {
    fn name(&self) -> &'static str {
        "ydotool"
    }

    fn is_available(&self) -> bool {
        Self::ydotool_present()
    }

    async fn inject(&mut self, text: &str) -> Result<(), InjectionError> {
        // On real use we’d escape special chars. For demo purposes only:
        let status = Command::new("ydotool")
            .arg("type")
            .arg("--")
            .arg(text)
            .status();

        match status {
            Ok(s) if s.success() => Ok(()),
            Ok(s) => Err(InjectionError::PermissionDenied(format!(
                "ydotool exited with status {s}"
            ))),
            Err(e) => Err(InjectionError::MethodNotAvailable(format!(
                "ydotool not runnable: {e}"
            ))),
        }
    }

    fn metrics(&self) -> &InjectionMetrics {
        &self.metrics
    }
}
