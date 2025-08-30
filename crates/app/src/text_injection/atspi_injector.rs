#![cfg(feature = "text-injection-atspi")]

use crate::text_injection::{InjectionError, TextInjector};
use anyhow::{anyhow, Result};
use std::env;
// Import kept for later; ok to be unused for now if you allow dead_code.
// If you want zero warnings, gate it behind a cfg(test) or comment it out until needed.
use atspi::connection::AccessibilityConnection; // <-- correct import

pub struct AtspiInjector;

impl AtspiInjector {
    pub fn new() -> Self { Self }

    fn env_bus_present() -> bool {
        env::var_os("AT_SPI_BUS_ADDRESS").is_some()
            || env::var_os("DBUS_SESSION_BUS_ADDRESS").is_some()
    }
}

impl TextInjector for AtspiInjector {
    fn name(&self) -> &'static str { "AT-SPI" }

    fn is_available(&self) -> bool {
        // Cheap probe only; no async work yet.
        Self::env_bus_present()
    }

    fn inject(&self, _text: &str) -> Result<()> {
        // Stub for now — don’t open a connection yet to avoid async in this phase.
        Err(anyhow!(InjectionError::MethodNotAvailable(
            "AT-SPI direct injection not implemented yet".into()
        )))
    }
}
