use async_trait::async_trait;
use coldvox_vad::types::VadEvent;
use tokio::sync::mpsc::Sender;

/// Represents a global hotkey/shortcut
#[derive(Debug, Clone)]
pub struct Shortcut {
    pub id: String,
    pub description: String,
    pub default_keys: Option<String>, // e.g., "Ctrl+Alt+Space"
}

/// Status of the hotkey backend
#[derive(Debug, Clone)]
pub enum BackendStatus {
    Connected,
    Disconnected,
    Error(String),
    ShortcutRegistered(String),  // shortcut id
    ShortcutActivated(String),   // shortcut id
    ShortcutDeactivated(String), // shortcut id
}

/// Trait for hotkey backend implementations
#[async_trait]
pub trait HotkeyBackend: Send + Sync {
    /// Initialize the backend
    async fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Register a shortcut with the system
    async fn register_shortcut(
        &mut self,
        shortcut: &Shortcut,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Start listening for hotkey events
    async fn start_listening(
        self: Box<Self>,
        event_tx: Sender<VadEvent>,
        status_tx: Option<Sender<BackendStatus>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Get backend name for logging
    fn name(&self) -> &str;

    /// Check if this backend is available on the current system
    async fn is_available() -> bool
    where
        Self: Sized;
}

/// Detect the best available backend for the current desktop environment
pub async fn detect_best_backend() -> Box<dyn HotkeyBackend> {
    // Check for KDE Plasma first (most specific)
    #[cfg(kde_globalaccel)]
    {
        if crate::hotkey::kglobalaccel::KGlobalAccelBackend::is_available().await {
            tracing::info!("Using KDE KGlobalAccel backend");
            return Box::new(crate::hotkey::kglobalaccel::KGlobalAccelBackend::new());
        }
    }

    // Fallback to a dummy backend
    tracing::warn!("No hotkey backend available, using dummy implementation");
    Box::new(DummyBackend)
}

/// Dummy backend for systems without hotkey support
struct DummyBackend;

#[async_trait]
impl HotkeyBackend for DummyBackend {
    async fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    async fn register_shortcut(
        &mut self,
        _shortcut: &Shortcut,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    async fn start_listening(
        self: Box<Self>,
        _event_tx: Sender<VadEvent>,
        status_tx: Option<Sender<BackendStatus>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(tx) = status_tx {
            let _ = tx
                .send(BackendStatus::Error(
                    "No hotkey backend available".to_string(),
                ))
                .await;
        }
        // Sleep forever
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
        }
    }

    fn name(&self) -> &str {
        "Dummy"
    }

    async fn is_available() -> bool {
        false
    }
}
