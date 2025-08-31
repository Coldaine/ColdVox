use std::process::Command;
use tracing::{debug, warn};

/// A collection of checks to determine which injection methods are available.
#[derive(Debug, Clone, Default)]
pub struct CapabilityReport {
    pub is_wayland: bool,
    pub is_atspi_available: bool,
    pub is_wl_clipboard_available: bool,
    pub is_ydotool_available: bool,
    pub is_kdotool_available: bool,
    pub has_uinput_access: bool,
}

impl CapabilityReport {
    /// Run all probes and generate a report.
    pub fn new() -> Self {
        Self {
            is_wayland: is_wayland(),
            is_atspi_available: is_atspi_available(),
            is_wl_clipboard_available: is_wl_clipboard_available(),
            is_ydotool_available: is_ydotool_available(),
            is_kdotool_available: is_kdotool_available(),
            has_uinput_access: has_uinput_access(),
        }
    }

    pub fn log(&self) {
        debug!("Capability Report:");
        debug!("  Wayland session: {}", self.is_wayland);
        debug!("  AT-SPI bus: {}", self.is_atspi_available);
        debug!("  wl-clipboard: {}", self.is_wl_clipboard_available);
        debug!("  ydotool: {}", self.is_ydotool_available);
        debug!("  kdotool: {}", self.is_kdotool_available);
        debug!("  uinput access: {}", self.has_uinput_access);
    }
}

/// Check if running in a Wayland session.
pub fn is_wayland() -> bool {
    std::env::var("WAYLAND_DISPLAY").is_ok()
}

/// Check if the AT-SPI bus is available.
/// This is a basic check; a full check would involve trying to connect.
pub fn is_atspi_available() -> bool {
    // A proper check would be to try to connect to the bus.
    // For now, we'll check for the accessibility environment variable.
    // This is not foolproof, but it's a good hint.
    let atspi_bus_addr = std::env::var("AT_SPI_BUS_ADDRESS");
    if atspi_bus_addr.is_err() {
        warn!("AT_SPI_BUS_ADDRESS not set, assuming accessibility is disabled.");
        return false;
    }
    true
}

/// Check if `wl-copy` binary is in the PATH.
pub fn is_wl_clipboard_available() -> bool {
    Command::new("which")
        .arg("wl-copy")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check for `ydotool` binary and daemon socket.
pub fn is_ydotool_available() -> bool {
    let binary_exists = Command::new("which")
        .arg("ydotool")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !binary_exists {
        return false;
    }

    // Check for the socket
    let user_id = std::env::var("UID").unwrap_or_else(|_| "1000".to_string());
    let socket_path = format!("/run/user/{}/.ydotool_socket", user_id);
    std::path::Path::new(&socket_path).exists()
}

/// Check for `kdotool` binary.
pub fn is_kdotool_available() -> bool {
    Command::new("which")
        .arg("kdotool")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check for write access to `/dev/uinput`.
pub fn has_uinput_access() -> bool {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    if let Ok(metadata) = fs::metadata("/dev/uinput") {
        let perms = metadata.permissions();
        // Check if the file is writable by the current user.
        // This is a simplified check. A more robust check would involve checking group membership.
        return perms.mode() & 0o002 != 0; // Writable by "other"
    }
    false
}
