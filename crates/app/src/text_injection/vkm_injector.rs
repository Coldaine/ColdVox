#![cfg(feature = "text-injection-vkm")]

use crate::text_injection::{InjectionError, TextInjector};
use anyhow::{anyhow, Result};
use std::env;

/// Demo of the Wayland virtual keyboard path.
/// We intentionally DO NOT pull wayland-client/protocols yet.
/// Next step:
/// - Add deps: wayland-client, wayland-protocols (and optionally smithay-client-toolkit)
/// - Probe registry for `zwp_virtual_keyboard_manager_v1`
/// - Establish a vkbd seat and send key events
#[derive(Debug, Default)]
pub struct VkmInjector;

impl VkmInjector {
    pub fn new() -> Self { Self }

    fn compositor_may_support_vkm() -> bool {
        // Minimal heuristic; real check will query globals via wayland-client.
        env::var_os("WAYLAND_DISPLAY").is_some()
    }
}

impl TextInjector for VkmInjector {
    fn name(&self) -> &'static str { "Wayland-VKM" }

    fn is_available(&self) -> bool { Self::compositor_may_support_vkm() }

    fn inject(&self, _text: &str) -> Result<()> {
        Err(anyhow!(InjectionError::MethodNotAvailable(
            "VKM typing not implemented yet (will use wayland-client & virtual_keyboard_unstable_v1)".into()
        )))
    }
}
