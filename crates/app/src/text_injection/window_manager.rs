use crate::text_injection::types::InjectionError;
use std::process::Command;
use tracing::debug;
use serde_json;

/// Get the currently active window class name
pub async fn get_active_window_class() -> Result<String, InjectionError> {
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
    
    Err(InjectionError::Other("Could not determine active window".to_string()))
}

async fn get_kde_window_class() -> Result<String, InjectionError> {
    // Use KWin DBus interface
    let output = Command::new("qdbus")
        .args(&[
            "org.kde.KWin",
            "/KWin",
            "org.kde.KWin.activeClient"
        ])
        .output()
        .map_err(|e| InjectionError::Process(format!("qdbus failed: {}", e)))?;
    
    if output.status.success() {
        let window_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        
        // Get window class from ID
        let class_output = Command::new("qdbus")
            .args(&[
                "org.kde.KWin",
                &format!("/Windows/{}", window_id),
                "org.kde.KWin.Window.resourceClass"
            ])
            .output()
            .map_err(|e| InjectionError::Process(format!("qdbus failed: {}", e)))?;
        
        if class_output.status.success() {
            return Ok(String::from_utf8_lossy(&class_output.stdout).trim().to_string());
        }
    }
    
    Err(InjectionError::Other("KDE window class not available".to_string()))
}

async fn get_x11_window_class() -> Result<String, InjectionError> {
    // Use xprop to get active window class
    let output = Command::new("xprop")
        .args(&["-root", "_NET_ACTIVE_WINDOW"])
        .output()
        .map_err(|e| InjectionError::Process(format!("xprop failed: {}", e)))?;
    
    if output.status.success() {
        let window_str = String::from_utf8_lossy(&output.stdout);
        if let Some(window_id) = window_str.split("# ").nth(1) {
            let window_id = window_id.trim();
            
            // Get window class
            let class_output = Command::new("xprop")
                .args(&["-id", window_id, "WM_CLASS"])
                .output()
                .map_err(|e| InjectionError::Process(format!("xprop failed: {}", e)))?;
            
            if class_output.status.success() {
                let class_str = String::from_utf8_lossy(&class_output.stdout);
                // Parse WM_CLASS string (format: WM_CLASS(STRING) = "instance", "class")
                if let Some(class_part) = class_str.split('"').nth(3) {
                    return Ok(class_part.to_string());
                }
            }
        }
    }
    
    Err(InjectionError::Other("X11 window class not available".to_string()))
}

async fn get_wayland_window_class() -> Result<String, InjectionError> {
    // Try using wlr-foreign-toplevel-management protocol if available
    // This requires compositor support (e.g., Sway, some KWin versions)
    
    // For now, we'll try using swaymsg if Sway is running
    let output = Command::new("swaymsg")
        .args(&["-t", "get_tree"])
        .output()
        .map_err(|e| InjectionError::Process(format!("swaymsg failed: {}", e)))?;
    
    if output.status.success() {
        // Parse JSON to find focused window using serde_json
        let tree = String::from_utf8_lossy(&output.stdout);
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&tree) {
            // Depth-first search for focused node with app_id
            fn dfs(node: &serde_json::Value) -> Option<String> {
                if node.get("focused").and_then(|v| v.as_bool()).unwrap_or(false) {
                    if let Some(app_id) = node.get("app_id").and_then(|v| v.as_str()) {
                        return Some(app_id.to_string());
                    }
                    if let Some(window_props) = node.get("window_properties") {
                        if let Some(class) = window_props.get("class").and_then(|v| v.as_str()) {
                            return Some(class.to_string());
                        }
                    }
                }
                if let Some(nodes) = node.get("nodes").and_then(|v| v.as_array()) {
                    for n in nodes {
                        if let Some(found) = dfs(n) { return Some(found); }
                    }
                }
                if let Some(floating_nodes) = node.get("floating_nodes").and_then(|v| v.as_array()) {
                    for n in floating_nodes {
                        if let Some(found) = dfs(n) { return Some(found); }
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
    
    Err(InjectionError::Other("Wayland window class not available".to_string()))
}

/// Get window information using multiple methods
pub async fn get_window_info() -> WindowInfo {
    let class = get_active_window_class().await.unwrap_or_else(|_| "unknown".to_string());
    let title = get_window_title().await.unwrap_or_default();
    let pid = get_window_pid().await.unwrap_or(0);
    
    WindowInfo {
        class,
        title,
        pid,
    }
}

/// Window information structure
#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub class: String,
    pub title: String,
    pub pid: u32,
}

/// Get the title of the active window
async fn get_window_title() -> Result<String, InjectionError> {
    // Try X11 method
    let output = Command::new("xprop")
        .args(&["-root", "_NET_ACTIVE_WINDOW"])
        .output()
        .map_err(|e| InjectionError::Process(format!("xprop failed: {}", e)))?;
    
    if output.status.success() {
        let window_str = String::from_utf8_lossy(&output.stdout);
        if let Some(window_id) = window_str.split("# ").nth(1) {
            let window_id = window_id.trim();
            
            // Get window title
            let title_output = Command::new("xprop")
                .args(&["-id", window_id, "_NET_WM_NAME"])
                .output()
                .map_err(|e| InjectionError::Process(format!("xprop failed: {}", e)))?;
            
            if title_output.status.success() {
                let title_str = String::from_utf8_lossy(&title_output.stdout);
                // Parse title string
                if let Some(title_start) = title_str.find(" = \"") {
                    let title = &title_str[title_start + 4..];
                    if let Some(title_end) = title.find('"') {
                        return Ok(title[..title_end].to_string());
                    }
                }
            }
        }
    }
    
    Err(InjectionError::Other("Could not get window title".to_string()))
}

/// Get the PID of the active window
async fn get_window_pid() -> Result<u32, InjectionError> {
    // Try X11 method
    let output = Command::new("xprop")
        .args(&["-root", "_NET_ACTIVE_WINDOW"])
        .output()
        .map_err(|e| InjectionError::Process(format!("xprop failed: {}", e)))?;
    
    if output.status.success() {
        let window_str = String::from_utf8_lossy(&output.stdout);
        if let Some(window_id) = window_str.split("# ").nth(1) {
            let window_id = window_id.trim();
            
            // Get window PID
            let pid_output = Command::new("xprop")
                .args(&["-id", window_id, "_NET_WM_PID"])
                .output()
                .map_err(|e| InjectionError::Process(format!("xprop failed: {}", e)))?;
            
            if pid_output.status.success() {
                let pid_str = String::from_utf8_lossy(&pid_output.stdout);
                // Parse PID (format: _NET_WM_PID(CARDINAL) = <pid>)
                if let Some(pid_part) = pid_str.split(" = ").nth(1) {
                    if let Ok(pid) = pid_part.trim().parse::<u32>() {
                        return Ok(pid);
                    }
                }
            }
        }
    }
    
    Err(InjectionError::Other("Could not get window PID".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_window_detection() {
        // This test will only work in a graphical environment
        if std::env::var("DISPLAY").is_ok() || std::env::var("WAYLAND_DISPLAY").is_ok() {
            let result = get_active_window_class().await;
            // We can't assert success since it depends on the environment
            // but we can check that it doesn't panic
            match result {
                Ok(class) => {
                    debug!("Detected window class: {}", class);
                    assert!(!class.is_empty());
                }
                Err(e) => {
                    debug!("Window detection failed (expected in CI): {}", e);
                }
            }
        }
    }
    
    #[tokio::test]
    async fn test_window_info() {
        let info = get_window_info().await;
        // Basic sanity check
        assert!(!info.class.is_empty());
    }
}