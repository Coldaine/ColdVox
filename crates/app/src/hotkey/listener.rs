use coldvox_vad::types::VadEvent;
use futures::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::Sender;
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};
use zbus::{Connection, Proxy};

pub struct PortalShortcuts {
    connection: Arc<Connection>,
    portal: Proxy<'static>,
}

impl PortalShortcuts {
    pub async fn new() -> Result<Self, zbus::Error> {
        let connection = Connection::session().await?;
        let portal = Proxy::new(
            &connection,
            "org.freedesktop.portal.Desktop",
            "/org/freedesktop/portal/desktop",
            "org.freedesktop.portal.GlobalShortcuts",
        )
        .await?;

        Ok(PortalShortcuts {
            connection: Arc::new(connection),
            portal,
        })
    }

    async fn wait_request_response(
        &self,
        request_path: &OwnedObjectPath,
        timeout: Duration,
    ) -> Result<(u32, HashMap<String, OwnedValue>), zbus::Error> {
        // Create a proxy to the Request object and listen for the Response signal
        let req_proxy = Proxy::new(
            &*self.connection,
            "org.freedesktop.portal.Desktop",
            request_path.as_str(),
            "org.freedesktop.portal.Request",
        )
        .await?;

        // Receive a single Response signal with timeout
        let mut stream = req_proxy.receive_signal("Response").await?;
        let msg = tokio::time::timeout(timeout, async { StreamExt::next(&mut stream).await })
            .await
            .map_err(|_| {
                zbus::Error::InputOutput(Arc::new(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "portal request timed out",
                )))
            })?
            .ok_or_else(|| {
                zbus::Error::InputOutput(Arc::new(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "portal request stream ended",
                )))
            })?;

        // Response signal body: (u status, a{sv} results)
        // Extract body now to avoid lifetime issues with `msg`
        let body: Result<(u32, HashMap<String, OwnedValue>), _> = msg.body();
        match body {
            Ok((status, results)) => Ok((status, results)),
            Err(e) => Err(zbus::Error::InputOutput(Arc::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("failed to decode portal Response: {e}"),
            )))),
        }
    }

    pub async fn create_session(&self) -> Result<OwnedObjectPath, zbus::Error> {
        // Provide handle tokens to correlate requests
        // Simple tokens derived from time to avoid extra deps
        let now_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let handle_token = format!("coldvox_{:x}", now_ns);
        let session_handle_token = format!("session_{:x}", now_ns ^ 0xA5A5_5A5A);

        let mut options: HashMap<String, Value> = HashMap::new();
        options.insert("handle_token".into(), Value::from(handle_token.as_str()));
        options.insert(
            "session_handle_token".into(),
            Value::from(session_handle_token.as_str()),
        );

        // CreateSession returns a Request object path
        let request_handle: OwnedObjectPath = self.portal.call("CreateSession", &(options)).await?;

        // Wait for Response to get the session_handle (string-encoded object path)
        let (_status, results) = self
            .wait_request_response(&request_handle, Duration::from_secs(10))
            .await?;

        if let Some(v) = results.get("session_handle") {
            let session_handle: String = v.clone().try_into().map_err(|e| {
                zbus::Error::InputOutput(Arc::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("invalid session_handle: {e}"),
                )))
            })?;
            // Convert to object path for subsequent calls
            let opath = OwnedObjectPath::try_from(session_handle.as_str()).map_err(|e| {
                zbus::Error::InputOutput(Arc::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("invalid session_handle: {e}"),
                )))
            })?;
            Ok(opath)
        } else {
            Err(zbus::Error::InputOutput(Arc::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "missing session_handle in portal response",
            ))))
        }
    }

    pub async fn bind_shortcuts(
        &self,
        session_handle: &OwnedObjectPath,
        shortcuts: Vec<(String, String)>, // (id, description)
    ) -> Result<(), zbus::Error> {
        // Build a(sa{sv}) array: (id, { description, preferred_trigger? })
        let mut sc_defs: Vec<(String, HashMap<String, Value>)> = Vec::new();
        for (id, description) in shortcuts.iter() {
            let mut map: HashMap<String, Value> = HashMap::new();
            map.insert("description".into(), Value::from(description.as_str()));
            sc_defs.push((id.clone(), map));
        }

        let parent_window = ""; // no parent window available in TUI
        let mut options: HashMap<String, Value> = HashMap::new();
        let now_ns2 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        options.insert(
            "handle_token".into(),
            Value::from(format!("bind_{:x}", now_ns2 & 0xffff)),
        );

        // BindShortcuts returns a Request handle; wait for Response for confirmation
        let request_handle: OwnedObjectPath = self
            .portal
            .call(
                "BindShortcuts",
                &(session_handle.clone(), sc_defs, parent_window, options),
            )
            .await?;

        let (_status, _results) = self
            .wait_request_response(&request_handle, Duration::from_secs(15))
            .await?;
        Ok(())
    }
}

/// Spawn the KDE Plasma/XDG GlobalShortcuts listener and forward events as VadEvents.
/// Registers a single shortcut id "coldvox_ptt" with a human description. The actual
/// trigger is user-configurable via the portal dialog.
pub fn spawn_hotkey_listener(event_tx: Sender<VadEvent>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let start = Instant::now();

        // 1) Connect to portal and create a session
        let shortcuts = match PortalShortcuts::new().await {
            Ok(p) => p,
            Err(e) => {
                tracing::error!("GlobalShortcuts portal unavailable: {e}");
                return;
            }
        };

        let session_handle = match shortcuts.create_session().await {
            Ok(h) => h,
            Err(e) => {
                tracing::error!("Failed to create GlobalShortcuts session: {e}");
                return;
            }
        };

        // 2) Bind our shortcuts (id, description). No preferred trigger here; let user pick.
        if let Err(e) = shortcuts
            .bind_shortcuts(
                &session_handle,
                vec![(
                    "coldvox_ptt".to_string(),
                    "ColdVox Push-to-talk".to_string(),
                )],
            )
            .await
        {
            tracing::error!("Failed to bind shortcuts: {e}");
            return;
        }

        // 3) Subscribe to Activated/Deactivated signals and forward as VadEvents
        let mut activated_stream = match shortcuts.portal.receive_signal("Activated").await {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to subscribe to Activated signal: {e}");
                return;
            }
        };
        let mut deactivated_stream = match shortcuts.portal.receive_signal("Deactivated").await {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to subscribe to Deactivated signal: {e}");
                return;
            }
        };

        tracing::info!(
            "GlobalShortcuts listener ready (session: {})",
            session_handle.as_str()
        );

        loop {
            tokio::select! {
                Some(msg) = futures::StreamExt::next(&mut activated_stream) => {
                    // Signal: (session_handle o, shortcut_id s, timestamp t, options a{sv})
                    if let Ok((session_o, shortcut_id, _ts, _opts)) = msg.body::<(OwnedObjectPath, String, u64, HashMap<String, OwnedValue>)>() {
                        if session_o.as_str() == session_handle.as_str() && shortcut_id == "coldvox_ptt" {
                            let ts_ms = start.elapsed().as_millis() as u64;
                            let _ = event_tx.send(VadEvent::SpeechStart { timestamp_ms: ts_ms, energy_db: 0.0 }).await;
                        }
                    }
                }
                Some(msg) = futures::StreamExt::next(&mut deactivated_stream) => {
                    if let Ok((session_o, shortcut_id, _ts, _opts)) = msg.body::<(OwnedObjectPath, String, u64, HashMap<String, OwnedValue>)>() {
                        if session_o.as_str() == session_handle.as_str() && shortcut_id == "coldvox_ptt" {
                            let ts_ms = start.elapsed().as_millis() as u64;
                            // Duration isn't provided by portal; send 0 and let downstream compute if needed
                            let _ = event_tx.send(VadEvent::SpeechEnd { timestamp_ms: ts_ms, duration_ms: 0, energy_db: 0.0 }).await;
                        }
                    }
                }
                else => {
                    // Streams ended; likely portal exited. Exit task.
                    tracing::warn!("GlobalShortcuts signal streams ended; exiting listener");
                    break;
                }
            }
        }
    })
}
