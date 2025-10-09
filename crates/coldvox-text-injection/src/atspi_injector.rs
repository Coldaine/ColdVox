// Real implementation when the `atspi` feature is enabled.
#[cfg(feature = "atspi")]
mod real {
    use crate::log_throttle::log_atspi_connection_failure;
    use crate::types::{InjectionConfig, InjectionResult};
    use crate::TextInjector;
    use async_trait::async_trait;
    use std::time::Instant;
    use tracing::{debug, trace, warn};

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
            use atspi::connection::AccessibilityConnection;
            use tokio::time;

            let timeout_duration = self._config.per_method_timeout();

            let availability_check = async { AccessibilityConnection::new().await.is_ok() };

            match time::timeout(timeout_duration, availability_check).await {
                Ok(is_ok) => is_ok,
                Err(_) => {
                    warn!(
                        "AT-SPI availability check timed out after {}ms",
                        timeout_duration.as_millis()
                    );
                    false
                }
            }
        }

        async fn inject_text(&self, text: &str) -> InjectionResult<()> {
            use crate::types::InjectionError;
            use atspi::{
                connection::AccessibilityConnection, proxy::collection::CollectionProxy,
                proxy::editable_text::EditableTextProxy, proxy::text::TextProxy, Interface,
                MatchType, ObjectMatchRule, SortOrder, State,
            };
            use tokio::time;

            let per_method_timeout = self._config.per_method_timeout();

            let start = Instant::now();
            trace!("AT-SPI injection starting for {} chars of text", text.len());

            let conn = time::timeout(per_method_timeout, AccessibilityConnection::new())
                .await
                .map_err(|_| InjectionError::Timeout(per_method_timeout.as_millis() as u64))?
                .map_err(|e| {
                    log_atspi_connection_failure(&e.to_string());
                    InjectionError::Other(format!("AT-SPI connect failed: {e}"))
                })?;
            let zbus_conn = conn.connection();
            trace!("AT-SPI connection established");

            let collection_fut = CollectionProxy::builder(zbus_conn)
                .destination("org.a11y.atspi.Registry")
                .map_err(|e| {
                    InjectionError::Other(format!("CollectionProxy destination failed: {e}"))
                })?
                .path("/org/a11y/atspi/accessible/root")
                .map_err(|e| InjectionError::Other(format!("CollectionProxy path failed: {e}")))?
                .build();
            let collection = time::timeout(per_method_timeout, collection_fut)
                .await
                .map_err(|_| InjectionError::Timeout(per_method_timeout.as_millis() as u64))?
                .map_err(|e| InjectionError::Other(format!("CollectionProxy build failed: {e}")))?;

            let mut rule = ObjectMatchRule::default();
            rule.states = State::Focused.into();
            rule.states_mt = MatchType::All;
            rule.ifaces = Interface::EditableText.into();
            rule.ifaces_mt = MatchType::All;

            let get_matches = collection.get_matches(rule.clone(), SortOrder::Canonical, 1, false);
            let mut matches = time::timeout(per_method_timeout, get_matches)
                .await
                .map_err(|_| InjectionError::Timeout(per_method_timeout.as_millis() as u64))?
                .map_err(|e| {
                    InjectionError::Other(format!("Collection.get_matches failed: {e}"))
                })?;

            if matches.is_empty() {
                debug!("No focused EditableText found, retrying once after 30ms");
                tokio::time::sleep(std::time::Duration::from_millis(30)).await;

                let retry = collection.get_matches(rule, SortOrder::Canonical, 1, false);
                matches = time::timeout(per_method_timeout, retry)
                    .await
                    .map_err(|_| InjectionError::Timeout(per_method_timeout.as_millis() as u64))?
                    .map_err(|e| {
                        InjectionError::Other(format!("Collection.get_matches retry failed: {e}"))
                    })?;
            }

            let Some(obj_ref) = matches.pop() else {
                debug!(
                    "No focused EditableText found after retry ({}ms elapsed)",
                    start.elapsed().as_millis()
                );
                return Err(crate::types::InjectionError::NoEditableFocus);
            };

            debug!(
                "Found editable element at path: {:?} in app: {:?}",
                obj_ref.path, obj_ref.name
            );

            let editable_fut = EditableTextProxy::builder(zbus_conn)
                .destination(obj_ref.name.clone())
                .map_err(|e| {
                    InjectionError::Other(format!("EditableTextProxy destination failed: {e}"))
                })?
                .path(obj_ref.path.clone())
                .map_err(|e| InjectionError::Other(format!("EditableTextProxy path failed: {e}")))?
                .build();
            let editable = time::timeout(per_method_timeout, editable_fut)
                .await
                .map_err(|_| InjectionError::Timeout(per_method_timeout.as_millis() as u64))?
                .map_err(|e| {
                    InjectionError::Other(format!("EditableTextProxy build failed: {e}"))
                })?;

            let text_iface_fut = TextProxy::builder(zbus_conn)
                .destination(obj_ref.name.clone())
                .map_err(|e| InjectionError::Other(format!("TextProxy destination failed: {e}")))?
                .path(obj_ref.path.clone())
                .map_err(|e| InjectionError::Other(format!("TextProxy path failed: {e}")))?
                .build();
            let text_iface = time::timeout(per_method_timeout, text_iface_fut)
                .await
                .map_err(|_| InjectionError::Timeout(per_method_timeout.as_millis() as u64))?
                .map_err(|e| InjectionError::Other(format!("TextProxy build failed: {e}")))?;

            let caret_fut = text_iface.caret_offset();
            let caret = time::timeout(per_method_timeout, caret_fut)
                .await
                .map_err(|_| InjectionError::Timeout(per_method_timeout.as_millis() as u64))?
                .map_err(|e| {
                    warn!("Failed to get caret offset from {:?}: {}", obj_ref.path, e);
                    InjectionError::Other(format!("Text.caret_offset failed: {e}"))
                })?;
            trace!("Current caret position: {}", caret);

            let insert_fut = editable.insert_text(caret, text, text.chars().count() as i32);
            time::timeout(per_method_timeout, insert_fut)
                .await
                .map_err(|_| InjectionError::Timeout(per_method_timeout.as_millis() as u64))?
                .map_err(|e| {
                    warn!(
                        "Failed to insert text at position {} in {:?}: {}",
                        caret, obj_ref.path, e
                    );
                    InjectionError::Other(format!("EditableText.insert_text failed: {e}"))
                })?;

            let elapsed = start.elapsed();
            debug!(
                "Successfully injected {} chars via AT-SPI to {:?} in {}ms",
                text.len(),
                obj_ref.name,
                elapsed.as_millis()
            );

            Ok(())
        }
    }
}

// Lightweight stub implementation when `atspi` feature is disabled. This
// preserves the public type so other modules can compile without cfg
// branches. The stub reports unavailability and returns an error for inject.
#[cfg(not(feature = "atspi"))]
mod stub {
    use crate::types::{InjectionConfig, InjectionError, InjectionResult};
    use crate::TextInjector;
    use async_trait::async_trait;
    use tracing::warn;

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
            "atspi-insert-stub"
        }

        fn backend_info(&self) -> Vec<(&'static str, String)> {
            vec![("type", "AT-SPI stub".to_string())]
        }

        async fn is_available(&self) -> bool {
            warn!("AT-SPI feature disabled; AtspiInjector stub is not available");
            false
        }

        async fn inject_text(&self, _text: &str) -> InjectionResult<()> {
            Err(InjectionError::Other("AT-SPI feature not enabled".to_string()))
        }
    }
}

// Re-export the appropriate implementation at module root so the rest of the
// crate can refer to `atspi_injector::AtspiInjector` uniformly.
#[cfg(feature = "atspi")]
pub use real::AtspiInjector;

#[cfg(not(feature = "atspi"))]
pub use stub::AtspiInjector;
