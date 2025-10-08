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
            // Temporarily disabled due to AT-SPI API changes
            // TODO(#38): Update to work with current atspi crate API
            return Ok(FocusStatus::Unknown);
        }

        // Fallback: check if we can get focused element via other methods
        #[cfg(feature = "wl_clipboard")]
        {
            use std::process::Command;

            // Check for focused window using xdotool or similar
            let output = Command::new("xdotool").arg("getwindowfocus").output();
            if let Ok(output) = output {
                if !output.stdout.is_empty() {
                    return Ok(FocusStatus::NonEditable);
                }
            }
        }

        Ok(FocusStatus::Unknown)
    }

    #[cfg(feature = "atspi")]
    async fn get_atspi_focus_status(&mut self) -> Result<FocusStatus, InjectionError> {
        // Temporarily disabled due to AT-SPI API changes
        // TODO(#38): Update to work with current atspi crate API
        /*
        use atspi::{
            connection::AccessibilityConnection, proxy::component::ComponentProxy,
            Interface, State,
        };
        use tokio::time::timeout;

        // Connect to accessibility bus
        let conn = match AccessibilityConnection::new().await {
            Ok(conn) => conn,
            Err(_) => return Ok(FocusStatus::Unknown),
        };

        let zbus_conn = conn.connection();
        let desktop = conn.desktop();

        // Get the active window
        let active_window = match timeout(std::time::Duration::from_millis(100), desktop.active_window()).await {
            Ok(window) => window,
            Err(_) => return Ok(FocusStatus::Unknown),
        };

        if active_window.is_none() {
            return Ok(FocusStatus::Unknown);
        }

        let active_window = active_window.unwrap();
        let active_window_proxy = match ComponentProxy::builder(zbus_conn)
            .destination(active_window.name.clone())?
            .path(active_window.path.clone())?
            .build()
            .await
        {
            Ok(proxy) => proxy,
            Err(_) => return Ok(FocusStatus::Unknown),
        };

        // Check if the active window has focus
        let states = active_window_proxy.get_state().await.unwrap_or_default();
        if states.contains(State::Focused) {
            return Ok(FocusStatus::NonEditable);
        }

        // Check if the active window has editable text
        if states.contains(State::Editable) {
            return Ok(FocusStatus::EditableText);
        }
        */

        Ok(FocusStatus::Unknown)
    }
}

#[async_trait::async_trait]
impl FocusProvider for FocusTracker {
    async fn get_focus_status(&mut self) -> Result<FocusStatus, InjectionError> {
        FocusTracker::get_focus_status(self).await
    }
}
