//! GUI Service Trait and foundational types.
//! Phase 1 scaffolding: minimal interface aligning with implementation plan.

#[cfg(feature = "backend-integration")]
use coldvox_audio::{DeviceConfig, DeviceInfo};
#[cfg(feature = "backend-integration")]
use coldvox_stt::{TranscriptionEvent, TranscriptionConfig};
#[cfg(feature = "backend-integration")]
use coldvox_vad::{VadEvent, VadState};
#[cfg(feature = "backend-integration")]
use coldvox_text_injection::{InjectionConfig, InjectionResult};

#[cfg(feature = "backend-integration")]
use tokio::sync::broadcast::{Receiver as BroadcastReceiver, Sender as BroadcastSender};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceState {
    Idle,
    Recording,
    Processing,
    Complete,
    Paused,
    Error,
}

#[derive(Debug, Clone)]
pub enum ServiceError {
    Audio(String),
    Stt(String),
    Vad(String),
    TextInjection(String),
    Config(String),
    Internal(String),
}

#[cfg(feature = "backend-integration")]
#[derive(Debug, Clone)]
pub struct GuiConfig {
    pub sample_rate: u32,
    pub channels: u16,
}

#[cfg(feature = "backend-integration")]
pub trait GuiService: Send + Sync {
    // Audio control
    fn start_recording(&mut self) -> Result<(), ServiceError>;
    fn stop_recording(&mut self) -> Result<(), ServiceError>;
    fn pause_recording(&mut self) -> Result<(), ServiceError>;
    fn resume_recording(&mut self) -> Result<(), ServiceError>;

    // Device management
    fn get_audio_devices(&self) -> Result<Vec<DeviceInfo>, ServiceError>;
    fn set_audio_device(&mut self, device_id: String) -> Result<(), ServiceError>;

    // Configuration
    fn get_config(&self) -> Result<GuiConfig, ServiceError>;
    fn update_config(&mut self, config: GuiConfig) -> Result<(), ServiceError>;

    // Event subscription (opaque for now)
    fn subscribe_transcript_updates(&mut self) -> Option<BroadcastReceiver<TranscriptionEvent>>;
    fn subscribe_state_changes(&mut self) -> Option<BroadcastReceiver<ServiceState>>;
}

// When backend integration feature is not enabled, provide a stub so the rest of the crate compiles.
#[cfg(not(feature = "backend-integration"))]
pub trait GuiService: Send + Sync {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_state_enum_values() {
        assert_eq!(format!("{:?}", ServiceState::Idle), "Idle");
        assert_eq!(format!("{:?}", ServiceState::Recording), "Recording");
    }

    #[test]
    fn service_error_variants() {
        let e = ServiceError::Audio("device failed".into());
        match e { ServiceError::Audio(msg) => assert!(msg.contains("device")), _ => panic!() }
    }
}
