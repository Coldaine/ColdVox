use super::{InjectionError, TextInjector};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InjectionConfig {
    pub silence_timeout_ms: u64,
    pub inject_on_unknown_focus: bool,
    pub allow_ydotool: bool,
    pub restore_clipboard: bool,
    pub max_total_latency_ms: u64,
}

pub struct StrategyManager {
    config: InjectionConfig,
    injectors: Vec<Arc<dyn TextInjector>>,
}

impl StrategyManager {
    pub fn new(config: InjectionConfig) -> Self {
        Self {
            config,
            injectors: Vec::new(),
        }
    }

    pub fn with_default_order(config: InjectionConfig) -> Self {
        let mut mgr = Self::new(config.clone());

        // 1) AT-SPI direct
        #[cfg(feature = "text-injection-atspi")]
        {
            let inj = crate::text_injection::AtspiInjector::new();
            if inj.is_available() {
                mgr.push(std::sync::Arc::new(inj));
            }
        }

        // 2) Wayland clipboard (optionally followed by AT-SPI paste later)
        #[cfg(feature = "text-injection-clipboard")]
        {
            let inj = crate::text_injection::ClipboardInjector::new();
            if inj.is_available() {
                mgr.push(std::sync::Arc::new(inj));
            }
        }

        // 3) XDG Portal / EIS (permissioned typed input)
        #[cfg(feature = "text-injection-portal-eis")]
        {
            let inj = crate::text_injection::PortalEisInjector::new();
            if inj.is_available() {
                mgr.push(std::sync::Arc::new(inj));
            }
        }

        // 4) Wayland virtual keyboard (compositor-dependent)
        #[cfg(feature = "text-injection-vkm")]
        {
            let inj = crate::text_injection::VkmInjector::new();
            if inj.is_available() {
                mgr.push(std::sync::Arc::new(inj));
            }
        }

        // 5) X11/XWayland clipboard fallback
        #[cfg(feature = "text-injection-clipboard-x11")]
        {
            let inj = crate::text_injection::X11ClipboardInjector::new();
            if inj.is_available() {
                mgr.push(std::sync::Arc::new(inj));
            }
        }

        // 6) ydotool last (explicit opt-in)
        #[cfg(feature = "text-injection-ydotool")]
        {
            if config.allow_ydotool {
                let inj = crate::text_injection::YdotoolInjector::new();
                if inj.is_available() {
                    mgr.push(std::sync::Arc::new(inj));
                }
            }
        }

        mgr
    }

    pub fn push(&mut self, injector: Arc<dyn TextInjector>) {
        info!("Registered text injector: {}", injector.name());
        self.injectors.push(injector);
    }

    pub fn try_inject(&self, text: &str) -> Result<()> {
        if self.injectors.is_empty() {
            warn!("No text injectors available or enabled.");
            return Err(anyhow::anyhow!(InjectionError::MethodNotAvailable(
                "No injectors registered".to_string()
            )));
        }

        for injector in &self.injectors {
            debug!("Attempting injection with {}", injector.name());
            match injector.inject(text) {
                Ok(()) => {
                    info!("Successfully injected text with {}", injector.name());
                    return Ok(());
                }
                Err(e) => {
                    warn!(
                        "Injector {} failed: {}. Trying next method.",
                        injector.name(),
                        e
                    );
                }
            }
        }

        Err(anyhow::anyhow!(InjectionError::InjectionFailed(
            "All injection methods failed".to_string()
        )))
    }
}
