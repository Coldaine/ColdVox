//! Service registry scaffolding (Phase 1).
//! Provides a factory for GUI service instances. Minimal placeholder for now.

use crate::service::{GuiService, ServiceError};

#[cfg(feature = "backend-integration")]
use crate::service::{GuiConfig, ServiceState};

#[cfg(feature = "backend-integration")]
pub struct ServiceRegistry {}

#[cfg(feature = "backend-integration")]
impl ServiceRegistry {
    pub async fn new() -> Result<Self, ServiceError> {
        // In later phases, initialize backend services here.
        Ok(Self {})
    }

    pub fn create_gui_service(&self) -> impl GuiService {
        super::gui_service_impl::GuiServiceImpl::default()
    }
}

// When backend-integration is disabled, expose a no-op registry API to keep builds working.
#[cfg(not(feature = "backend-integration"))]
pub struct ServiceRegistry;

#[cfg(not(feature = "backend-integration"))]
impl ServiceRegistry {
    pub fn new_blocking() -> Self { ServiceRegistry }
}
