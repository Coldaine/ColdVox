use anyhow::Result;
use std::process::Command;
use std::process::Stdio;
use std::time::Duration;
use tracing::{info, warn, debug, error};
use tokio::io::AsyncWriteExt;
// Session-based injection modules
pub mod session;
pub mod processor;

// Re-export session management types
pub use session::{InjectionSession, SessionConfig, SessionState};
pub use processor::{InjectionProcessor, AsyncInjectionProcessor, InjectionProcessorConfig, InjectionMetrics};

/// Production-ready text injection for KDE Plasma Wayland
/// Based on 2024-2025 best practices
pub struct TextInjector {
    use_ydotool: bool,
    use_kdotool: bool,
    has_uinput_access: bool,
}

impl TextInjector {
    pub fn new() -> Self {
        // Check ydotool availability and service status
        let use_ydotool = Self::check_ydotool();
        
        // Check kdotool for focus detection
        let use_kdotool = Command::new("which")
            .arg("kdotool")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        
        // Check uinput access
        let has_uinput_access = Self::check_uinput_access();
        
        // Log capabilities
        info!("Text injection capabilities:");
        info!("  ydotool: {}", if use_ydotool { "available" } else { "not available" });
        info!("  kdotool: {}", if use_kdotool { "available" } else { "not available" });
        info!("  uinput access: {}", if has_uinput_access { "yes" } else { "no" });
        
        if !use_ydotool && has_uinput_access {
            warn!("uinput access available but ydotool not found - consider installing ydotool");
        }
        
        Self { 
            use_ydotool,
            use_kdotool,
            has_uinput_access,
        }
    }
    
    /// Check if ydotool is available and properly configured
    fn check_ydotool() -> bool {
        // First check if binary exists
        let binary_exists = Command::new("which")
            .arg("ydotool")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        
        if !binary_exists {
            return false;
        }
        
        // Check if the ydotool socket exists (most reliable check)
        let user_id = std::env::var("UID").unwrap_or_else(|_| "1000".to_string());
        let socket_path = format!("/run/user/{}/.ydotool_socket", user_id);
        if std::path::Path::new(&socket_path).exists() {
            debug!("ydotool socket found at {}", socket_path);
            return true;
        }
        
        // Socket doesn't exist, check if service claims to be running
        let user_service = Command::new("systemctl")
            .args(&["--user", "is-active", "ydotool"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        
        let system_service = Command::new("systemctl")
            .args(&["is-active", "ydotool"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        
        if user_service || system_service {
            warn!("ydotool service claims to be active but socket not found at {}", socket_path);
        }
        
        // Socket doesn't exist, service may or may not be running
        // Try to actually use ydotool to verify it works
        // Use a real command that requires the daemon, not --help
        let test_result = Command::new("sh")
            .arg("-c")
            .arg("timeout 1 ydotool key 0 2>&1 | grep -v 'failed to connect socket'")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
            
        if !test_result {
            debug!("ydotool daemon not accessible, marking as unavailable");
        }
        
        test_result
    }
    
    /// Check if we have uinput access
    fn check_uinput_access() -> bool {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;
        
        // Check if /dev/uinput is accessible
        if let Ok(metadata) = fs::metadata("/dev/uinput") {
            let perms = metadata.permissions();
            let mode = perms.mode();
            
            // Check if readable and writable by group or other
            // Group: 0o060, Other: 0o006
            if (mode & 0o060) == 0o060 || (mode & 0o006) == 0o006 {
                return true;
            }
        }
        
        // Check if user is in input group
        Command::new("groups")
            .output()
            .map(|o| {
                String::from_utf8_lossy(&o.stdout).contains("input")
            })
            .unwrap_or(false)
    }
    
    /// Inject text into the focused application
    pub async fn inject(&self, text: &str) -> Result<()> {
        debug!("Injecting text: {} chars", text.len());
        
        // Optional: Ensure focus (if kdotool available)
        if self.use_kdotool {
            self.ensure_focus().await?;
        }
        
        // Try clipboard + paste method first (most reliable).
        // We handle errors here to allow fallback to other methods.
        match self.try_clipboard_paste(text).await {
            Ok(true) => {
                // Successfully pasted, we're done.
                return Ok(());
            }
            Ok(false) => {
                // Paste was not attempted or failed, proceed to fallbacks.
                debug!("Clipboard paste not successful, proceeding to fallbacks.");
            }
            Err(e) => {
                // Setting clipboard itself failed.
                warn!("Clipboard paste method failed with an error: {}. Proceeding to fallbacks.", e);
            }
        };
        
        // Fallback to direct typing if available
        if self.use_ydotool {
            match self.direct_type(text).await {
                Ok(_) => {
                    info!("Successfully injected text via direct typing");
                    return Ok(());
                }
                Err(e) => {
                    warn!("Direct typing failed: {}. Falling back to clipboard-only.", e);
                    // Continue to clipboard-only fallback instead of returning error
                }
            }
        }
        
        // Last resort: clipboard only
        self.set_clipboard(text).await?;
        let message = format!("Text copied to clipboard ({} chars) - press Ctrl+V to paste", text.len());
        self.notify_user(&message).await;
        info!("Text injection completed via clipboard only - user needs to press Ctrl+V");
        Ok(())
    }
    
    /// Try clipboard + paste combo (recommended method)
    async fn try_clipboard_paste(&self, text: &str) -> Result<bool> {
        // Set clipboard
        self.set_clipboard(text).await?;
        
        if !self.use_ydotool {
            warn!("ydotool not available, falling back to clipboard only");
            return Ok(false);
        }
        
        // Small delay for clipboard to settle (best practice from research)
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        // Trigger paste
        match self.trigger_paste().await {
            Ok(_) => {
                info!("Successfully pasted text via clipboard + paste");
                Ok(true)
            },
            Err(e) => {
                error!("Paste failed with ydotool: {}. Falling back to clipboard only.", e);
                Ok(false)
            }
        }
    }
    
    /// Direct typing fallback
    async fn direct_type(&self, text: &str) -> Result<()> {
        let mut cmd = tokio::process::Command::new("ydotool");
        cmd.arg("type").arg("--delay").arg("10").arg(text); // 10ms delay between chars
        
        let output = tokio::time::timeout(Duration::from_secs(5), cmd.output()).await??;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("ydotool type failed: {}", stderr);
        }
        
        debug!("Direct typing successful");
        Ok(())
    }

    /// Set clipboard content
    async fn set_clipboard(&self, text: &str) -> Result<()> {
        let mut cmd = tokio::process::Command::new("wl-copy");
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn()?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(text.as_bytes()).await?;
            drop(stdin); // Close stdin to signal end of input.
        } else {
            anyhow::bail!("Failed to get stdin for wl-copy");
        }

        let output = tokio::time::timeout(Duration::from_secs(3), child.wait_with_output()).await??;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("wl-copy failed to set clipboard: {}", stderr);
        }
        
        debug!("Clipboard set successfully");
        Ok(())
    }
    
    /// Trigger paste action via ydotool
    async fn trigger_paste(&self) -> Result<()> {
        let mut cmd = tokio::process::Command::new("ydotool");
        cmd.args(&["key", "ctrl+v"]);
        
        let output = tokio::time::timeout(Duration::from_secs(3), cmd.output()).await??;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("ydotool key failed: {}", stderr);
        }
        
        debug!("Paste triggered successfully");
        Ok(())
    }
    
    /// Use kdotool to ensure we have focus
    async fn ensure_focus(&self) -> Result<()> {
        let mut cmd = tokio::process::Command::new("kdotool");
        cmd.args(&["getactivewindow"]);
        
        let output = tokio::time::timeout(Duration::from_secs(2), cmd.output()).await??;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to get active window: {}", stderr);
        }
        
        let window_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        debug!("Active window: {}", window_id);
        Ok(())
    }
    
    /// Notify user via desktop notification
    async fn notify_user(&self, message: &str) {
        // Try to notify, but don't fail if notify-send missing  
        match tokio::time::timeout(
            Duration::from_secs(2),
            tokio::process::Command::new("notify-send")
                .args(&["ColdVox", message])
                .output(),
        ).await {
            Ok(Ok(_)) => { /* delivered */ }
            Ok(Err(e)) => debug!("Could not send notification: {}", e),
            Err(e) => debug!("Notification timed out: {}", e),
        }
    }
    
    /// Check if all required tools are properly set up
    pub fn check_setup(&self) -> Vec<String> {
        let mut issues = Vec::new();
        
        // Check wl-clipboard
        if !Command::new("which").arg("wl-copy").output()
            .map(|o| o.status.success()).unwrap_or(false) 
        {
            issues.push("wl-clipboard not installed (required)".to_string());
        }
        
        // Check ydotool
        if !self.use_ydotool {
            issues.push("ydotool not available (recommended)".to_string());
            if !self.has_uinput_access {
                issues.push("No uinput access - add user to 'input' group".to_string());
            }
        }
        
        // Check kdotool
        if !self.use_kdotool {
            issues.push("kdotool not installed (optional, improves focus detection)".to_string());
        }
        
        issues
    }
}

impl Default for TextInjector {
    fn default() -> Self {
        Self::new()
    }
}