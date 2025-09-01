use crate::types::InjectionConfig;
use std::env;

/// Available text injection backends
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// Wayland with virtual keyboard (wlroots/wlr-virtual-keyboard)
    WaylandVirtualKeyboard,
    /// Wayland with xdg-desktop-portal's RemoteDesktop/VirtualKeyboard
    WaylandXdgDesktopPortal,
    /// X11 with xdotool/xtest
    X11Xdotool,
    /// X11 with native Rust wrapper
    X11Native,
    /// macOS with CGEvent/AX API
    MacCgEvent,
    /// macOS with NSPasteboard
    MacPasteboard,
    /// Windows with SendInput
    WindowsSendInput,
    /// Windows with clipboard
    WindowsClipboard,
}

/// Backend capability detector
pub struct BackendDetector {
    _config: InjectionConfig,
}

impl BackendDetector {
    /// Create a new backend detector
    pub fn new(config: InjectionConfig) -> Self {
        Self { _config: config }
    }

    /// Detect available backends on the current system
    pub fn detect_available_backends(&self) -> Vec<Backend> {
        let mut available = Vec::new();
        
        // Detect Wayland backends
        if self.is_wayland() {
            // Check for xdg-desktop-portal VirtualKeyboard
            if self.has_xdg_desktop_portal_virtual_keyboard() {
                available.push(Backend::WaylandXdgDesktopPortal);
            }
            
            // Check for wlr-virtual-keyboard (requires compositor support)
            if self.has_wlr_virtual_keyboard() {
                available.push(Backend::WaylandVirtualKeyboard);
            }
        }
        
        // Detect X11 backends
        if self.is_x11() {
            // Check for xdotool
            if self.has_xdotool() {
                available.push(Backend::X11Xdotool);
            }
            
            // Native X11 wrapper is always available if on X11
            available.push(Backend::X11Native);
        }
        
        // Detect macOS backends
        if self.is_macos() {
            available.push(Backend::MacCgEvent);
            available.push(Backend::MacPasteboard);
        }
        
        // Detect Windows backends
        if self.is_windows() {
            available.push(Backend::WindowsSendInput);
            available.push(Backend::WindowsClipboard);
        }
        
        available
    }
    
    /// Get the preferred backend based on availability and configuration
    pub fn get_preferred_backend(&self) -> Option<Backend> {
        let available = self.detect_available_backends();
        
        // Return the most preferred available backend
        Self::preferred_order().into_iter().find(|&preferred| available.contains(&preferred))
    }
    
    /// Get the preferred order of backends
    fn preferred_order() -> Vec<Backend> {
        vec![
            Backend::WaylandXdgDesktopPortal,      // Preferred on Wayland
            Backend::WaylandVirtualKeyboard,       // Fallback on Wayland
            Backend::X11Xdotool,                   // Preferred on X11
            Backend::X11Native,                    // Fallback on X11
            Backend::MacCgEvent,                   // Preferred on macOS
            Backend::MacPasteboard,                // Fallback on macOS
            Backend::WindowsSendInput,             // Preferred on Windows
            Backend::WindowsClipboard,             // Fallback on Windows
        ]
    }
    
    /// Check if running on Wayland
    fn is_wayland(&self) -> bool {
        env::var("XDG_SESSION_TYPE")
            .map(|s| s == "wayland")
            .unwrap_or(false)
            || env::var("WAYLAND_DISPLAY").is_ok()
    }
    
    /// Check if running on X11
    fn is_x11(&self) -> bool {
        env::var("XDG_SESSION_TYPE")
            .map(|s| s == "x11")
            .unwrap_or(false)
            || env::var("DISPLAY").is_ok()
    }
    
    /// Check if running on macOS
    fn is_macos(&self) -> bool {
        cfg!(target_os = "macos")
    }
    
    /// Check if running on Windows
    fn is_windows(&self) -> bool {
        cfg!(target_os = "windows")
    }
    
    /// Check if xdg-desktop-portal VirtualKeyboard is available
    fn has_xdg_desktop_portal_virtual_keyboard(&self) -> bool {
        // Check if xdg-desktop-portal is running and supports VirtualKeyboard
        // This would typically involve D-Bus communication
        // For now, we'll check if the portal is available
        std::process::Command::new("pgrep")
            .arg("xdg-desktop-portal")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
    
    /// Check if wlr-virtual-keyboard is available
    fn has_wlr_virtual_keyboard(&self) -> bool {
        // This would require checking if the compositor supports wlr-virtual-keyboard
        // For now, we'll check if the binary is available
        std::process::Command::new("which")
            .arg("wlr-virtual-keyboard")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
    
    /// Check if xdotool is available
    fn has_xdotool(&self) -> bool {
        std::process::Command::new("which")
            .arg("xdotool")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_backend_detection() {
        let config = InjectionConfig::default();
        let detector = BackendDetector::new(config);
        
        let backends = detector.detect_available_backends();
        
        // At least one backend should be available
        assert!(!backends.is_empty());
        
        // Check that the preferred backend is in the list
        if let Some(preferred) = detector.get_preferred_backend() {
            assert!(backends.contains(&preferred));
        }
    }
    
    #[test]
    fn test_preferred_order() {
        let order = BackendDetector::preferred_order();
        
        // Check that Wayland backends are preferred first
        assert_eq!(order[0], Backend::WaylandXdgDesktopPortal);
        assert_eq!(order[1], Backend::WaylandVirtualKeyboard);
        
        // Check that X11 backends come next
        assert_eq!(order[2], Backend::X11Xdotool);
        assert_eq!(order[3], Backend::X11Native);
    }
}