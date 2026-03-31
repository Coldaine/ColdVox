use serde::{Deserialize, Serialize};

pub const OVERLAY_EVENT_NAME: &str = "coldvox://overlay";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum OverlayStatus {
    Idle,
    Listening,
    Processing,
    Ready,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OverlaySnapshot {
    pub expanded: bool,
    pub status: OverlayStatus,
    pub paused: bool,
    pub partial_transcript: String,
    pub final_transcript: String,
    pub status_detail: String,
    pub error_message: Option<String>,
}

impl Default for OverlaySnapshot {
    fn default() -> Self {
        Self {
            expanded: false,
            status: OverlayStatus::Idle,
            paused: false,
            partial_transcript: String::new(),
            final_transcript: String::new(),
            status_detail: "Overlay shell ready. Expand to inspect the seam.".to_string(),
            error_message: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OverlayEvent {
    pub reason: String,
    pub snapshot: OverlaySnapshot,
}
