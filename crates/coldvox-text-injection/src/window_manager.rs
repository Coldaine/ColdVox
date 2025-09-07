use crate::error::InjectionError;
use std::process::Command;
use tracing::debug;

/// Get the currently active window class name
pub async fn get_active_window_class() -> Result<String, InjectionError> {
    // This function shells out to external commands. This is not ideal and
    // should be replaced with direct D-Bus/X11 library calls where possible.
    // For now, we accept the brittleness. The new architecture does not rely
    // on this for the core injection path.

    // Try KDE-specific method first
    if let Ok(class) = get_kde_window_class().await {
        return Ok(class);
    }

    // Try generic X11 method
    if let Ok(class) = get_x11_window_class().await {
        return Ok(class);
    }

    // Try Wayland method
    if let Ok(class) = get_wayland_window_class().await {
        return Ok(class);
    }

    Err(InjectionError::Other(
        "Could not determine active window".to_string(),
    ))
}

async fn get_kde_window_class() -> Result<String, InjectionError> {
    let output = Command::new("qdbus")
        .args(["org.kde.KWin", "/KWin", "org.kde.KWin.activeClient"])
        .output()
        .map_err(|e| InjectionError::Io {
            backend: crate::probe::BackendId::Fallback,
            msg: format!("qdbus failed: {}", e),
        })?;

    if output.status.success() {
        let window_id = String::from_utf8_lossy(&output.stdout).trim().to_string();

        let class_output = Command::new("qdbus")
            .args([
                "org.kde.KWin",
                &format!("/Windows/{}", window_id),
                "org.kde.KWin.Window.resourceClass",
            ])
            .output()
            .map_err(|e| InjectionError::Io {
                backend: crate::probe::BackendId::Fallback,
                msg: format!("qdbus failed: {}", e),
            })?;

        if class_output.status.success() {
            return Ok(String::from_utf8_lossy(&class_output.stdout)
                .trim()
                .to_string());
        }
    }

    Err(InjectionError::Other(
        "KDE window class not available".to_string(),
    ))
}

async fn get_x11_window_class() -> Result<String, InjectionError> {
    let output = Command::new("xprop")
        .args(["-root", "_NET_ACTIVE_WINDOW"])
        .output()
        .map_err(|e| InjectionError::Io {
            backend: crate::probe::BackendId::Fallback,
            msg: format!("xprop failed: {}", e),
        })?;

    if output.status.success() {
        let window_str = String::from_utf8_lossy(&output.stdout);
        if let Some(window_id) = window_str.split("# ").nth(1) {
            let window_id = window_id.trim();

            let class_output = Command::new("xprop")
                .args(["-id", window_id, "WM_CLASS"])
                .output()
                .map_err(|e| InjectionError::Io {
                    backend: crate::probe::BackendId::Fallback,
                    msg: format!("xprop failed: {}", e),
                })?;

            if class_output.status.success() {
                let class_str = String::from_utf8_lossy(&class_output.stdout);
                if let Some(class_part) = class_str.split('"').nth(3) {
                    return Ok(class_part.to_string());
                }
            }
        }
    }

    Err(InjectionError::Other(
        "X11 window class not available".to_string(),
    ))
}

async fn get_wayland_window_class() -> Result<String, InjectionError> {
    let output = Command::new("swaymsg")
        .args(["-t", "get_tree"])
        .output()
        .map_err(|e| InjectionError::Io {
            backend: crate::probe::BackendId::Fallback,
            msg: format!("swaymsg failed: {}", e),
        })?;

    if output.status.success() {
        let tree = String::from_utf8_lossy(&output.stdout);
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&tree) {
            fn dfs(node: &serde_json::Value) -> Option<String> {
                if node.get("focused").and_then(|v| v.as_bool()).unwrap_or(false) {
                    if let Some(app_id) = node.get("app_id").and_then(|v| v.as_str()) {
                        return Some(app_id.to_string());
                    }
                }
                if let Some(nodes) = node.get("nodes").and_then(|v| v.as_array()) {
                    for n in nodes {
                        if let Some(found) = dfs(n) {
                            return Some(found);
                        }
                    }
                }
                None
            }
            if let Some(app_id) = dfs(&json) {
                return Ok(app_id);
            }
        } else {
            debug!("Failed to parse swaymsg JSON; falling back");
        }
    }

    Err(InjectionError::Other(
        "Wayland window class not available".to_string(),
    ))
}
