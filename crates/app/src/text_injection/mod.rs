pub mod common;
pub mod manager;

pub use common::{InjectionError, TextInjector};
pub use manager::{InjectionConfig, StrategyManager};

#[cfg(feature = "text-injection-atspi")]
mod atspi_injector;
#[cfg(feature = "text-injection-atspi")]
pub use atspi_injector::AtspiInjector;

#[cfg(feature = "text-injection-clipboard")]
mod clipboard_injector;
#[cfg(feature = "text-injection-clipboard")]
pub use clipboard_injector::ClipboardInjector;

#[cfg(feature = "text-injection-portal-eis")]
mod portal_eis_injector;
#[cfg(feature = "text-injection-portal-eis")]
pub use portal_eis_injector::PortalEisInjector;

#[cfg(feature = "text-injection-vkm")]
mod vkm_injector;
#[cfg(feature = "text-injection-vkm")]
pub use vkm_injector::VkmInjector;

#[cfg(feature = "text-injection-clipboard-x11")]
mod x11_clipboard_injector;
#[cfg(feature = "text-injection-clipboard-x11")]
pub use x11_clipboard_injector::X11ClipboardInjector;

#[cfg(feature = "text-injection-ydotool")]
mod ydotool_injector;
#[cfg(feature = "text-injection-ydotool")]
pub use ydotool_injector::YdotoolInjector;
