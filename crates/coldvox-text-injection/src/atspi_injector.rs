//! # AT-SPI Text Injector
//!
//! This module provides a text injection implementation using the AT-SPI
//! (Assistive Technology Service Provider Interface) D-Bus API. It is the
//! most reliable method for injecting text into native GUI applications on Linux.

use crate::constants::{ATSPI_METHOD_TIMEOUT_MS, FOCUS_ACQUISITION_TIMEOUT_MS, READINESS_POLL_INTERVAL_MS};
use crate::error::{InjectionError, UnavailableCause};
use crate::outcome::InjectionOutcome;
use crate::probe::BackendId;
use crate::{async_trait, InjectionConfig, TextInjector};
use atspi::connection::AccessibilityConnection;
use atspi::proxy::collection::CollectionProxy;
use atspi::proxy::editable_text::EditableTextProxy;
use atspi::proxy::text::TextProxy;
use atspi::{Interface, MatchType, ObjectAddress, ObjectMatchRule, SortOrder, State};
use std::time::{Duration, Instant};
use tokio::time::timeout;
use tracing::{debug, trace, warn};

pub struct AtspiInjector {
    config: InjectionConfig,
}

impl AtspiInjector {
    pub fn new(config: InjectionConfig) -> Self {
        Self { config }
    }
}

/// Waits for a focusable, editable element to become active.
///
/// This replaces fixed sleeps with a polling mechanism under a strict timeout,
/// making focus acquisition more robust.
async fn wait_for_editable_focus(
    conn: &AccessibilityConnection,
) -> Result<ObjectAddress, InjectionError> {
    let deadline = Instant::now() + Duration::from_millis(FOCUS_ACQUISITION_TIMEOUT_MS);
    let zbus_conn = conn.connection();

    let collection = CollectionProxy::builder(zbus_conn)
        .destination("org.a11y.atspi.Registry")
        .path("/org/a11y/atspi/accessible/root")
        .build()
        .await
        .map_err(|e| InjectionError::Unavailable {
            backend: BackendId::Atspi,
            cause: UnavailableCause::AtspiRegistry,
        })?;

    loop {
        if Instant::now() > deadline {
            return Err(InjectionError::Timeout {
                backend: BackendId::Atspi,
                phase: "focus",
                elapsed_ms: FOCUS_ACQUISITION_TIMEOUT_MS as u32,
            });
        }

        let mut rule = ObjectMatchRule::default();
        rule.states = State::Focused.into();
        rule.states_mt = MatchType::All;
        rule.ifaces = Interface::EditableText.into();
        rule.ifaces_mt = MatchType::All;

        let get_matches = collection.get_matches(rule, SortOrder::Canonical, 1, false);
        match timeout(Duration::from_millis(ATSPI_METHOD_TIMEOUT_MS), get_matches).await {
            Ok(Ok(mut matches)) => {
                if let Some(obj_ref) = matches.pop() {
                    debug!("Found focused editable element: {:?}", obj_ref.name);
                    return Ok(obj_ref);
                }
                // No match found yet, continue polling.
            }
            Ok(Err(e)) => {
                // An error occurred during the D-Bus call.
                return Err(InjectionError::Transient {
                    reason: "Failed to get matches from AT-SPI registry",
                    retryable: false, // This is likely a permanent issue.
                });
            }
            Err(_) => {
                // The D-Bus call timed out.
                return Err(InjectionError::Timeout {
                    backend: BackendId::Atspi,
                    phase: "get_matches",
                    elapsed_ms: ATSPI_METHOD_TIMEOUT_MS as u32,
                });
            }
        }

        tokio::time::sleep(Duration::from_millis(READINESS_POLL_INTERVAL_MS)).await;
    }
}

#[async_trait]
impl TextInjector for AtspiInjector {
    fn backend_id(&self) -> BackendId {
        BackendId::Atspi
    }

    async fn is_available(&self) -> bool {
        match timeout(
            Duration::from_millis(ATSPI_METHOD_TIMEOUT_MS),
            AccessibilityConnection::new(),
        )
        .await
        {
            Ok(Ok(_)) => true,
            _ => false,
        }
    }

    async fn inject_text(&self, text: &str) -> Result<InjectionOutcome, InjectionError> {
        let start_time = Instant::now();

        // 1. Establish AT-SPI connection.
        let conn = timeout(
            Duration::from_millis(ATSPI_METHOD_TIMEOUT_MS),
            AccessibilityConnection::new(),
        )
        .await
        .map_err(|_| InjectionError::Timeout {
            backend: BackendId::Atspi,
            phase: "connect",
            elapsed_ms: ATSPI_METHOD_TIMEOUT_MS as u32,
        })?
        .map_err(|_| InjectionError::Unavailable {
            backend: BackendId::Atspi,
            cause: UnavailableCause::AtspiRegistry,
        })?;
        let zbus_conn = conn.connection();

        // 2. Wait for a focused, editable element.
        let obj_ref = wait_for_editable_focus(&conn).await?;
        debug!("Injecting into element: {:?}", obj_ref.name);

        // 3. Build proxies for the target element.
        let editable_proxy = EditableTextProxy::builder(zbus_conn)
            .destination(obj_ref.name.clone())
            .path(obj_ref.path.clone())
            .build()
            .await
            .map_err(|_| InjectionError::PreconditionNotMet {
                reason: "Failed to build EditableTextProxy for focused element",
            })?;

        let text_proxy = TextProxy::builder(zbus_conn)
            .destination(obj_ref.name.clone())
            .path(obj_ref.path.clone())
            .build()
            .await
            .map_err(|_| InjectionError::PreconditionNotMet {
                reason: "Failed to build TextProxy for focused element",
            })?;

        // 4. Get caret position.
        let caret_pos = timeout(
            Duration::from_millis(ATSPI_METHOD_TIMEOUT_MS),
            text_proxy.caret_offset(),
        )
        .await
        .map_err(|_| InjectionError::Timeout {
            backend: BackendId::Atspi,
            phase: "get_caret",
            elapsed_ms: ATSPI_METHOD_TIMEOUT_MS as u32,
        })?
        .unwrap_or(-1); // -1 indicates failure, we'll insert at the end.

        let insertion_pos = if caret_pos < 0 {
            warn!("Failed to get caret position, inserting at end.");
            // As a fallback, get character count and insert at the end.
            text_proxy.character_count().await.unwrap_or(0)
        } else {
            caret_pos
        };

        // 5. Insert the text.
        let insert_fut = editable_proxy.insert_text(insertion_pos, text, text.chars().count() as i32);
        timeout(
            Duration::from_millis(ATSPI_METHOD_TIMEOUT_MS),
            insert_fut,
        )
        .await
        .map_err(|_| InjectionError::Timeout {
            backend: BackendId::Atspi,
            phase: "insert_text",
            elapsed_ms: ATSPI_METHOD_TIMEOUT_MS as u32,
        })?
        .map_err(|e| InjectionError::Transient {
            reason: "insert_text D-Bus call failed",
            retryable: false,
        })?;

        let latency_ms = start_time.elapsed().as_millis() as u32;
        trace!("AT-SPI injection successful in {}ms", latency_ms);

        Ok(InjectionOutcome {
            backend: BackendId::Atspi,
            latency_ms,
            degraded: caret_pos < 0, // Degraded if we couldn't get the caret pos.
        })
    }
}
