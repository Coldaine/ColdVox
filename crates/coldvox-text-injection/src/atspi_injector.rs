use crate::types::{InjectionConfig, InjectionResult};
use crate::TextInjector;
use async_trait::async_trait;
use tracing::{debug, warn};

pub struct AtspiInjector {
    _config: InjectionConfig,
}

impl AtspiInjector {
    pub fn new(config: InjectionConfig) -> Self {
        Self { _config: config }
    }
}

#[async_trait]
impl TextInjector for AtspiInjector {
    fn backend_name(&self) -> &'static str {
        "atspi-insert"
    }

    fn backend_info(&self) -> Vec<(&'static str, String)> {
        vec![
            ("type", "AT-SPI accessibility".to_string()),
            (
                "description",
                "Injects text directly into focused editable text fields using AT-SPI".to_string(),
            ),
            ("platform", "Linux".to_string()),
            ("requires", "AT-SPI accessibility service".to_string()),
        ]
    }

    async fn is_available(&self) -> bool {
        #[cfg(feature = "atspi")]
        {
            use atspi::connection::AccessibilityConnection;

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

    async fn inject_text(&self, text: &str) -> InjectionResult<()> {
        #[cfg(feature = "atspi")]
        {
            use atspi::{
                connection::AccessibilityConnection, proxy::collection::CollectionProxy,
                proxy::editable_text::EditableTextProxy, proxy::text::TextProxy, Interface,
                MatchType, ObjectMatchRule, SortOrder, State,
            };

            let conn = AccessibilityConnection::new().await.map_err(|e| {
                crate::types::InjectionError::Other(format!("AT-SPI connect failed: {e}"))
            })?;
            let zbus_conn = conn.connection();

            let collection = CollectionProxy::builder(zbus_conn)
                .destination("org.a11y.atspi.Registry")
                .map_err(|e| {
                    crate::types::InjectionError::Other(format!(
                        "CollectionProxy destination failed: {e}"
                    ))
                })?
                .path("/org/a11y/atspi/accessible/root")
                .map_err(|e| {
                    crate::types::InjectionError::Other(format!("CollectionProxy path failed: {e}"))
                })?
                .build()
                .await
                .map_err(|e| {
                    crate::types::InjectionError::Other(format!(
                        "CollectionProxy build failed: {e}"
                    ))
                })?;

            let mut rule = ObjectMatchRule::default();
            rule.states = State::Focused.into();
            rule.states_mt = MatchType::All;
            rule.ifaces = Interface::EditableText.into();
            rule.ifaces_mt = MatchType::All;

            let mut matches = collection
                .get_matches(rule, SortOrder::Canonical, 1, false)
                .await
                .map_err(|e| {
                    crate::types::InjectionError::Other(format!(
                        "Collection.get_matches failed: {e}"
                    ))
                })?;

            let Some(obj_ref) = matches.pop() else {
                debug!("No focused EditableText found");
                return Err(crate::types::InjectionError::NoEditableFocus);
            };

            let editable = EditableTextProxy::builder(zbus_conn)
                .destination(obj_ref.name.clone())
                .map_err(|e| {
                    crate::types::InjectionError::Other(format!(
                        "EditableTextProxy destination failed: {e}"
                    ))
                })?
                .path(obj_ref.path.clone())
                .map_err(|e| {
                    crate::types::InjectionError::Other(format!(
                        "EditableTextProxy path failed: {e}"
                    ))
                })?
                .build()
                .await
                .map_err(|e| {
                    crate::types::InjectionError::Other(format!(
                        "EditableTextProxy build failed: {e}"
                    ))
                })?;

            let text_iface = TextProxy::builder(zbus_conn)
                .destination(obj_ref.name.clone())
                .map_err(|e| {
                    crate::types::InjectionError::Other(format!(
                        "TextProxy destination failed: {e}"
                    ))
                })?
                .path(obj_ref.path.clone())
                .map_err(|e| {
                    crate::types::InjectionError::Other(format!("TextProxy path failed: {e}"))
                })?
                .build()
                .await
                .map_err(|e| {
                    crate::types::InjectionError::Other(format!("TextProxy build failed: {e}"))
                })?;

            let caret = text_iface.caret_offset().await.map_err(|e| {
                crate::types::InjectionError::Other(format!("Text.caret_offset failed: {e}"))
            })?;

            editable
                .insert_text(caret, text, text.chars().count() as i32)
                .await
                .map_err(|e| {
                    crate::types::InjectionError::Other(format!(
                        "EditableText.insert_text failed: {e}"
                    ))
                })?;

            Ok(())
        }

        #[cfg(not(feature = "atspi"))]
        {
            warn!("AT-SPI injector compiled without 'atspi' feature");
            Err(crate::types::InjectionError::Other(
                "AT-SPI feature is disabled at compile time".to_string(),
            ))
        }
    }
}
