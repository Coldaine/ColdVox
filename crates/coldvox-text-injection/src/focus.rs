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

#[async_trait]
impl FocusBackend for SystemFocusAdapter {
    async fn query_focus(&self) -> Result<FocusStatus, InjectionError> {
        // Temporarily disabled due to AT-SPI API changes
        // TODO(#38): Update to work with current atspi crate API
        Ok(FocusStatus::Unknown)
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
