//! Shared helpers for integration tests that exercise clipboard behavior.

use std::process::Command as StdCommand;
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::process::Command as TokioCommand;

const DEFAULT_CLIPBOARD_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_XCLIP_TIMEOUT: Duration = Duration::from_secs(5);

/// Best-effort detection of a Wayland environment, falling back to wl-paste availability.
pub fn is_wayland_environment() -> bool {
    if std::env::var("WAYLAND_DISPLAY")
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false)
    {
        return true;
    }

    if std::env::var("XDG_SESSION_TYPE")
        .map(|v| v.eq_ignore_ascii_case("wayland"))
        .unwrap_or(false)
    {
        return true;
    }

    command_exists("wl-paste")
}

/// Best-effort detection of an X11 (or XWayland) environment.
pub fn is_x11_environment() -> bool {
    if std::env::var("DISPLAY")
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false)
    {
        return true;
    }

    std::env::var("XDG_SESSION_TYPE")
        .map(|v| v.eq_ignore_ascii_case("x11"))
        .unwrap_or(false)
}

/// Check whether a command is available on the current PATH.
pub fn command_exists(cmd: &str) -> bool {
    StdCommand::new("which")
        .arg(cmd)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Read text/plain clipboard content using wl-paste with a default timeout.
pub async fn read_clipboard_with_wl_paste() -> Result<String> {
    read_clipboard_with_wl_paste_with_timeout(DEFAULT_CLIPBOARD_TIMEOUT).await
}

/// Read text/plain clipboard content using wl-paste, timing out after the provided duration.
pub async fn read_clipboard_with_wl_paste_with_timeout(limit: Duration) -> Result<String> {
    let command = TokioCommand::new("wl-paste")
        .args(["--type", "text/plain"])
        .output();

    let output = tokio::time::timeout(limit, command)
        .await
        .context("Timed out waiting for wl-paste output")??;

    if !output.status.success() {
        anyhow::bail!("wl-paste exited with status {:?}", output.status);
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Read clipboard content via xclip (X11) with a default timeout.
pub async fn read_clipboard_with_xclip() -> Result<String> {
    read_clipboard_with_xclip_with_timeout(DEFAULT_XCLIP_TIMEOUT).await
}

/// Read clipboard content via xclip (X11) using the provided timeout.
pub async fn read_clipboard_with_xclip_with_timeout(limit: Duration) -> Result<String> {
    let command = TokioCommand::new("xclip")
        .args(["-selection", "clipboard", "-out"])
        .output();

    let output = tokio::time::timeout(limit, command)
        .await
        .context("Timed out waiting for xclip output")??;

    if !output.status.success() {
        anyhow::bail!("xclip exited with status {:?}", output.status);
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
