use super::backend::{BackendStatus, HotkeyBackend, Shortcut};
use async_trait::async_trait;
use coldvox_vad::types::VadEvent;
use futures::StreamExt;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::Sender;
use zbus::{Connection, Proxy};

/// KDE KGlobalAccel backend for native Plasma integration
pub struct KGlobalAccelBackend {
    connection: Option<Arc<Connection>>,
    component_name: String,
    action_name: String,
    shortcuts: Vec<Shortcut>,
}

impl Default for KGlobalAccelBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl KGlobalAccelBackend {
    pub fn new() -> Self {
        Self::with_names("coldvox".to_string(), "push_to_talk".to_string())
    }

    pub fn with_names(component: String, action: String) -> Self {
        Self {
            connection: None,
            component_name: component,
            action_name: action,
            shortcuts: Vec::new(),
        }
    }

    async fn get_component_proxy(
        &self,
    ) -> Result<Proxy<'static>, Box<dyn std::error::Error + Send + Sync>> {
        let conn = self.connection.as_ref().ok_or("Not initialized")?;

        // The component path is /component/<componentUnique>
        let component_path = format!("/component/{}", self.component_name);

        let proxy = Proxy::new(
            conn,
            "org.kde.kglobalaccel",
            component_path,
            "org.kde.kglobalaccel.Component",
        )
        .await?;

        Ok(proxy)
    }
}

#[async_trait]
impl HotkeyBackend for KGlobalAccelBackend {
    async fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let connection = Connection::session().await?;
        self.connection = Some(Arc::new(connection));
        Ok(())
    }

    async fn register_shortcut(
        &mut self,
        shortcut: &Shortcut,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.shortcuts.push(shortcut.clone());

        // Get connection
        let conn = self.connection.as_ref().ok_or("Not initialized")?;

        // Try to programmatically register the shortcut
        // Default: Left Ctrl + Super (Meta)
        let default_shortcut = "Meta+Ctrl";

        tracing::info!(
            "Registering global shortcut for component: '{}', action: '{}'",
            self.component_name,
            self.action_name
        );

        // Create proxy to main KGlobalAccel service
        let proxy = Proxy::new(
            conn,
            "org.kde.kglobalaccel",
            "/kglobalaccel",
            "org.kde.KGlobalAccel",
        )
        .await?;

        // First, register the component and action using doRegister
        // doRegister signature: as (array of strings)
        let action_id = vec![
            self.component_name.clone(),
            self.action_name.clone(),
            "ColdVox Push-to-Talk".to_string(),
            "ColdVox".to_string(),
        ];

        match proxy.call_method("doRegister", &(action_id.clone(),)).await {
            Ok(_) => {
                tracing::debug!("Component/action registered with KGlobalAccel");
            }
            Err(e) => {
                tracing::debug!("Could not register component (may already exist): {}", e);
            }
        }

        // Now try to set default shortcut
        // setShortcut signature: asaiu -> ai
        // Parameters: action_list, key_codes, flags

        // Qt key codes for Meta+Ctrl
        // Meta modifier = 0x08000000
        // Ctrl modifier = 0x04000000
        // Combined: Meta+Ctrl (without Space)
        let key_code = vec![
            0x08000000_i32 | 0x04000000_i32, // Meta+Ctrl
        ];

        // Build the action specifier
        let action_spec = vec![
            self.component_name.clone(),
            self.action_name.clone(),
            "Push-to-Talk".to_string(),
            "ColdVox".to_string(),
        ];

        match proxy
            .call_method(
                "setShortcut",
                &(
                    action_spec, // as: action specifier
                    key_code,    // ai: key codes
                    0x3u32,      // u: flags (3 = present | active | default)
                ),
            )
            .await
        {
            Ok(_) => {
                tracing::info!(
                    "Successfully registered shortcut: {} (Left Super + Left Ctrl)",
                    default_shortcut
                );
                tracing::info!(
                    "The shortcut should now be active. Press Meta+Ctrl to activate push-to-talk."
                );
            }
            Err(e) => {
                tracing::warn!("Could not programmatically set shortcut: {}", e);
                tracing::info!(
                    "Please manually configure shortcut in KDE System Settings → Shortcuts"
                );
                tracing::info!(
                    "Component: '{}', Action: '{}', Suggested: {}",
                    self.component_name,
                    self.action_name,
                    default_shortcut
                );
            }
        }

        Ok(())
    }

    async fn start_listening(
        self: Box<Self>,
        event_tx: Sender<VadEvent>,
        status_tx: Option<Sender<BackendStatus>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let component_name = self.component_name.clone();
        let action_name = self.action_name.clone();

        // Reconnection loop with backoff
        let mut backoff = Duration::from_millis(250);
        let max_backoff = Duration::from_secs(2);
        let mut no_event_warning_shown = false;

        loop {
            match Self::listen_with_reconnect(
                &self,
                event_tx.clone(),
                status_tx.clone(),
                &component_name,
                &action_name,
                &mut no_event_warning_shown,
            )
            .await
            {
                Ok(_) => {
                    // Clean exit (should not happen in normal operation)
                    break;
                }
                Err(e) => {
                    tracing::warn!(
                        "KGlobalAccel listener error: {}, reconnecting in {:?}",
                        e,
                        backoff
                    );

                    if let Some(tx) = &status_tx {
                        let _ = tx.send(BackendStatus::Disconnected).await;
                    }

                    tokio::time::sleep(backoff).await;

                    // Exponential backoff with jitter
                    backoff = std::cmp::min(backoff * 2, max_backoff);
                    let jitter = Duration::from_millis(fastrand::u64(0..100));
                    backoff += jitter;
                }
            }
        }

        Ok(())
    }

    fn name(&self) -> &str {
        "KGlobalAccel"
    }

    async fn is_available() -> bool {
        // Check if KGlobalAccel service is available
        match Connection::session().await {
            Ok(conn) => {
                // First check if the service exists at all
                match Proxy::new(
                    &conn,
                    "org.kde.kglobalaccel",
                    "/kglobalaccel",
                    "org.freedesktop.DBus.Introspectable",
                )
                .await
                {
                    Ok(proxy) => {
                        // Try to introspect to verify it's actually available
                        proxy.introspect().await.is_ok()
                    }
                    Err(_) => false,
                }
            }
            Err(_) => false,
        }
    }
}

impl KGlobalAccelBackend {
    async fn listen_with_reconnect(
        backend: &Self,
        event_tx: Sender<VadEvent>,
        status_tx: Option<Sender<BackendStatus>>,
        component_name: &str,
        action_name: &str,
        no_event_warning_shown: &mut bool,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let proxy = backend.get_component_proxy().await?;
        let start = Instant::now();

        // Subscribe to both Pressed and Released signals
        let mut pressed_stream = proxy.receive_signal("globalShortcutPressed").await?;
        let mut released_stream = proxy.receive_signal("globalShortcutReleased").await?;

        if let Some(tx) = &status_tx {
            let _ = tx.send(BackendStatus::Connected).await;
        }

        tracing::info!(
            "KGlobalAccel listener ready for component: '{}', action: '{}'",
            component_name,
            action_name
        );

        // Track when we last saw an event
        let mut last_event_time = Instant::now();
        let warning_timeout = Duration::from_secs(10);

        // Track pressed state for debouncing
        let mut is_pressed = false;
        let mut press_timestamp_ms = 0u64;

        loop {
            // Check if we should warn about no events
            if !*no_event_warning_shown && last_event_time.elapsed() > warning_timeout {
                tracing::info!(
                    "No hotkey events received for {} seconds. Please ensure a shortcut is bound in KDE System Settings.",
                    warning_timeout.as_secs()
                );
                tracing::info!("Component: '{}', Action: '{}'", component_name, action_name);
                *no_event_warning_shown = true;
            }

            tokio::select! {
                Some(msg) = pressed_stream.next() => {
                    if let Ok((component, action, _timestamp)) =
                        msg.body().deserialize::<(String, String, i64)>()
                    {
                        if component == component_name && action == action_name {
                            last_event_time = Instant::now();
                            let ts_ms = start.elapsed().as_millis() as u64;

                            // Debounce: only send if not already pressed
                            if !is_pressed {
                                is_pressed = true;
                                press_timestamp_ms = ts_ms;  // Store press timestamp

                                tracing::debug!("Hotkey pressed: {} / {}", component, action);

                                if let Some(tx) = &status_tx {
                                    let _ = tx
                                        .send(BackendStatus::ShortcutActivated(action.clone()))
                                        .await;
                                }

                                let _ = event_tx
                                    .send(VadEvent::SpeechStart {
                                        timestamp_ms: ts_ms,
                                        energy_db: 0.0,
                                    })
                                    .await;
                            }
                        }
                    }
                }
                Some(msg) = released_stream.next() => {
                    if let Ok((component, action, _timestamp)) =
                        msg.body().deserialize::<(String, String, i64)>()
                    {
                        tracing::trace!(
                            "Received globalShortcutReleased: component='{}', action='{}'",
                            component, action
                        );

                        if component == component_name && action == action_name {
                            last_event_time = Instant::now();
                            let ts_ms = start.elapsed().as_millis() as u64;

                            if is_pressed {
                                is_pressed = false;

                                tracing::debug!("Hotkey released: {} / {}", component, action);

                                if let Some(tx) = &status_tx {
                                    let _ = tx
                                        .send(BackendStatus::ShortcutDeactivated(action.clone()))
                                        .await;
                                }

                                let _ = event_tx
                                    .send(VadEvent::SpeechEnd {
                                        timestamp_ms: ts_ms,
                                        duration_ms: ts_ms - press_timestamp_ms,  // Calculate actual duration
                                        energy_db: 0.0,
                                    })
                                    .await;
                            }
                        }
                    }
                }
                else => {
                    return Err("Signal streams ended".into());
                }
            }
        }
    }
}
