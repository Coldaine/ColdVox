use super::indicator::RecordingIndicator;
use coldvox_vad::types::VadEvent;
use device_query::{DeviceQuery, DeviceState, Keycode};
use std::time::{Duration, Instant};
use tokio::sync::mpsc::Sender;

/// Spawn a blocking task that listens for Ctrl+Super key combinations
/// and emits synthetic `VadEvent`s to control the STT pipeline.
/// While the hotkey is active, a small terminal widget is displayed.
pub fn spawn_hotkey_listener(event_tx: Sender<VadEvent>) -> tokio::task::JoinHandle<()> {
    tokio::task::spawn_blocking(move || {
        let device_state = DeviceState::new();
        let mut indicator = RecordingIndicator::new();
        let mut active = false;
        let mut start = Instant::now();
        let app_start = Instant::now();
        loop {
            let keys = device_state.get_keys();
            let ctrl = keys.contains(&Keycode::LControl) || keys.contains(&Keycode::RControl);
            let meta = keys.contains(&Keycode::LMeta) || keys.contains(&Keycode::RMeta);
            if ctrl && meta {
                if !active {
                    active = true;
                    start = Instant::now();
                    indicator.show();
                    let ts = app_start.elapsed().as_millis() as u64;
                    let _ = event_tx.blocking_send(VadEvent::SpeechStart {
                        timestamp_ms: ts,
                        energy_db: 0.0,
                    });
                }
            } else if active {
                active = false;
                indicator.hide();
                let duration = start.elapsed().as_millis() as u64;
                let ts = app_start.elapsed().as_millis() as u64;
                let _ = event_tx.blocking_send(VadEvent::SpeechEnd {
                    timestamp_ms: ts,
                    duration_ms: duration,
                    energy_db: 0.0,
                });
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    })
}
