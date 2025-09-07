//! Environment probing to determine available injection backends and system state.

// Using heapless for the metrics samples vector as requested to avoid std::collections::HashMap
// and keep dependencies minimal. Let's add it to the `use` statements.
// Ah, wait, that's for metrics. This file is for probing. I'll stick to the proposal's definitions.

use serde::Serialize;

/// Unique identifier for each text injection backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum BackendId {
    Atspi,
    ClipboardWayland,
    ClipboardX11,
    Ydotool,
    Fallback, // Represents a failure to select any specific backend.
}

/// Represents the readiness of the environment for text injection.
/// This is determined by a single, fast probe at the start of an injection attempt.
#[derive(Debug, Serialize)]
pub enum ProbeState {
    /// The environment is fully configured and ready for injection.
    FullyAvailable {
        /// A list of backends that are ready to be used.
        usable: Vec<BackendId>,
    },
    /// The environment is partially functional. Some backends are available.
    Degraded {
        /// A list of backends that can still be used.
        usable: Vec<BackendId>,
        /// A list of components that are missing or misconfigured.
        missing: Vec<(&'static str, &'static str)>, // (component, reason)
    },
    /// The environment is not suitable for text injection.
    Missing {
        /// A list of reasons why injection is not possible.
        causes: Vec<(&'static str, &'static str)>, // (component, reason)
    },
}

/// Asynchronously probes the environment to determine injection readiness.
///
use crate::constants::SUBPROCESS_PROBE_TIMEOUT_MS;
use std::time::Duration;

/// This function is designed to be fast and non-blocking, with strict timeouts
/// on all potentially blocking operations (like subprocess calls or D-Bus pings).
pub async fn probe_environment() -> ProbeState {
    // 1. Check for essential environment variables.
    let have_x = std::env::var("DISPLAY").is_ok();
    let have_wayland = std::env::var("WAYLAND_DISPLAY").is_ok();
    let have_dbus = std::env::var("DBUS_SESSION_BUS_ADDRESS").is_ok();

    let mut usable = Vec::new();
    let mut missing = Vec::new();

    // 2. Check for D-Bus and AT-SPI registry liveness.
    if have_dbus {
        // The actual ping requires an async D-Bus connection.
        // We'll use a placeholder for now and add a TODO.
        if atspi_registry_ping().await {
            usable.push(BackendId::Atspi);
        } else {
            missing.push(("at-spi", "registry ping failed or timed out"));
        }
    } else {
        missing.push(("dbus", "DBUS_SESSION_BUS_ADDRESS not set"));
    }

    // 3. Check for clipboard tools.
    let wl_ok =
        quick_subprocess_ok("wl-paste", &["--version"], SUBPROCESS_PROBE_TIMEOUT_MS).await;
    let xclip_ok = quick_subprocess_ok("xclip", &["-version"], SUBPROCESS_PROBE_TIMEOUT_MS).await;

    if have_wayland && wl_ok {
        usable.push(BackendId::ClipboardWayland);
    }
    if have_x && xclip_ok {
        usable.push(BackendId::ClipboardX11);
    }

    if usable
        .iter()
        .all(|b| !matches!(b, BackendId::ClipboardWayland | BackendId::ClipboardX11))
    {
        missing.push(("clipboard", "no usable clipboard tool found (wl-paste/xclip)"));
    }

    // 4. Classify the final state.
    if usable.is_empty() {
        ProbeState::Missing { causes: missing }
    } else if !missing.is_empty() {
        ProbeState::Degraded { usable, missing }
    } else {
        ProbeState::FullyAvailable { usable }
    }
}

/// A placeholder for a real AT-SPI registry ping.
///
/// TODO: Implement this using an async D-Bus call with a short timeout.
/// This will require adding `zbus` as a dependency.
async fn atspi_registry_ping() -> bool {
    // For now, we assume it's available if the D-Bus session exists.
    // A real implementation would try to connect and call a method.
    // We'll simulate a small delay to represent a real check.
    tokio::time::sleep(Duration::from_millis(10)).await;
    true // Placeholder
}

/// Quickly checks if a command can be spawned and then immediately terminates it.
/// This is used to verify the presence and basic functionality of external tools
/// without waiting for them to complete any real work.
///
/// ## Arguments
/// * `cmd` - The command to check.
/// * `args` - Arguments to pass to the command.
/// * `budget_ms` - The maximum time to allow for this check.
///
/// ## Returns
/// * `true` - If the command spawns and can be terminated within the budget.
/// * `false` - Otherwise.
pub async fn quick_subprocess_ok(cmd: &str, args: &[&str], budget_ms: u64) -> bool {
    let cmd_str = cmd.to_string();
    let args_owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();

    let check = async move {
        let mut child = tokio::process::Command::new(&cmd_str)
            .args(&args_owned)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .ok()?;

        // We don't need the command to run, just to spawn. Kill it immediately.
        // The `start_kill` is preferred as it's non-blocking.
        child.start_kill().ok()?;
        // `wait` is still necessary to clean up the zombie process.
        child.wait().await.ok()?;
        Some(())
    };

    tokio::time::timeout(std::time::Duration::from_millis(budget_ms), check)
        .await
        .is_ok()
}
