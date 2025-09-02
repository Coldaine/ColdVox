use crate::types::{InjectionConfig, InjectionError, InjectionMetrics, TextInjector};
use async_trait::async_trait;
use tracing::{debug, warn};

pub struct AtspiInjector {
    _config: InjectionConfig,
    metrics: InjectionMetrics,
}

impl AtspiInjector {
    pub fn new(config: InjectionConfig) -> Self {
        Self {
            _config: config,
            metrics: InjectionMetrics::default(),
        }
    }

    /// Lightweight availability probe. Synchronously attempts an AT-SPI connection.
    pub fn is_available(&self) -> bool {
        #[cfg(feature = "atspi")]
        {
            use atspi::connection::AccessibilityConnection;

            // Try on the current Tokio runtime if present; else spin a tiny current-thread RT.
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                return handle.block_on(AccessibilityConnection::new()).is_ok();
            }
            match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt.block_on(AccessibilityConnection::new()).is_ok(),
                Err(_) => {
                    warn!("AT-SPI availability check: failed to create a runtime");
                    false
                }
            }
        }
        #[cfg(not(feature = "atspi"))]
        {
            warn!("AT-SPI feature disabled; injector unavailable");
            false
        }
    }
}

#[async_trait]
impl TextInjector for AtspiInjector {
    fn name(&self) -> &'static str {
        "atspi-insert"
    }

    fn metrics(&self) -> &InjectionMetrics {
        &self.metrics
    }

    async fn inject(&mut self, text: &str) -> Result<(), InjectionError> {
        #[cfg(feature = "atspi")]
        {
            use atspi::{
                connection::AccessibilityConnection,
                proxy::{
                    editable_text::EditableTextProxy,
                    proxy_ext::ProxyExt,
                    text::TextProxy,
                    accessible::AccessibleProxy,
                    collection::CollectionProxy,
                },
                Interface, MatchType, ObjectMatchRule, SortOrder, State, StateSet,
            };

            // 1) Connect
            let conn = AccessibilityConnection::new()
                .await
                .map_err(|e| InjectionError::Other(format!("AT-SPI connect failed: {e}")))?;

            // 2) Bind root accessible + collection (same pattern as focus tracking)
            let root = AccessibleProxy::new(&conn)
                .await
                .map_err(|e| InjectionError::Other(format!("root AccessibleProxy failed: {e}")))?;
            let root_dest = root.inner().destination().to_owned();
            let root_path = root.inner().path().to_owned();
            let collection = CollectionProxy::builder(&conn)
                .destination(root_dest)
                .path(root_path)
                .build()
                .await
                .map_err(|e| InjectionError::Other(format!("CollectionProxy failed: {e}")))?;

            // 3) Find the *focused editable* object
            let states = StateSet::from_iter([State::Focused]);
            let rule = ObjectMatchRule {
                states: Some(states),
                states_match: MatchType::All,
                attributes: None,
                attributes_match: MatchType::None,
                roles: None,
                roles_match: MatchType::None,
                interfaces: Some(vec![Interface::EditableText]),
                interfaces_match: MatchType::All,
                invert: false,
            };

            let mut matches = collection
                .get_matches(&(&rule).into(), SortOrder::Canonical, 1, false)
                .await
                .map_err(|e| InjectionError::Other(format!("Collection.get_matches failed: {e}")))?;

            let Some(objref) = matches.pop() else {
                debug!("No focused EditableText found");
                return Err(InjectionError::NoEditableFocus);
            };

            // 4) Proxies on the *same object* (destination + path from ObjectRef)
            let editable = EditableTextProxy::builder(&conn)
                .destination(objref.name().to_owned())
                .path(objref.path().to_owned())
                .build()
                .await
                .map_err(|e| InjectionError::Other(format!("EditableTextProxy failed: {e}")))?;

            let text_iface = TextProxy::builder(&conn)
                .destination(objref.name().to_owned())
                .path(objref.path().to_owned())
                .build()
                .await
                .map_err(|e| InjectionError::Other(format!("TextProxy failed: {e}")))?;

            // 5) Insert at current caret
            let caret = text_iface
                .caret_offset()
                .await
                .map_err(|e| InjectionError::Other(format!("Text.caret_offset failed: {e}")))?;

            let len = text.chars().count() as i32;
            editable
                .insert_text(caret, text, len)
                .await
                .map_err(|e| InjectionError::Other(format!("EditableText.insert_text failed: {e}")))?;

            Ok(())
        }

        #[cfg(not(feature = "atspi"))]
        {
            warn!("AT-SPI injector compiled without 'atspi' feature");
            Err(InjectionError::Other(
                "AT-SPI feature is disabled at compile time".to_string(),
            ))
        }
    }
}