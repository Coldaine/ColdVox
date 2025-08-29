use crate::text_injection::types::{InjectionConfig, InjectionError, InjectionMethod, InjectionMetrics};
use enigo::{Enigo, KeyboardControllable, Key};
use std::time::Duration;
use tokio::time::{timeout, error::Elapsed};
use tracing::{debug, error, info, warn};

/// Trait for all text injection methods
pub trait TextInjector {
    /// Name of the injector for logging and metrics
    fn name(&self) -> &'static str;
    
    /// Check if this injector is available for use
    fn is_available(&self) -> bool;
    
    /// Inject text using this method
    fn inject(&mut self, text: &str) -> Result<(), InjectionError>;
    
    /// Get metrics for this injector
    fn metrics(&self) -> &InjectionMetrics;
}

/// Enigo injector for synthetic input
pub struct EnigoInjector {
    config: InjectionConfig,
    metrics: InjectionMetrics,
    /// Whether enigo is available and can be used
    is_available: bool,
}

impl EnigoInjector {
    /// Create a new enigo injector
    pub fn new(config: InjectionConfig) -> Self {
        let is_available = Self::check_availability();
        
        Self {
            config,
            metrics: InjectionMetrics::default(),
            is_available,
        }
    }

    /// Check if enigo can be used (permissions, backend availability)
    fn check_availability() -> bool {
        // Check if we can create an Enigo instance
        // This will fail if we don't have the necessary permissions
        Enigo::new().is_ok()
    }

    /// Type text using enigo
    fn type_text(&mut self, text: &str) -> Result<(), InjectionError> {
        let start = std::time::Instant::now();
        
        let mut enigo = Enigo::new();
        
        // Type each character with a small delay
        for c in text.chars() {
            match c {
                ' ' => enigo.key_click(Key::Space),
                '\n' => enigo.key_click(Key::Return),
                '\t' => enigo.key_click(Key::Tab),
                _ => {
                    if c.is_ascii() {
                        enigo.key_sequence(&c.to_string());
                    } else {
                        // For non-ASCII characters, we might need to use clipboard
                        return Err(InjectionError::MethodFailed("Enigo doesn't support non-ASCII characters directly".to_string()));
                    }
                }
            }
        }
        
        let duration = start.elapsed().as_millis() as u64;
        self.metrics.record_success(InjectionMethod::EnigoText, duration);
        info!("Successfully typed text via enigo ({} chars)", text.len());
        
        Ok(())
    }

    /// Trigger paste action using enigo (Ctrl+V)
    fn trigger_paste(&mut self) -> Result<(), InjectionError> {
        let start = std::time::Instant::now();
        
        let mut enigo = Enigo::new();
        
        // Press Ctrl+V
        enigo.key_down(Key::Control);
        enigo.key_click(Key::Layout('v'));
        enigo.key_up(Key::Control);
        
        let duration = start.elapsed().as_millis() as u64;
        self.metrics.record_success(InjectionMethod::EnigoText, duration);
        info!("Successfully triggered paste action via enigo");
        
        Ok(())
    }
}

impl TextInjector for EnigoInjector {
    fn name(&self) -> &'static str {
        "Enigo"
    }

    fn is_available(&self) -> bool {
        self.is_available && self.config.allow_enigo
    }

    fn inject(&mut self, text: &str) -> Result<(), InjectionError> {
        if text.is_empty() {
            return Ok(());
        }

        // First try paste action (more reliable for batch text)
        // We need to set the clipboard first, but that's handled by the strategy manager
        // So we just trigger the paste
        match self.trigger_paste() {
            Ok(()) => Ok(()),
            Err(e) => {
                debug!("Paste action failed: {}", e);
                // Fall back to direct typing
                self.type_text(text)
            }
        }
    }

    fn metrics(&self) -> &InjectionMetrics {
        &self.metrics
    }
}