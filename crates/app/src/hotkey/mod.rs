pub mod backend;
pub mod indicator;
#[cfg(kde_globalaccel)]
pub mod kglobalaccel;
pub mod listener;

use coldvox_vad::types::VadEvent;
use tokio::sync::mpsc::Sender;

/// Spawn a hotkey listener using the best available backend
pub fn spawn_hotkey_listener(event_tx: Sender<VadEvent>) -> tokio::task::JoinHandle<()> {
    listener::spawn_hotkey_listener(event_tx)
}
