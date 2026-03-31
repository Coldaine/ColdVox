mod contract;
mod demo;
mod state;
mod window;

use std::{
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use contract::{OverlayEvent, OverlaySnapshot, OVERLAY_EVENT_NAME};
use demo::demo_script;
use state::OverlayModel;
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};

#[derive(Clone, Default)]
struct OverlayRuntime {
    inner: Arc<Mutex<OverlayModel>>,
}

impl OverlayRuntime {
    fn snapshot(&self) -> OverlaySnapshot {
        self.with_model(|model| model.snapshot())
    }

    fn with_model<R>(&self, update: impl FnOnce(&mut OverlayModel) -> R) -> R {
        let mut model = self.inner.lock().expect("overlay model poisoned");
        update(&mut model)
    }

    fn shared(&self) -> Arc<Mutex<OverlayModel>> {
        Arc::clone(&self.inner)
    }
}

type CommandResult = Result<OverlaySnapshot, String>;

fn emit_snapshot(app: &AppHandle, snapshot: &OverlaySnapshot, reason: &str) -> Result<(), String> {
    app.emit(
        OVERLAY_EVENT_NAME,
        OverlayEvent {
            reason: reason.to_string(),
            snapshot: snapshot.clone(),
        },
    )
    .map_err(|error| error.to_string())
}

fn sync_window(window: &WebviewWindow, snapshot: &OverlaySnapshot) -> Result<(), String> {
    window::sync_window(window, snapshot).map_err(|error| error.to_string())
}

fn emit_and_resize(
    app: &AppHandle,
    window: &WebviewWindow,
    snapshot: &OverlaySnapshot,
    reason: &str,
) -> CommandResult {
    sync_window(window, snapshot)?;
    emit_snapshot(app, snapshot, reason)?;
    Ok(snapshot.clone())
}

#[tauri::command]
fn get_overlay_snapshot(runtime: State<'_, OverlayRuntime>) -> OverlaySnapshot {
    runtime.snapshot()
}

#[tauri::command]
fn set_overlay_expanded(
    expanded: bool,
    runtime: State<'_, OverlayRuntime>,
    window: WebviewWindow,
    app: AppHandle,
) -> CommandResult {
    let snapshot = runtime.with_model(|model| model.set_expanded(expanded));
    emit_and_resize(
        &app,
        &window,
        &snapshot,
        if expanded { "expanded" } else { "collapsed" },
    )
}

#[tauri::command]
fn start_demo_driver(
    runtime: State<'_, OverlayRuntime>,
    window: WebviewWindow,
    app: AppHandle,
) -> CommandResult {
    let (token, snapshot) = runtime.with_model(|model| model.start_demo());
    let shared = runtime.shared();
    spawn_demo_driver(shared, app.clone(), token);
    emit_and_resize(&app, &window, &snapshot, "demo-started")
}

#[tauri::command]
fn toggle_pause_state(
    runtime: State<'_, OverlayRuntime>,
    window: WebviewWindow,
    app: AppHandle,
) -> CommandResult {
    let snapshot = runtime.with_model(|model| model.toggle_pause());
    emit_and_resize(&app, &window, &snapshot, "pause-toggled")
}

#[tauri::command]
fn stop_demo_driver(
    runtime: State<'_, OverlayRuntime>,
    window: WebviewWindow,
    app: AppHandle,
) -> CommandResult {
    let snapshot = runtime.with_model(|model| model.stop());
    emit_and_resize(&app, &window, &snapshot, "demo-stopped")
}

#[tauri::command]
fn clear_overlay_transcript(
    runtime: State<'_, OverlayRuntime>,
    window: WebviewWindow,
    app: AppHandle,
) -> CommandResult {
    let snapshot = runtime.with_model(|model| model.clear());
    emit_and_resize(&app, &window, &snapshot, "transcript-cleared")
}

#[tauri::command]
fn open_settings_placeholder(
    runtime: State<'_, OverlayRuntime>,
    window: WebviewWindow,
    app: AppHandle,
) -> CommandResult {
    let snapshot = runtime.with_model(|model| model.open_settings_placeholder());
    emit_and_resize(&app, &window, &snapshot, "settings-placeholder")
}

fn spawn_demo_driver(shared: Arc<Mutex<OverlayModel>>, app: AppHandle, token: u64) {
    thread::spawn(move || {
        for step in demo_script() {
            if !wait_for_turn(&shared, token, step.delay_ms) {
                return;
            }

            let snapshot = {
                let mut model = shared.lock().expect("overlay model poisoned");
                if model.current_demo_token() != token {
                    return;
                }
                model.apply_demo_step(&step)
            };

            if let Err(error) = emit_snapshot(&app, &snapshot, step.reason) {
                eprintln!("coldvox-gui demo emit failed: {error}");
                return;
            }
        }
    });
}

fn wait_for_turn(shared: &Arc<Mutex<OverlayModel>>, token: u64, delay_ms: u64) -> bool {
    let mut remaining_ms = delay_ms;

    while remaining_ms > 0 {
        thread::sleep(Duration::from_millis(120));

        let (current_token, paused) = {
            let model = shared.lock().expect("overlay model poisoned");
            (model.current_demo_token(), model.is_paused())
        };

        if current_token != token {
            return false;
        }

        if paused {
            continue;
        }

        remaining_ms = remaining_ms.saturating_sub(120);
    }

    true
}

pub fn run() {
    tauri::Builder::default()
        .manage(OverlayRuntime::default())
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                let runtime = app.state::<OverlayRuntime>();
                let snapshot = runtime.snapshot();

                if let Err(error) = sync_window(&window, &snapshot) {
                    eprintln!("coldvox-gui window sync failed: {error}");
                }

                if let Err(error) = window.center() {
                    eprintln!("coldvox-gui window center failed: {error}");
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_overlay_snapshot,
            set_overlay_expanded,
            start_demo_driver,
            toggle_pause_state,
            stop_demo_driver,
            clear_overlay_transcript,
            open_settings_placeholder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::contract::{OverlayEvent, OverlaySnapshot, OverlayStatus};

    #[test]
    fn overlay_event_serializes_camel_case_contract_fields() {
        let payload = OverlayEvent {
            reason: "contract-check".to_string(),
            snapshot: OverlaySnapshot {
                expanded: true,
                status: OverlayStatus::Ready,
                paused: false,
                partial_transcript: String::new(),
                final_transcript: "final transcript".to_string(),
                status_detail: "ready".to_string(),
                error_message: None,
            },
        };

        let json = serde_json::to_string(&payload).expect("serialize overlay event");

        assert!(json.contains("partialTranscript"));
        assert!(json.contains("finalTranscript"));
        assert!(json.contains("statusDetail"));
        assert!(json.contains("ready"));
    }
}
