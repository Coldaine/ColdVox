//! # Robust Subprocess Execution
//!
//! This module provides helpers for running external commands with strict
//! timeouts, ensuring that no process can hang indefinitely and block the
//! application. This is critical for interacting with potentially unreliable
//! command-line tools like `wl-paste` or `xclip`.

use crate::error::ClipboardError;
use std::time::Duration;
use tokio::io::AsyncWriteExt;

/// Runs a command and captures its stdout, with a strict timeout.
///
/// If the command takes longer than the specified duration, it is killed
/// and a `ClipboardError::Timeout` is returned. This is achieved by setting
/// `kill_on_drop(true)` on the child process.
///
/// ## Arguments
/// * `cmd` - The command to execute.
/// * `args` - The arguments for the command.
/// * `ms` - The timeout in milliseconds.
///
/// ## Returns
/// * `Ok(String)` - The captured stdout content as a string.
/// * `Err(ClipboardError)` - If the command fails, times out, or produces invalid UTF-8.
pub async fn run_tool_with_timeout(
    cmd: &str,
    args: &[&str],
    ms: u64,
) -> Result<String, ClipboardError> {
    let mut command = tokio::process::Command::new(cmd);
    command
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true); // This is the key to ensuring cleanup on timeout.

    let child = command
        .spawn()
        .map_err(|e| ClipboardError::Launch(e.to_string()))?;

    match tokio::time::timeout(Duration::from_millis(ms), child.wait_with_output()).await {
        Ok(Ok(output)) => {
            if output.status.success() {
                String::from_utf8(output.stdout).map_err(|_| ClipboardError::InvalidUtf8)
            } else {
                Err(ClipboardError::Launch(format!(
                    "{} exited with status {}",
                    cmd, output.status
                )))
            }
        }
        Ok(Err(e)) => Err(ClipboardError::Io(e)),
        Err(_) => Err(ClipboardError::Timeout),
    }
}

/// Runs a command, writes data to its stdin, and waits for it to complete,
/// with a strict timeout.
///
/// This is useful for commands like `wl-copy` or `xclip -i`. The timeout is
/// split between the write and wait operations.
///
/// ## Arguments
/// * `cmd` - The command to execute.
/// * `args` - The arguments for the command.
/// * `input` - The data to write to the command's stdin.
/// * `ms` - The total timeout in milliseconds.
pub async fn run_tool_with_stdin_timeout(
    cmd: &str,
    args: &[&str],
    input: &[u8],
    ms: u64,
) -> Result<(), ClipboardError> {
    let mut command = tokio::process::Command::new(cmd);
    command
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);

    let mut child = command
        .spawn()
        .map_err(|e| ClipboardError::Launch(e.to_string()))?;

    if let Some(mut stdin) = child.stdin.take() {
        // Give half the budget to writing to stdin.
        match tokio::time::timeout(Duration::from_millis(ms / 2), stdin.write_all(input)).await {
            Ok(Ok(_)) => {
                // Drop stdin to signal EOF to the child process.
                drop(stdin);
            }
            _ => return Err(ClipboardError::Timeout),
        }
    }

    // Give the other half of the budget to waiting for the process to exit.
    match tokio::time::timeout(Duration::from_millis(ms.saturating_sub(ms / 2)), child.wait()).await
    {
        Ok(Ok(status)) => {
            if status.success() {
                Ok(())
            } else {
                Err(ClipboardError::Launch(format!(
                    "{} exited with status {}",
                    cmd, status
                )))
            }
        }
        Ok(Err(e)) => Err(ClipboardError::Io(e)),
        Err(_) => Err(ClipboardError::Timeout),
    }
}
