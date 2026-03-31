use crate::contract::{OverlaySnapshot, OverlayStatus};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DemoStep {
    pub delay_ms: u64,
    pub reason: &'static str,
    pub snapshot: OverlaySnapshot,
}

pub fn demo_script() -> Vec<DemoStep> {
    vec![
        DemoStep {
            delay_ms: 280,
            reason: "demo-partial-1",
            snapshot: OverlaySnapshot {
                expanded: true,
                status: OverlayStatus::Listening,
                paused: false,
                partial_transcript: "Control seam online.".to_string(),
                final_transcript: String::new(),
                status_detail: "Streaming partial words inside the transparent host shell."
                    .to_string(),
                error_message: None,
            },
        },
        DemoStep {
            delay_ms: 520,
            reason: "demo-partial-2",
            snapshot: OverlaySnapshot {
                expanded: true,
                status: OverlayStatus::Listening,
                paused: false,
                partial_transcript:
                    "Control seam online. Streaming partials stay visible while you speak."
                        .to_string(),
                final_transcript: String::new(),
                status_detail: "Partial text remains provisional until the shell promotes it."
                    .to_string(),
                error_message: None,
            },
        },
        DemoStep {
            delay_ms: 620,
            reason: "demo-processing",
            snapshot: OverlaySnapshot {
                expanded: true,
                status: OverlayStatus::Processing,
                paused: false,
                partial_transcript:
                    "Control seam online. Streaming partials stay visible while you speak."
                        .to_string(),
                final_transcript: String::new(),
                status_detail: "Processing the utterance into a committed transcript.".to_string(),
                error_message: None,
            },
        },
        DemoStep {
            delay_ms: 760,
            reason: "demo-ready",
            snapshot: OverlaySnapshot {
                expanded: true,
                status: OverlayStatus::Ready,
                paused: false,
                partial_transcript: String::new(),
                final_transcript:
                    "Control seam online. Streaming partials stay visible while you speak."
                        .to_string(),
                status_detail:
                    "Final transcript staged. Real injection wiring lands in a later tranche."
                        .to_string(),
                error_message: None,
            },
        },
    ]
}
