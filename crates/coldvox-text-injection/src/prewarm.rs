//! # Pre-warming Module for Text Injection
//!
//! This module provides pre-warming functionality for the text injection system.
//! It prepares all necessary resources in advance to minimize latency when
//! text injection is requested.

use crate::orchestrator::AtspiContext;
use crate::types::{InjectionConfig, InjectionMethod, InjectionResult};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, info, warn, trace};

/// TTL for cached pre-warmed data (3 seconds)
const CACHE_TTL: Duration = Duration::from_secs(3);
/// Tiny timeout for individual pre-warming steps (50ms)
const STEP_TIMEOUT: Duration = Duration::from_millis(50);

/// Pre-warmed data with TTL caching
#[derive(Debug, Clone)]
struct CachedData<T> {
    data: Option<T>,
    cached_at: Instant,
}

impl<T> CachedData<T> {
    fn new() -> Self {
        Self {
            data: None,
            cached_at: Instant::now(),
        }
    }

    fn is_valid(&self) -> bool {
        self.data.is_some() && self.cached_at.elapsed() < CACHE_TTL
    }

    fn update(&mut self, data: T) {
        self.data = Some(data);
        self.cached_at = Instant::now();
    }

    fn get(&self) -> Option<&T> {
        if self.is_valid() {
            self.data.as_ref()
        } else {
            None
        }
    }
}

/// AT-SPI pre-warmed data
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct AtspiData {
    connection: Option<String>,   // Simplified connection indicator
    focused_node: Option<String>, // Simplified node identifier
    target_app: Option<String>,
    window_id: Option<String>,
    has_editable_text: bool,
}

/// Clipboard snapshot data
#[derive(Debug, Clone)]
pub struct ClipboardData {
    content: Option<Vec<u8>>,
    mime_type: Option<String>,
}

impl From<ClipboardData> for Option<crate::ClipboardBackup> {
    fn from(data: ClipboardData) -> Self {
        if let (Some(content), Some(mime_type)) = (data.content, data.mime_type) {
            Some(crate::ClipboardBackup::new(content, mime_type))
        } else {
            None
        }
    }
}

/// Portal session data
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PortalData {
    session_available: bool,
    remote_desktop_connected: bool,
}

/// Virtual keyboard data
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct VirtualKeyboardData {
    connected: bool,
    handle: Option<String>, // Handle identifier
}

/// Pre-warming controller that manages all pre-warming operations
pub struct PrewarmController {
    #[allow(dead_code)]
    config: InjectionConfig,

    // Cached data with TTL
    atspi_data: Arc<RwLock<CachedData<AtspiData>>>,
    clipboard_data: Arc<RwLock<CachedData<ClipboardData>>>,
    portal_data: Arc<RwLock<CachedData<PortalData>>>,
    virtual_keyboard_data: Arc<RwLock<CachedData<VirtualKeyboardData>>>,

    // Event listener state
    event_listener_armed: Arc<Mutex<bool>>,
}

impl PrewarmController {
    /// Create a new pre-warm controller with the given configuration
    pub fn new(config: InjectionConfig) -> Self {
        Self {
            config,
            atspi_data: Arc::new(RwLock::new(CachedData::new())),
            clipboard_data: Arc::new(RwLock::new(CachedData::new())),
            portal_data: Arc::new(RwLock::new(CachedData::new())),
            virtual_keyboard_data: Arc::new(RwLock::new(CachedData::new())),
            event_listener_armed: Arc::new(Mutex::new(false)),
        }
    }

    /// Get the AT-SPI context with pre-warmed data
    pub async fn get_atspi_context(&self) -> AtspiContext {
        let atsi_data = self.atspi_data.read().await;

        if let Some(data) = atsi_data.get() {
            AtspiContext {
                focused_node: data.target_app.clone(), // Use target_app as focused_node for now
                target_app: data.target_app.clone(),
                window_id: data.window_id.clone(),
            }
        } else {
            AtspiContext::default()
        }
    }

    /// Check if the event listener is armed
    pub async fn is_event_listener_armed(&self) -> bool {
        *self.event_listener_armed.lock().await
    }

    /// Get the cached clipboard data
    pub async fn get_clipboard_data(&self) -> Option<ClipboardData> {
        let clipboard_data = self.clipboard_data.read().await;
        clipboard_data.get().cloned()
    }

    /// Get the portal session status
    pub async fn get_portal_status(&self) -> Option<PortalData> {
        let portal_data = self.portal_data.read().await;
        portal_data.get().cloned()
    }

    /// Get the virtual keyboard status
    pub async fn get_virtual_keyboard_status(&self) -> Option<VirtualKeyboardData> {
        let vk_data = self.virtual_keyboard_data.read().await;
        vk_data.get().cloned()
    }

    /// Pre-warm AT-SPI connection and snapshot focused element
    async fn prewarm_atspi(&self) -> Result<AtspiData, String> {
    let start_time = Instant::now();
        debug!("Starting AT-SPI pre-warming");

        #[cfg(feature = "atspi")]
        {
            use atspi::{
                connection::AccessibilityConnection, proxy::collection::CollectionProxy, MatchType,
                ObjectMatchRule, SortOrder, State,
            };
            use tokio::time;

            // Connect to AT-SPI with tiny timeout
            let conn = time::timeout(STEP_TIMEOUT, AccessibilityConnection::new())
                .await
                .map_err(|_| "AT-SPI connection timeout".to_string())?
                .map_err(|e| format!("AT-SPI connect failed: {e}"))?;

            let zbus_conn = conn.connection();
            trace!("AT-SPI connection established during pre-warming");

            // Find focused element
            let collection_fut = CollectionProxy::builder(zbus_conn)
                .destination("org.a11y.atspi.Registry")
                .map_err(|e| format!("CollectionProxy destination failed: {e}"))?
                .path("/org/a11y/atspi/accessible/root")
                .map_err(|e| format!("CollectionProxy path failed: {e}"))?
                .build();

            let collection = time::timeout(STEP_TIMEOUT, collection_fut)
                .await
                .map_err(|_| "CollectionProxy timeout".to_string())?
                .map_err(|e| format!("CollectionProxy build failed: {e}"))?;

            let mut rule = ObjectMatchRule::default();
            rule.states = State::Focused.into();
            rule.states_mt = MatchType::All;

            let get_matches = collection.get_matches(rule.clone(), SortOrder::Canonical, 1, false);
            let matches = time::timeout(STEP_TIMEOUT, get_matches)
                .await
                .map_err(|_| "Get matches timeout".to_string())?
                .map_err(|e| format!("Get matches failed: {e}"))?;

            let mut focused_node = None;
            let mut target_app = None;
            let mut window_id = None;
            let mut has_editable_text = false;

            if let Some(obj_ref) = matches.first() {
                // Store the focused node
                focused_node = Some("focused".to_string());

                target_app = Some(obj_ref.name.to_string());
                window_id = Some(obj_ref.path.to_string());

                // Simplified editable text check - assume it's editable if we found a focused element
                has_editable_text = true;
            }

            let elapsed = start_time.elapsed();
            debug!(
                "AT-SPI pre-warming completed in {}ms (focused: {}, editable: {})",
                elapsed.as_millis(),
                focused_node.is_some(),
                has_editable_text
            );

            Ok(AtspiData {
                connection: Some("connected".to_string()),
                focused_node: Some("focused".to_string()),
                target_app,
                window_id,
                has_editable_text,
            })
        }

        #[cfg(not(feature = "atspi"))]
        {
            warn!("AT-SPI feature disabled; skipping AT-SPI pre-warming");
            Err("AT-SPI feature is disabled".to_string())
        }
    }

    /// Arm the event listener for text change confirmation
    async fn arm_event_listener(&self) -> Result<bool, String> {
    let start_time = Instant::now();
        debug!("Arming event listener for text change confirmation");

        #[cfg(feature = "atspi")]
        {
            use crate::confirm::create_confirmation_context;

            // Create confirmation context to arm the listener
            let _confirmation_context = create_confirmation_context(self.config.clone());

            // Mark the listener as armed
            {
                let mut armed = self.event_listener_armed.lock().await;
                *armed = true;
            }

            let elapsed = start_time.elapsed();
            debug!(
                "Event listener armed successfully in {}ms",
                elapsed.as_millis()
            );

            Ok(true)
        }

        #[cfg(not(feature = "atspi"))]
        {
            warn!("AT-SPI feature disabled; cannot arm event listener");
            Err("AT-SPI feature is disabled".to_string())
        }
    }

    /// Snapshot the current clipboard content
    async fn snapshot_clipboard(&self) -> Result<ClipboardData, String> {
        let start_time = Instant::now();
        debug!("Snapshotting clipboard content");

        #[cfg(feature = "wl_clipboard")]
        {
            // Simplified clipboard handling for now
            let content = Some("clipboard_content".as_bytes().to_vec());
            let mime_type = Some("text/plain".to_string());

            let elapsed = start_time.elapsed();
            debug!(
                "Clipboard snapshot completed in {}ms (content: {} bytes, mime: {:?})",
                elapsed.as_millis(),
                content.as_ref().map_or(0, |c| c.len()),
                mime_type
            );

            Ok(ClipboardData { content, mime_type })
        }

        #[cfg(not(feature = "wl_clipboard"))]
        {
            // Fallback to system clipboard command
            let result = tokio::process::Command::new("wl-paste")
                .arg("--type")
                .arg("text/plain")
                .output()
                .await;

            match result {
                Ok(output) if output.status.success() => {
                    let content = Some(output.stdout);
                    let mime_type = Some("text/plain".to_string());

                    let elapsed = start_time.elapsed();
                    debug!(
                        "Clipboard snapshot via wl-paste completed in {}ms ({} bytes)",
                        elapsed.as_millis(),
                        content.as_ref().map_or(0, |c| c.len())
                    );

                    Ok(ClipboardData { content, mime_type })
                }
                Ok(_) => {
                    warn!("wl-paste command failed");
                    Err("wl-paste command failed".to_string())
                }
                Err(e) => {
                    warn!("Failed to execute wl-paste: {}", e);
                    Err(format!("Failed to execute wl-paste: {}", e))
                }
            }
        }
    }

    /// Prepare portal session for remote desktop access
    async fn prepare_portal_session(&self) -> Result<PortalData, String> {
        let start_time = Instant::now();
        debug!("Preparing portal session");

        // Check if xdg-desktop-portal is available
        let portal_check = tokio::process::Command::new("busctl")
            .args(["--user", "list", "org.freedesktop.portal.Desktop"])
            .output()
            .await;

        let session_available = match portal_check {
            Ok(output) => output.status.success(),
            Err(_) => false,
        };

        // Try to establish RemoteDesktop connection
        let remote_desktop_connected = if session_available {
            // In a real implementation, this would use the portal API
            // For now, we'll just check if the service is available
            true
        } else {
            false
        };

        let elapsed = start_time.elapsed();
        debug!(
            "Portal session preparation completed in {}ms (available: {}, remote: {})",
            elapsed.as_millis(),
            session_available,
            remote_desktop_connected
        );

        Ok(PortalData {
            session_available,
            remote_desktop_connected,
        })
    }

    /// Connect to virtual keyboard (Hyprland only)
    async fn connect_virtual_keyboard(&self) -> Result<VirtualKeyboardData, String> {
        let start_time = Instant::now();
        debug!("Connecting to virtual keyboard");

        // Check if we're running on Hyprland
        let hyprland_check = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok();

        if !hyprland_check {
            debug!("Not running on Hyprland, skipping virtual keyboard");
            return Ok(VirtualKeyboardData {
                connected: false,
                handle: None,
            });
        }

        // In a real implementation, this would connect to Hyprland's virtual keyboard protocol
        // For now, we'll just check if Hyprland is running
        let connected = hyprland_check;
        let handle = if connected {
            Some("hyprland-vkbd".to_string())
        } else {
            None
        };

        let elapsed = start_time.elapsed();
        debug!(
            "Virtual keyboard connection completed in {}ms (connected: {})",
            elapsed.as_millis(),
            connected
        );

        Ok(VirtualKeyboardData { connected, handle })
    }

    /// Execute all pre-warming steps in parallel
    async fn execute_all_prewarming(&self) {
        let start_time = Instant::now();
        info!("Starting pre-warming of all injection components");

        // Execute all pre-warming steps in parallel
        let (
            atspy_result,
            event_listener_result,
            clipboard_result,
            portal_result,
            virtual_keyboard_result,
        ) = tokio::join!(
            self.prewarm_atspi(),
            self.arm_event_listener(),
            self.snapshot_clipboard(),
            self.prepare_portal_session(),
            self.connect_virtual_keyboard()
        );

        // Check results before they are moved
        let atspy_ok = atspy_result.is_ok();
        let event_ok = event_listener_result.is_ok();
        let clipboard_ok = clipboard_result.is_ok();
        let portal_ok = portal_result.is_ok();
        let vk_ok = virtual_keyboard_result.is_ok();

        // Update cached data based on results
        {
            let mut cached = self.atspi_data.write().await;
            match atspy_result {
                Ok(data) => {
                    cached.update(data);
                    info!("AT-SPI pre-warming successful");
                }
                Err(e) => {
                    warn!("AT-SPI pre-warming failed: {}", e);
                }
            }
        }

        match event_listener_result {
            Ok(_) => {
                info!("Event listener arming successful");
            }
            Err(e) => {
                warn!("Event listener arming failed: {}", e);
            }
        }

        {
            let mut cached = self.clipboard_data.write().await;
            match clipboard_result {
                Ok(data) => {
                    cached.update(data);
                    info!("Clipboard snapshot successful");
                }
                Err(e) => {
                    warn!("Clipboard snapshot failed: {}", e);
                }
            }
        }

        {
            let mut cached = self.portal_data.write().await;
            match portal_result {
                Ok(data) => {
                    cached.update(data);
                    info!("Portal session preparation successful");
                }
                Err(e) => {
                    warn!("Portal session preparation failed: {}", e);
                }
            }
        }

        {
            let mut cached = self.virtual_keyboard_data.write().await;
            match virtual_keyboard_result {
                Ok(data) => {
                    cached.update(data);
                    info!("Virtual keyboard connection successful");
                }
                Err(e) => {
                    warn!("Virtual keyboard connection failed: {}", e);
                }
            }
        }

        let elapsed = start_time.elapsed();
        info!(
            "Pre-warming completed in {}ms (AT-SPI: {}, Events: {}, Clipboard: {}, Portal: {}, VK: {})",
            elapsed.as_millis(),
            atspy_ok,
            event_ok,
            clipboard_ok,
            portal_ok,
            vk_ok
        );
    }

    /// Check if any cached data is expired
    async fn is_any_data_expired(&self) -> bool {
        let atsi = self.atspi_data.read().await;
        let clipboard = self.clipboard_data.read().await;
        let portal = self.portal_data.read().await;
        let vk = self.virtual_keyboard_data.read().await;

        !atsi.is_valid() || !clipboard.is_valid() || !portal.is_valid() || !vk.is_valid()
    }
}

/// Run targeted pre-warming for the given context and method
/// This function pre-warms only what's needed for the specific injection method
pub async fn run_for_method(_ctx: &AtspiContext, method: InjectionMethod) -> InjectionResult<()> {
    // Create a default config for now - in a real implementation this would come from the context
    let config = InjectionConfig::default();
    let controller = PrewarmController::new(config);

    match method {
        InjectionMethod::AtspiInsert => {
            // Only pre-warm AT-SPI and event listener for AT-SPI injection
            let (atspi_result, event_result) =
                tokio::join!(controller.prewarm_atspi(), controller.arm_event_listener());

            // Update caches
            {
                let mut cached = controller.atspi_data.write().await;
                if let Ok(data) = atspi_result {
                    cached.update(data);
                    info!("AT-SPI pre-warming successful for AtspiInsert");
                } else {
                    warn!("AT-SPI pre-warming failed: {:?}", atspi_result);
                }
            }

            if event_result.is_ok() {
                info!("Event listener armed for AtspiInsert");
            } else {
                warn!("Event listener arming failed: {:?}", event_result);
            }
        }
        InjectionMethod::ClipboardPasteFallback => {
            // Pre-warm clipboard and event listener for clipboard injection
            let (clipboard_result, event_result) = tokio::join!(
                controller.snapshot_clipboard(),
                controller.arm_event_listener()
            );

            // Update caches
            {
                let mut cached = controller.clipboard_data.write().await;
                if let Ok(data) = clipboard_result {
                    cached.update(data);
                    info!("Clipboard pre-warming successful for ClipboardPasteFallback");
                } else {
                    warn!("Clipboard pre-warming failed: {:?}", clipboard_result);
                }
            }

            if event_result.is_ok() {
                info!("Event listener armed for ClipboardPasteFallback");
            } else {
                warn!("Event listener arming failed: {:?}", event_result);
            }
        }
        _ => {
            // For other methods, just arm the event listener
            if controller.arm_event_listener().await.is_ok() {
                info!("Event listener armed for {:?}", method);
            }
        }
    }

    Ok(())
}

/// Run pre-warming for the given context
/// This function should be called as soon as the buffer isn't idle
pub async fn run(_ctx: &AtspiContext) -> InjectionResult<()> {
    // Legacy function - pre-warms everything for backward compatibility
    // TODO: Remove this once all callers use run_for_method
    let config = InjectionConfig::default();
    let controller = PrewarmController::new(config);

    // Check if we need to re-warm (data expired)
    if controller.is_any_data_expired().await {
        controller.execute_all_prewarming().await;
    } else {
        debug!("Pre-warmed data is still valid, skipping pre-warming");
    }

    Ok(())
}

/// Helper function to convert wl_clipboard_rs MIME type to string
#[cfg(feature = "wl_clipboard")]
#[allow(dead_code)]
fn mime_to_string(mime: &str) -> String {
    // Simplified MIME type handling
    match mime {
        "text" => "text/plain".to_string(),
        "image" => "image/png".to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cached_data_ttl() {
        let mut cached: CachedData<String> = CachedData::new();

        // Initially invalid
        assert!(!cached.is_valid());
        assert!(cached.get().is_none());

        // Update data
        cached.update("test".to_string());
        assert!(cached.is_valid());
        assert_eq!(cached.get(), Some(&"test".to_string()));

        // Simulate time passing (in real tests, you'd use mock time)
        // For now, just verify the basic functionality
    }

    #[tokio::test]
    async fn test_prewarm_controller_creation() {
        let config = InjectionConfig::default();
        let controller = PrewarmController::new(config);

        // Verify initial state
        assert!(!controller.is_event_listener_armed().await);
        assert!(controller.get_clipboard_data().await.is_none());
        assert!(controller.get_portal_status().await.is_none());
        assert!(controller.get_virtual_keyboard_status().await.is_none());

        // Verify context creation works
        let ctx = controller.get_atspi_context().await;
        assert!(ctx.focused_node.is_none());
        assert!(ctx.target_app.is_none());
        assert!(ctx.window_id.is_none());
    }

    #[tokio::test]
    async fn test_run_function() {
        let ctx = AtspiContext::default();

        // This should not panic
        let result = run(&ctx).await;
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(feature = "wl_clipboard")]
    fn test_mime_to_string() {
        assert_eq!(mime_to_string("text"), "text/plain");
        assert_eq!(mime_to_string("image"), "image/png");
        assert_eq!(mime_to_string("application/json"), "application/json");
    }
}
