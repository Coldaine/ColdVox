use crate::types::{InjectionConfig, InjectionError};
use std::time::{Duration, Instant};
use tracing::debug;

#[async_trait::async_trait]
pub trait FocusProvider: Send + Sync {
    async fn get_focus_status(&mut self) -> Result<FocusStatus, InjectionError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusStatus {
    EditableText,
    NonEditable,
    Unknown,
}

pub struct FocusTracker {
    _config: InjectionConfig,
    last_check: Option<Instant>,
    cached_status: Option<FocusStatus>,
    cache_duration: Duration,
}

impl FocusTracker {
    pub fn new(config: InjectionConfig) -> Self {
        let cache_duration = Duration::from_millis(config.focus_cache_duration_ms);
        Self {
            _config: config,
            last_check: None,
            cached_status: None,
            cache_duration,
        }
    }

    pub async fn get_focus_status(&mut self) -> Result<FocusStatus, InjectionError> {
        if let (Some(last_check), Some(status)) = (self.last_check, self.cached_status) {
            if last_check.elapsed() < self.cache_duration {
                debug!("Using cached focus status: {:?}", status);
                return Ok(status);
            }
        }
        let status = self.check_focus_status().await?;
        self.last_check = Some(Instant::now());
        self.cached_status = Some(status);
        debug!("Focus status determined: {:?}", status);
        Ok(status)
    }

    #[allow(clippy::unused_async)] // Keep async because cfg(feature = "atspi") block contains await calls
    async fn check_focus_status(&self) -> Result<FocusStatus, InjectionError> {
        #[cfg(feature = "atspi")]
        {
            use atspi::{
                connection::AccessibilityConnection,
                object_ref::ObjectRef,
                proxy::accessible::AccessibleProxy,
                State, AtspiError,
            };
            use std::ops::Deref;

            let conn = AccessibilityConnection::new().await?;

            let registry_proxy = conn.deref();
            let zbus_proxy = registry_proxy.inner();

            let msg = zbus_proxy.call_method("GetFocus", &())
                .await
                .map_err(AtspiError::from)?;

            let (focused,): (ObjectRef,) = msg.body().deserialize()
                .map_err(AtspiError::from)?;

            if focused.path.as_str() == "/org/a11y/atspi/null" {
                return Ok(FocusStatus::NonEditable);
            }

            let proxy = AccessibleProxy::builder(conn.connection())
                .destination(focused.name.clone())
                .map_err(AtspiError::from)?
                .path(focused.path.clone())
                .map_err(AtspiError::from)?
                .build()
                .await
                .map_err(AtspiError::from)?;

            let states = proxy.get_state().await.map_err(AtspiError::from)?;
            if states.contains(State::Editable) {
                Ok(FocusStatus::EditableText)
            } else {
                Ok(FocusStatus::NonEditable)
            }
        }
        #[cfg(not(feature = "atspi"))]
        {
            // Fallback for when AT-SPI is not enabled
            Ok(FocusStatus::Unknown)
        }
    }
}

#[async_trait::async_trait]
impl FocusProvider for FocusTracker {
    async fn get_focus_status(&mut self) -> Result<FocusStatus, InjectionError> {
        FocusTracker::get_focus_status(self).await
    }
}
