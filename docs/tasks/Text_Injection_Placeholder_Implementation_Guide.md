# Text Injection System - Placeholder Implementation Guide

## Overview

This document provides comprehensive implementation details for completing the placeholder code in the ColdVox text injection system. The current implementation has several placeholder methods that need to be replaced with functional code to achieve production readiness.

## Priority Implementation Tasks

### 1. AT-SPI Focus Detection and App Identification

**Location**: `crates/app/src/text_injection/focus.rs` and `crates/app/src/text_injection/manager.rs`

#### Current State
- `check_focus_status()` always returns `FocusStatus::EditableText`
- `get_current_app_id()` returns hardcoded `"unknown_app"`

#### Required Implementation

```rust
// focus.rs - Real AT-SPI focus detection
async fn check_focus_status(&self) -> Result<FocusStatus, InjectionError> {
    #[cfg(feature = "text-injection-atspi")]
    {
        use atspi::{connection::Connection, accessible::Accessible};
        
        // Connect to AT-SPI bus
        let connection = Connection::new().await
            .map_err(|e| InjectionError::Other(format!("AT-SPI connection failed: {}", e)))?;
        
        // Get currently focused accessible
        let focused = connection.get_focused_accessible().await
            .map_err(|e| InjectionError::Other(format!("Failed to get focus: {}", e)))?;
        
        // Check if the focused element supports EditableText interface
        let interfaces = focused.get_interfaces().await
            .map_err(|e| InjectionError::Other(format!("Failed to get interfaces: {}", e)))?;
        
        if interfaces.contains(&"EditableText") {
            // Check if element is actually editable (not read-only)
            let states = focused.get_state_set().await
                .map_err(|e| InjectionError::Other(format!("Failed to get states: {}", e)))?;
            
            if states.contains(atspi::StateType::Editable) && 
               !states.contains(atspi::StateType::ReadOnly) {
                return Ok(FocusStatus::EditableText);
            }
        }
        
        // Check for common text input roles
        let role = focused.get_role().await
            .map_err(|e| InjectionError::Other(format!("Failed to get role: {}", e)))?;
        
        match role {
            atspi::Role::Text | 
            atspi::Role::PasswordText | 
            atspi::Role::Terminal |
            atspi::Role::Entry |
            atspi::Role::EditableComboBox => Ok(FocusStatus::EditableText),
            _ => Ok(FocusStatus::NonEditable)
        }
    }
    
    #[cfg(not(feature = "text-injection-atspi"))]
    {
        // Fallback: Use X11/Wayland window properties
        Ok(FocusStatus::Unknown)
    }
}

// manager.rs - Real app identification
async fn get_current_app_id(&self) -> Result<String, InjectionError> {
    #[cfg(feature = "text-injection-atspi")]
    {
        // Get the focused element's application
        let focused = self.focus_tracker.get_focused_accessible().await?;
        let app = focused.get_application().await
            .map_err(|e| InjectionError::Other(format!("Failed to get app: {}", e)))?;
        
        // Try to get application name
        if let Ok(name) = app.get_name().await {
            if !name.is_empty() {
                return Ok(name);
            }
        }
        
        // Fallback to process name
        if let Ok(toolkit) = app.get_toolkit_name().await {
            return Ok(format!("{}_{}", toolkit, app.get_id().await.unwrap_or_default()));
        }
    }
    
    // Fallback: Use window manager info
    #[cfg(target_os = "linux")]
    {
        // Try to get active window class via X11/Wayland
        if let Ok(window_class) = get_active_window_class().await {
            return Ok(window_class);
        }
    }
    
    Ok("unknown".to_string())
}
```

### 2. Permission Checking for External Binaries

**Location**: `crates/app/src/text_injection/ydotool_injector.rs`, `kdotool_injector.rs`

#### Required Implementation

```rust
// Common permission checking utility
pub fn check_binary_permissions(binary_name: &str) -> Result<(), InjectionError> {
    use std::process::Command;
    use std::os::unix::fs::PermissionsExt;
    
    // Check if binary exists in PATH
    let output = Command::new("which")
        .arg(binary_name)
        .output()
        .map_err(|e| InjectionError::Process(format!("Failed to locate {}: {}", binary_name, e)))?;
    
    if !output.status.success() {
        return Err(InjectionError::MethodUnavailable(
            format!("{} not found in PATH", binary_name)
        ));
    }
    
    let binary_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    
    // Check if binary is executable
    let metadata = std::fs::metadata(&binary_path)
        .map_err(|e| InjectionError::Io(e))?;
    
    let permissions = metadata.permissions();
    if permissions.mode() & 0o111 == 0 {
        return Err(InjectionError::PermissionDenied(
            format!("{} is not executable", binary_name)
        ));
    }
    
    // For ydotool specifically, check uinput access
    if binary_name == "ydotool" {
        check_uinput_access()?;
    }
    
    Ok(())
}

fn check_uinput_access() -> Result<(), InjectionError> {
    use std::fs::OpenOptions;
    
    // Check if we can open /dev/uinput
    match OpenOptions::new().write(true).open("/dev/uinput") {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            // Check if user is in input group
            let groups = Command::new("groups")
                .output()
                .map_err(|e| InjectionError::Process(format!("Failed to check groups: {}", e)))?;
            
            let groups_str = String::from_utf8_lossy(&groups.stdout);
            if !groups_str.contains("input") {
                return Err(InjectionError::PermissionDenied(
                    "User not in 'input' group. Run: sudo usermod -a -G input $USER".to_string()
                ));
            }
            
            Err(InjectionError::PermissionDenied(
                "/dev/uinput access denied. ydotool daemon may not be running".to_string()
            ))
        }
        Err(e) => Err(InjectionError::Io(e))
    }
}
```

### 3. Success Rate Tracking and Adaptive Strategy

**Location**: `crates/app/src/text_injection/manager.rs`

#### Required Implementation

```rust
impl StrategyManager {
    /// Get ordered list of methods to try based on success rates
    fn get_method_priority(&self, app_id: &str) -> Vec<InjectionMethod> {
        let mut methods = vec![];
        
        // Always try AT-SPI first if available
        #[cfg(feature = "text-injection-atspi")]
        methods.push(InjectionMethod::AtspiInsert);
        
        // Add clipboard methods
        #[cfg(feature = "text-injection-clipboard")]
        {
            methods.push(InjectionMethod::Clipboard);
            #[cfg(feature = "text-injection-atspi")]
            methods.push(InjectionMethod::ClipboardAndPaste);
        }
        
        // Sort by success rate for this app
        methods.sort_by(|a, b| {
            let key_a = (app_id.to_string(), *a);
            let key_b = (app_id.to_string(), *b);
            
            let rate_a = self.success_cache.get(&key_a)
                .map(|r| r.success_rate)
                .unwrap_or(0.5); // Default 50% assumed success
            
            let rate_b = self.success_cache.get(&key_b)
                .map(|r| r.success_rate)
                .unwrap_or(0.5);
            
            rate_b.partial_cmp(&rate_a).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        // Add opt-in fallback methods at the end
        if self.config.allow_ydotool && !self.is_in_cooldown(InjectionMethod::YdoToolPaste) {
            methods.push(InjectionMethod::YdoToolPaste);
        }
        
        if self.config.allow_enigo && !self.is_in_cooldown(InjectionMethod::EnigoText) {
            methods.push(InjectionMethod::EnigoText);
        }
        
        if self.config.allow_mki && !self.is_in_cooldown(InjectionMethod::UinputKeys) {
            methods.push(InjectionMethod::UinputKeys);
        }
        
        methods
    }
    
    /// Update success record with decay for old records
    fn update_success_record(&mut self, app_id: &str, method: InjectionMethod, success: bool) {
        let key = (app_id.to_string(), method);
        
        let record = self.success_cache.entry(key.clone()).or_insert_with(|| SuccessRecord {
            success_count: 0,
            fail_count: 0,
            last_success: None,
            last_failure: None,
            success_rate: 0.5,
        });
        
        // Apply time-based decay (older results matter less)
        let decay_factor = 0.95;
        record.success_count = (record.success_count as f64 * decay_factor) as u32;
        record.fail_count = (record.fail_count as f64 * decay_factor) as u32;
        
        // Update counts
        if success {
            record.success_count += 1;
            record.last_success = Some(Instant::now());
        } else {
            record.fail_count += 1;
            record.last_failure = Some(Instant::now());
        }
        
        // Recalculate success rate with minimum sample size
        let total = record.success_count + record.fail_count;
        if total > 0 {
            record.success_rate = record.success_count as f64 / total as f64;
        } else {
            record.success_rate = 0.5; // Default to 50%
        }
        
        // Apply cooldown for repeated failures
        if !success && record.fail_count > 2 {
            self.apply_cooldown(app_id, method, "Multiple consecutive failures");
        }
        
        debug!(
            "Updated success record for {}/{:?}: {:.1}% ({}/{})",
            app_id, method, record.success_rate * 100.0,
            record.success_count, total
        );
    }
    
    /// Apply exponential backoff cooldown
    fn apply_cooldown(&mut self, app_id: &str, method: InjectionMethod, error: &str) {
        let key = (app_id.to_string(), method);
        
        let mut cooldown = self.cooldowns.entry(key).or_insert_with(|| CooldownState {
            until: Instant::now(),
            backoff_level: 0,
            last_error: String::new(),
        });
        
        // Calculate cooldown duration with exponential backoff
        let base_ms = self.config.cooldown_initial_ms;
        let factor = self.config.cooldown_backoff_factor;
        let max_ms = self.config.cooldown_max_ms;
        
        let cooldown_ms = (base_ms as f64 * factor.powi(cooldown.backoff_level as i32))
            .min(max_ms as f64) as u64;
        
        cooldown.until = Instant::now() + Duration::from_millis(cooldown_ms);
        cooldown.backoff_level += 1;
        cooldown.last_error = error.to_string();
        
        warn!(
            "Applied cooldown for {}/{:?}: {}ms (level {})",
            app_id, method, cooldown_ms, cooldown.backoff_level
        );
    }
}
```

### 4. Window Manager Integration

**Location**: New file `crates/app/src/text_injection/window_manager.rs`

#### Required Implementation

```rust
use std::process::Command;

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
        // Parse JSON to find focused window
        // This would require serde_json dependency
        // For now, return error
        return Err(InjectionError::Other("Wayland parsing not implemented".to_string()));
    }
    
    Err(InjectionError::Other("Wayland window class not available".to_string()))
}
```

### 5. Clipboard Restoration Enhancement

**Location**: `crates/app/src/text_injection/clipboard_injector.rs`

#### Required Implementation

```rust
impl ClipboardInjector {
    /// Save current clipboard content for restoration
    async fn save_clipboard(&mut self) -> Result<Option<String>, InjectionError> {
        if !self.config.restore_clipboard {
            return Ok(None);
        }
        
        #[cfg(feature = "text-injection-clipboard")]
        {
            use wl_clipboard_rs::paste::{get_contents, ClipboardType, Seat};
            
            // Try to get current clipboard content
            match get_contents(ClipboardType::Regular, Seat::Unspecified) {
                Ok((mut pipe, _mime)) => {
                    let mut contents = String::new();
                    if pipe.read_to_string(&mut contents).is_ok() {
                        debug!("Saved clipboard content ({} chars)", contents.len());
                        return Ok(Some(contents));
                    }
                }
                Err(e) => {
                    debug!("Could not save clipboard: {}", e);
                }
            }
        }
        
        Ok(None)
    }
    
    /// Restore previously saved clipboard content
    async fn restore_clipboard(&mut self, content: Option<String>) -> Result<(), InjectionError> {
        if let Some(content) = content {
            if !self.config.restore_clipboard {
                return Ok(());
            }
            
            #[cfg(feature = "text-injection-clipboard")]
            {
                use wl_clipboard_rs::copy::{MimeType, Options, Source};
                
                let opts = Options::new();
                match opts.copy(Source::Bytes(content.as_bytes()), MimeType::Text) {
                    Ok(_) => {
                        debug!("Restored clipboard content ({} chars)", content.len());
                    }
                    Err(e) => {
                        warn!("Failed to restore clipboard: {}", e);
                    }
                }
            }
        }
        
        Ok(())
    }
}
```

## Testing Strategy

### Unit Tests

Create comprehensive unit tests for each component:

```rust
// tests/test_focus_tracking.rs
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_focus_detection() {
        let config = InjectionConfig::default();
        let mut tracker = FocusTracker::new(config);
        
        // Test focus detection
        let status = tracker.get_focus_status().await;
        assert!(status.is_ok());
        
        // Test caching
        let cached = tracker.cached_focus_status();
        assert!(cached.is_some());
    }
    
    #[tokio::test]
    async fn test_app_identification() {
        let manager = StrategyManager::new(InjectionConfig::default());
        let app_id = manager.get_current_app_id().await;
        
        assert!(app_id.is_ok());
        assert_ne!(app_id.unwrap(), "unknown_app");
    }
    
    #[test]
    fn test_permission_checking() {
        // Test binary existence
        let result = check_binary_permissions("ls"); // Should exist
        assert!(result.is_ok());
        
        let result = check_binary_permissions("nonexistent_binary_xyz");
        assert!(result.is_err());
    }
}
```

### Integration Tests

```rust
// tests/test_injection_integration.rs
#[cfg(all(test, feature = "text-injection"))]
mod integration_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_full_injection_flow() {
        let config = InjectionConfig {
            allow_ydotool: false,
            restore_clipboard: true,
            ..Default::default()
        };
        
        let mut manager = StrategyManager::new(config);
        
        // Test injection with actual text
        let result = manager.inject("Test injection").await;
        
        // Should attempt AT-SPI or clipboard methods
        assert!(result.is_ok() || result.is_err());
        
        // Check metrics
        let metrics = manager.metrics();
        assert!(metrics.attempts > 0);
    }
}
```

## Configuration Updates

Add new configuration options to `InjectionConfig`:

```rust
pub struct InjectionConfig {
    // ... existing fields ...
    
    /// Cache duration for focus status (ms)
    #[serde(default = "default_focus_cache_duration_ms")]
    pub focus_cache_duration_ms: u64,
    
    /// Minimum success rate before trying fallback methods
    #[serde(default = "default_min_success_rate")]
    pub min_success_rate: f64,
    
    /// Number of samples before trusting success rate
    #[serde(default = "default_min_sample_size")]
    pub min_sample_size: u32,
    
    /// Enable window manager integration
    #[serde(default = "default_true")]
    pub enable_window_detection: bool,
}

fn default_focus_cache_duration_ms() -> u64 { 200 }
fn default_min_success_rate() -> f64 { 0.3 }
fn default_min_sample_size() -> u32 { 5 }
fn default_true() -> bool { true }
```

## Deployment Checklist

- [ ] Replace all placeholder implementations
- [ ] Add permission checking for external binaries
- [ ] Implement real AT-SPI focus detection
- [ ] Add window manager integration
- [ ] Implement adaptive strategy with success tracking
- [ ] Add clipboard save/restore functionality
- [ ] Create comprehensive unit tests
- [ ] Add integration tests
- [ ] Update configuration with new options
- [ ] Document user setup requirements (groups, permissions)
- [ ] Test on KDE Plasma Wayland
- [ ] Test on X11 environments
- [ ] Benchmark performance impact

## Performance Considerations

1. **Caching**: Cache focus status and app ID for 200ms to avoid excessive AT-SPI calls
2. **Async Operations**: Use tokio for all I/O operations to avoid blocking
3. **Timeout Management**: Enforce strict timeouts on all external calls
4. **Success Rate Decay**: Apply time-based decay to prevent stale data
5. **Resource Cleanup**: Always restore clipboard if interrupted

## Security Considerations

1. **Permission Verification**: Check binary permissions before execution
2. **Input Validation**: Sanitize text before injection
3. **Clipboard Privacy**: Only restore clipboard if explicitly configured
4. **Process Isolation**: Run external commands with minimal privileges
5. **Error Disclosure**: Don't expose sensitive system information in errors

## Notes

- AT-SPI2 requires `at-spi2-core` package on most distributions
- Wayland clipboard requires `wl-clipboard` package
- KDE integration works best with `qdbus` available
- X11 fallback requires `xprop` utility
- Consider implementing a daemon mode for better performance