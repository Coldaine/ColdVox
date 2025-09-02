'''use crate::types::{InjectionConfig, InjectionError};
use std::time::{Duration, Instant};
use tracing::debug;

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

    /// Check the actual focus status using AT-SPI
    async fn check_focus_status(&self) -> Result<FocusStatus, InjectionError> {
        #[cfg(feature = "atspi")]
        {
            use atspi::{
                connection::AccessibilityConnection,
                common::{Interface, MatchType, ObjectMatchRule, SortOrder, State, StateSet},
            };
            use atspi_proxies::{accessible::AccessibleProxy, collection::CollectionProxy};

            let conn = match AccessibilityConnection::new().await {
                Ok(c) => c,
                Err(err) => {
                    debug!(error = ?err, "AT-SPI: failed to connect");
                    return Ok(FocusStatus::Unknown);
                }
            };
            let zbus_conn = conn.connection();

            let root = match AccessibleProxy::new(zbus_conn).await {
                Ok(p) => p,
                Err(err) => {
                    debug!(error = ?err, "AT-SPI: failed to create root AccessibleProxy");
                    return Ok(FocusStatus::Unknown);
                }
            };

            let root_dest = root.inner().destination().to_owned();
            let root_path = root.inner().path().to_owned();
            let collection = match CollectionProxy::builder(zbus_conn)
                .destination(root_dest).map_err(|e| format!("Bad dest: {e}"))?
                .path(root_path).map_err(|e| format!("Bad path: {e}"))?
                .build()
                .await
            {
                Ok(p) => p,
                Err(err) => {
                    debug!(error = ?err, "AT-SPI: failed to create CollectionProxy on root");
                    return Ok(FocusStatus::Unknown);
                }
            };

            let states = StateSet::new([State::Focused]);
            let rule = ObjectMatchRule {
                states: Some(states),
                states_match_type: MatchType::All,
                attributes: None,
                attributes_match_type: MatchType::All,
                roles: None,
                roles_match_type: MatchType::All,
                interfaces: Some(vec![Interface::EditableText]),
                interfaces_match_type: MatchType::All,
                invert: false,
            };

            let matches = match collection
                .get_matches(&rule, SortOrder::Canonical, 1, false)
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
''