#![cfg(feature = "text-injection-portal-eis")]

use crate::text_injection::{InjectionError, TextInjector};
use anyhow::{anyhow, Result};
use std::env;

/// Demo of the XDG Portal + EIS route.
/// This injector intentionally avoids pulling `ashpd` right now to keep deps slim.
/// The code paths below show EXACTLY where `ashpd` async calls will go.
///
/// TODO(phase-up):
/// - Add dependency: ashpd = { version = "x.y", optional = true }
/// - Create a small Tokio runtime inside `inject()`
/// - Flow:
///   1) RemoteDesktop::create_session()
///   2) select_devices(DeviceType::Keyboard)
///   3) start() => triggers user consent
///   4a) Simple: notify_keyboard_keycode(...) for Ctrl+V to paste
///   4b) Advanced: connect_to_eis() -> libei client to type arbitrary text
#[derive(Debug, Default)]
pub struct PortalEisInjector;

impl PortalEisInjector {
    pub fn new() -> Self {
        Self
    }

    fn on_desktop_with_portal() -> bool {
        // Heuristics: Wayland or X11 + DBus session.
        (env::var_os("WAYLAND_DISPLAY").is_some() || env::var_os("DISPLAY").is_some())
            && env::var_os("DBUS_SESSION_BUS_ADDRESS").is_some()
    }
}

impl TextInjector for PortalEisInjector {
    fn name(&self) -> &'static str { "XDG-Portal-EIS" }

    fn is_available(&self) -> bool { Self::on_desktop_with_portal() }

    fn inject(&self, _text: &str) -> Result<()> {
        // This is deliberately a no-op stub with clear instructions.
        Err(anyhow!(InjectionError::MethodNotAvailable(
            "Portal/EIS path not implemented yet (will use `ashpd` RemoteDesktop)".into()
        )))
    }
}
