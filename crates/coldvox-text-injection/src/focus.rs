use crate::error::InjectionError;
use crate::types::InjectionConfig;
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
        // NOTE: The detailed timing configs were removed from InjectionConfig.
        // Using a hardcoded constant is acceptable for now as this logic is
        // not on the critical path of the main refactoring.
        let cache_duration = Duration::from_millis(200);
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

    async fn check_focus_status(&self) -> Result<FocusStatus, InjectionError> {
        #[cfg(feature = "atspi")]
        {
            use atspi::{
                connection::AccessibilityConnection, proxy::collection::CollectionProxy, Interface,
                MatchType, ObjectMatchRule, SortOrder, State,
            };

            let conn = match AccessibilityConnection::new().await {
                Ok(c) => c,
                Err(err) => {
                    debug!(error = ?err, "AT-SPI: failed to connect");
                    return Ok(FocusStatus::Unknown);
                }
            };
            let zbus_conn = conn.connection();

            let builder = CollectionProxy::builder(zbus_conn);
            let builder = match builder.destination("org.a11y.atspi.Registry") {
                Ok(b) => b,
                Err(e) => {
                    debug!(error = ?e, "AT-SPI: failed to set destination");
                    return Ok(FocusStatus::Unknown);
                }
            };
            let builder = match builder.path("/org/a11y/atspi/accessible/root") {
                Ok(b) => b,
                Err(e) => {
                    debug!(error = ?e, "AT-SPI: failed to set path");
                    return Ok(FocusStatus::Unknown);
                }
            };
            let collection = match builder.build().await {
                Ok(p) => p,
                Err(err) => {
                    debug!(error = ?err, "AT-SPI: failed to create CollectionProxy on root");
                    return Ok(FocusStatus::Unknown);
                }
            };

            let mut rule = ObjectMatchRule::default();
            rule.states = State::Focused.into();
            rule.states_mt = MatchType::All;
            rule.ifaces = Interface::EditableText.into();
            rule.ifaces_mt = MatchType::All;

            let matches = match collection
                .get_matches(rule, SortOrder::Canonical, 1, false)
                .await
            {
                Ok(v) => v,
                Err(err) => {
                    debug!(error = ?err, "AT-SPI: Collection.get_matches failed");
                    return Ok(FocusStatus::Unknown);
                }
            };

            if matches.is_empty() {
                return Ok(FocusStatus::NonEditable);
            }

            Ok(FocusStatus::EditableText)
        }

        #[cfg(not(feature = "atspi"))]
        {
            debug!("AT-SPI feature disabled; focus status unknown");
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
