use crate::contract::{OverlaySnapshot, OverlayStatus};
use crate::demo::DemoStep;

#[derive(Debug, Default)]
pub struct OverlayModel {
    snapshot: OverlaySnapshot,
}

impl OverlayModel {
    pub fn snapshot(&self) -> OverlaySnapshot {
        self.snapshot.clone()
    }

    pub fn set_status(&mut self, status: OverlayStatus, detail: String) -> OverlaySnapshot {
        self.snapshot.status = status;
        self.snapshot.status_detail = detail;
        self.snapshot()
    }

    pub fn is_paused(&self) -> bool {
        self.snapshot.paused
    }

    pub fn set_expanded(&mut self, expanded: bool) -> OverlaySnapshot {
        self.snapshot.expanded = expanded;

        if !expanded && self.snapshot.status == OverlayStatus::Idle {
            self.snapshot.status_detail =
                "Overlay shell ready. Expand to inspect the seam.".to_string();
        }

        self.snapshot()
    }

    pub fn update_partial(&mut self, text: String) -> OverlaySnapshot {
        self.snapshot.status = OverlayStatus::Listening;
        self.snapshot.partial_transcript = text;
        self.snapshot.status_detail = "Live transcription...".to_string();
        self.snapshot()
    }

    pub fn update_final(&mut self, text: String) -> OverlaySnapshot {
        self.snapshot.status = OverlayStatus::Ready;
        self.snapshot.partial_transcript.clear();
        self.snapshot.final_transcript = text;
        self.snapshot.status_detail = "Transcription complete.".to_string();
        self.snapshot()
    }

    pub fn reset_to_idle(&mut self, detail: String) -> OverlaySnapshot {
        self.snapshot.status = OverlayStatus::Idle;
        self.snapshot.status_detail = detail;
        self.snapshot.partial_transcript.clear();
        self.snapshot.error_message = None;
        self.snapshot()
    }

    pub fn toggle_pause(&mut self) -> OverlaySnapshot {
        if self.snapshot.status != OverlayStatus::Listening {
            return self.reject_command(
                "Pause/resume is only available during the listening demo phase.",
                "Pause is a placeholder seam until real capture wiring lands.",
            );
        }

        self.snapshot.paused = !self.snapshot.paused;
        self.snapshot.status_detail = if self.snapshot.paused {
            "Demo paused. Resume when you want the partial stream to continue.".to_string()
        } else {
            "Listening for provisional words from the demo driver.".to_string()
        };
        self.snapshot.error_message = None;
        self.snapshot()
    }

    pub fn stop(&mut self) -> OverlaySnapshot {
        if self.snapshot.status != OverlayStatus::Listening
            && self.snapshot.status != OverlayStatus::Processing
        {
            return self.reject_command(
                "Nothing is active to stop.",
                "Stop only applies while the demo is simulating live capture.",
            );
        }

        self.demo_token += 1;
        self.snapshot.status = OverlayStatus::Idle;
        self.snapshot.paused = false;
        self.snapshot.partial_transcript.clear();
        self.snapshot.status_detail =
            "Capture stopped. Expand again or rerun the demo when ready.".to_string();
        self.snapshot.error_message = None;
        self.snapshot()
    }

    pub fn clear(&mut self) -> OverlaySnapshot {
        self.demo_token += 1;
        let expanded = self.snapshot.expanded;
        self.snapshot = OverlaySnapshot {
            expanded,
            status_detail: "Transcript cleared. Demo seam remains ready.".to_string(),
            ..OverlaySnapshot::default()
        };
        self.snapshot()
    }

    pub fn open_settings_placeholder(&mut self) -> OverlaySnapshot {
        self.snapshot.expanded = true;
        self.snapshot.status_detail =
            "Settings stay out of scope in this tranche, but the command seam is in place."
                .to_string();
        if self.snapshot.status != OverlayStatus::Error {
            self.snapshot.error_message = None;
        }
        self.snapshot()
    }

    fn reject_command(&mut self, message: &str, detail: &str) -> OverlaySnapshot {
        self.demo_token += 1;
        self.snapshot.expanded = true;
        self.snapshot.status = OverlayStatus::Error;
        self.snapshot.paused = false;
        self.snapshot.status_detail = detail.to_string();
        self.snapshot.error_message = Some(message.to_string());
        self.snapshot()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::demo::demo_script;

    #[test]
    fn starts_idle_and_collapsed() {
        let model = OverlayModel::default();

        assert_eq!(model.snapshot.status, OverlayStatus::Idle);
        assert!(!model.snapshot.expanded);
        assert!(model.snapshot.partial_transcript.is_empty());
        assert!(model.snapshot.final_transcript.is_empty());
    }

    #[test]
    fn stop_from_idle_surfaces_error_state() {
        let mut model = OverlayModel::default();
        let snapshot = model.stop();

        assert_eq!(snapshot.status, OverlayStatus::Error);
        assert_eq!(
            snapshot.error_message.as_deref(),
            Some("Nothing is active to stop."),
        );
    }

    #[test]
    fn demo_script_reaches_ready_state_with_final_text() {
        let mut model = OverlayModel::default();
        let (_token, start_snapshot) = model.start_demo();

        assert_eq!(start_snapshot.status, OverlayStatus::Listening);

        let mut last_snapshot = start_snapshot;
        for step in demo_script() {
            last_snapshot = model.apply_demo_step(&step);
        }

        assert_eq!(last_snapshot.status, OverlayStatus::Ready);
        assert!(last_snapshot.partial_transcript.is_empty());
        assert!(last_snapshot
            .final_transcript
            .contains("Streaming partials"));
    }

    #[test]
    fn pause_round_trip_keeps_demo_in_listening_state() {
        let mut model = OverlayModel::default();
        model.start_demo();

        let paused = model.toggle_pause();
        assert_eq!(paused.status, OverlayStatus::Listening);
        assert!(paused.paused);

        let resumed = model.toggle_pause();
        assert_eq!(resumed.status, OverlayStatus::Listening);
        assert!(!resumed.paused);
    }
}
