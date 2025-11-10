use crate::types::InjectionConfig;
use async_trait::async_trait;
use coldvox_foundation::error::InjectionError;
use std::time::{Duration, Instant};
use tracing::debug;

#[async_trait]
pub trait FocusProvider: Send + Sync {
    async fn get_focus_status(&mut self) -> Result<FocusStatus, InjectionError>;
}

#[async_trait]
pub trait FocusBackend: Send + Sync {
    async fn query_focus(&self) -> Result<FocusStatus, InjectionError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusStatus {
    EditableText,
    NonEditable,
    Unknown,
}

pub struct FocusTracker<B: FocusBackend = SystemFocusAdapter> {
    config: InjectionConfig,
    backend: B,
    last_check: Option<Instant>,
    cached_status: Option<FocusStatus>,
    cache_duration: Duration,
}

impl FocusTracker<SystemFocusAdapter> {
    pub fn new(config: InjectionConfig) -> Self {
        Self::with_backend(config, SystemFocusAdapter)
    }
}

impl<B: FocusBackend> FocusTracker<B> {
    pub fn with_backend(config: InjectionConfig, backend: B) -> Self {
        let cache_duration = Duration::from_millis(config.focus_cache_duration_ms);
        Self {
            config,
            backend,
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

    async fn check_focus_status(&self) -> Result<FocusStatus, InjectionError> {
        match self.backend.query_focus().await {
            Ok(status) => Ok(status),
            Err(err) => {
                debug!("Focus backend error: {}", err);
                Ok(FocusStatus::Unknown)
            }
        }
    }

    pub fn config(&self) -> &InjectionConfig {
        &self.config
    }
}

#[async_trait]
impl<B: FocusBackend> FocusProvider for FocusTracker<B> {
    async fn get_focus_status(&mut self) -> Result<FocusStatus, InjectionError> {
        FocusTracker::get_focus_status(self).await
    }
}

#[derive(Default, Clone)]
pub struct SystemFocusAdapter;

#[cfg(not(feature = "atspi"))]
#[async_trait]
impl FocusBackend for SystemFocusAdapter {
    async fn query_focus(&self) -> Result<FocusStatus, InjectionError> {
        // Stub implementation when AT-SPI is not enabled.
        Ok(FocusStatus::Unknown)
    }
}

#[cfg(feature = "atspi")]
#[async_trait]
impl FocusBackend for SystemFocusAdapter {
    async fn query_focus(&self) -> Result<FocusStatus, InjectionError> {
        use crate::log_throttle::log_atspi_connection_failure;
        use atspi::{
            connection::AccessibilityConnection, proxy::accessible::AccessibleProxy,
            proxy::collection::CollectionProxy, Interface, MatchType, ObjectMatchRule, SortOrder,
            State,
        };

        let conn = match AccessibilityConnection::new().await {
            Ok(c) => c,
            Err(e) => {
                log_atspi_connection_failure(&e.to_string());
                return Ok(FocusStatus::Unknown);
            }
        };

        let zbus_conn = conn.connection();

        let collection = match CollectionProxy::builder(zbus_conn)
            .destination("org.a11y.atspi.Registry")
            .map_err(|e| InjectionError::Other(format!("Collection destination failed: {e}")))?
            .path("/org/a11y/atspi/accessible/root")
            .map_err(|e| InjectionError::Other(format!("Collection path failed: {e}")))?
            .build()
            .await
        {
            Ok(p) => p,
            Err(e) => {
                debug!("Failed to build CollectionProxy: {}", e);
                return Ok(FocusStatus::Unknown);
            }
        };

        let mut rule = ObjectMatchRule::default();
        rule.states = State::Focused.into();
        rule.states_mt = MatchType::All;

        let matches = match collection
            .get_matches(rule, SortOrder::Canonical, 1, false)
            .await
        {
            Ok(m) => m,
            Err(e) => {
                debug!("Failed to get matches from CollectionProxy: {}", e);
                return Ok(FocusStatus::Unknown);
            }
        };

        if let Some(obj_ref) = matches.first() {
            let accessible = match AccessibleProxy::builder(zbus_conn)
                .destination(obj_ref.name.clone())
                .map_err(|e| {
                    InjectionError::Other(format!("AccessibleProxy destination failed: {e}"))
                })?
                .path(obj_ref.path.clone())
                .map_err(|e| InjectionError::Other(format!("AccessibleProxy path failed: {e}")))?
                .build()
                .await
            {
                Ok(p) => p,
                Err(e) => {
                    debug!("Failed to build AccessibleProxy: {}", e);
                    return Ok(FocusStatus::Unknown);
                }
            };

            let ifaces = match accessible.get_interfaces().await {
                Ok(i) => i,
                Err(e) => {
                    debug!("Failed to get interfaces: {}", e);
                    return Ok(FocusStatus::Unknown);
                }
            };

            if ifaces.contains(Interface::EditableText) {
                debug!("Focused element is editable: {:?}", obj_ref.name);
                Ok(FocusStatus::EditableText)
            } else {
                debug!("Focused element is not editable: {:?}", obj_ref.name);
                Ok(FocusStatus::NonEditable)
            }
        } else {
            debug!("No focused element found");
            Ok(FocusStatus::Unknown)
        }
    }
}

#[async_trait]
impl<T> FocusBackend for std::sync::Arc<T>
where
    T: FocusBackend + ?Sized,
{
    async fn query_focus(&self) -> Result<FocusStatus, InjectionError> {
        (**self).query_focus().await
    }
}
