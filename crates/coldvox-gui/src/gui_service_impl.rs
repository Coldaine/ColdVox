//! Concrete GuiService implementation placeholder.
//! Phase 1: provide structure and compile with backend-integration gated logic.

use crate::service::{GuiService, ServiceError};

#[cfg(feature = "backend-integration")]
use crate::service::{GuiConfig, ServiceState};

#[cfg(feature = "backend-integration")]
use tokio::sync::broadcast::{self, Receiver as BroadcastReceiver, Sender as BroadcastSender};

#[cfg(feature = "backend-integration")]
use coldvox_stt::TranscriptionEvent;

#[cfg(feature = "backend-integration")]
#[derive(Default)]
pub struct GuiServiceImpl {
    state: ServiceState,
    transcript_tx: Option<BroadcastSender<TranscriptionEvent>>,
    state_tx: Option<BroadcastSender<ServiceState>>,
}

#[cfg(feature = "backend-integration")]
impl GuiServiceImpl {
    pub fn new() -> Self { Self::default() }
}

#[cfg(feature = "backend-integration")]
impl Default for ServiceState { fn default() -> Self { ServiceState::Idle } }

#[cfg(feature = "backend-integration")]
impl GuiService for GuiServiceImpl {
    fn start_recording(&mut self) -> Result<(), ServiceError> {
        self.state = ServiceState::Recording;
        if let Some(tx) = &self.state_tx { let _ = tx.send(self.state.clone()); }
        Ok(())
    }

    fn stop_recording(&mut self) -> Result<(), ServiceError> {
        self.state = ServiceState::Processing;
        if let Some(tx) = &self.state_tx { let _ = tx.send(self.state.clone()); }
        Ok(())
    }

    fn pause_recording(&mut self) -> Result<(), ServiceError> {
        self.state = ServiceState::Paused;
        if let Some(tx) = &self.state_tx { let _ = tx.send(self.state.clone()); }
        Ok(())
    }

    fn resume_recording(&mut self) -> Result<(), ServiceError> {
        self.state = ServiceState::Recording;
        if let Some(tx) = &self.state_tx { let _ = tx.send(self.state.clone()); }
        Ok(())
    }

    fn get_audio_devices(&self) -> Result<Vec<coldvox_audio::DeviceInfo>, ServiceError> {
        // Temporary stub returning empty list.
        Ok(vec![])
    }

    fn set_audio_device(&mut self, _device_id: String) -> Result<(), ServiceError> {
        Ok(())
    }

    fn get_config(&self) -> Result<GuiConfig, ServiceError> {
        Ok(GuiConfig { sample_rate: 16000, channels: 1 })
    }

    fn update_config(&mut self, _config: GuiConfig) -> Result<(), ServiceError> { Ok(()) }

    fn subscribe_transcript_updates(&mut self) -> Option<BroadcastReceiver<TranscriptionEvent>> {
        if self.transcript_tx.is_none() { let (tx, rx) = broadcast::channel(16); self.transcript_tx = Some(tx); return Some(rx); }
        self.transcript_tx.as_ref().map(|tx| tx.subscribe())
    }

    fn subscribe_state_changes(&mut self) -> Option<BroadcastReceiver<ServiceState>> {
        if self.state_tx.is_none() { let (tx, rx) = broadcast::channel(16); self.state_tx = Some(tx); return Some(rx); }
        self.state_tx.as_ref().map(|tx| tx.subscribe())
    }
}

// Non-backend builds: expose a dummy type to satisfy ServiceRegistry::create_gui_service return type.
#[cfg(not(feature = "backend-integration"))]
#[derive(Default)]
pub struct GuiServiceImpl;

#[cfg(not(feature = "backend-integration"))]
impl GuiService for GuiServiceImpl {}
