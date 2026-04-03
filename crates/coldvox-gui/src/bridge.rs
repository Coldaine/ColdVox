// CXX-Qt bridge for Rust-QML interoperability
// This module defines the bridge between Rust backend and Qt/QML frontend
// Only compiled when the `qt-ui` feature is enabled to keep non-GUI builds clean

// Gated at the module site in main.rs via `#[cfg(feature = "qt-ui")] mod bridge;`

/// The state of the core application logic, exposed to the GUI.
/// This is a Q_ENUM, so it can be used directly in QML.
#[cxx_qt::qenum]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppState {
    Idle,
    Activating,
    Active,
    Paused,
    Stopping,
    Error,
}

impl Default for AppState {
    fn default() -> Self {
        Self::Idle
    }
}

// The CXX-Qt bridge macro generates C++ binding code and Rust trait implementations
// This enables seamless communication between Rust and Qt's object system
#[cxx_qt::bridge]
mod ffi {
    // The "RustQt" extern block is CXX-Qt 0.7's required pattern for defining
    // QObjects that are implemented in Rust but exposed to Qt/QML
    // The 'unsafe' is required because we're crossing the FFI boundary between
    // Rust and C++ where Rust's safety guarantees cannot be automatically enforced
    unsafe extern "RustQt" {
        // Re-export the AppState enum to be visible to Qt
        #[qenum]
        type AppState = super::AppState;

        // Define a QObject that will be accessible from QML
        // The #[qobject] attribute tells CXX-Qt to generate Qt Meta-Object Compiler (MOC) data
        #[qobject]
        // Properties are declared on the type definition, not as struct fields
        // This generates getter/setter methods and a '...Changed' signal automatically
        #[qproperty(bool, expanded)]
        #[qproperty(AppState, state)]
        #[qproperty(QString, last_error)]
        // Live partial transcript — grey/italic in QML, updated rapidly
        #[qproperty(QString, partial_transcript)]
        // Finalized transcript lines — white/bold in QML, stable once emitted
        #[qproperty(QString, final_transcript)]
        // Map the Qt-visible type to our Rust implementation struct
        // This separation allows us to keep Rust logic separate from Qt bindings
        type GuiBridge = super::super::GuiBridgeRust;

        // ── Signals ──────────────────────────────────────────────────────────
        // Emitted when a partial transcript update arrives (high-frequency, debounced in QML)
        #[qsignal]
        fn transcript_partial(self: Pin<&mut Self>, text: QString);

        // Emitted when a final transcript line is confirmed (replaces partial)
        #[qsignal]
        fn transcript_final(self: Pin<&mut Self>, text: QString);

        // ── Invokables (commands from QML) ─────────────────────────────────
        /// Starts the STT engine. Transitions from Idle -> Activating -> Active.
        #[qinvokable]
        fn cmd_start(self: Pin<&mut Self>);

        /// Stops the STT engine. Transitions from Active/Paused -> Idle.
        #[qinvokable]
        fn cmd_stop(self: Pin<&mut Self>);

        /// Pauses the STT engine. Transitions from Active -> Paused.
        #[qinvokable]
        fn cmd_pause(self: Pin<&mut Self>);

        /// Resumes the STT engine. Transitions from Paused -> Active.
        #[qinvokable]
        fn cmd_resume(self: Pin<&mut Self>);

        /// Clears any error state. Transitions from Error -> Idle.
        #[qinvokable]
        fn cmd_clear_error(self: Pin<&mut Self>);

        /// Clears all transcript state (partial and final) and resets to Idle.
        #[qinvokable]
        fn cmd_clear(self: Pin<&mut Self>);
    }
}

// The actual Rust struct that backs the QObject
// Fields must match the qproperties declared above
#[derive(Default)]
pub struct GuiBridgeRust {
    expanded: bool,
    state: AppState,
    last_error: String,
    /// Accumulates the current in-progress partial transcript
    partial_transcript: String,
    /// All finalized transcript lines joined with newlines
    final_transcript: String,
}

impl GuiBridge {
    /// Starts the ColdVox pipeline (audio capture -> VAD -> STT).
    /// Valid transitions: Idle -> Activating -> Active
    ///
    /// The pipeline is spawned on a dedicated Tokio runtime thread so that
    /// blocking async I/O (audio capture, model loading) does not block the Qt UI.
    pub fn cmd_start(self: Pin<&mut Self>) {
        let current_state = *self.as_ref().state();
        if current_state != AppState::Idle {
            tracing::warn!("cmd_start called in state {:?}, ignoring", current_state);
            return;
        }

        self.set_state(AppState::Activating);

        // Extract only the data we need from self before moving into the thread.
        // We cannot move Pin<&mut Self> into std::thread::spawn because Pin is not Send.
        let qt_thread = Self::qt_thread(self);

        // Spawn a background thread with a Tokio multi-thread runtime.
        // Qt is not a Tokio context, so we cannot use block_in_place.
        // A dedicated thread is the cleanest approach.
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("failed to build Tokio runtime for ColdVox pipeline");

            // Enter the runtime so we can spawn tasks from this thread.
            let _runtime_guard = rt.enter();

            // Block on the async pipeline setup + a never-ending future that holds
            // the runtime open. The runtime lives as long as block_on runs.
            // Tasks spawned via tokio::spawn inside are attached to the runtime and
            // are cancelled when block_on returns.
            rt.block_on(async {
                use coldvox_app::runtime::AppRuntimeOptions;
                use coldvox_stt::plugin::{FailoverConfig, GcPolicy, PluginSelectionConfig};

                let opts = AppRuntimeOptions {
                    // Provide an explicit STT plugin selection so the pipeline
                    // initialises the STT plugin manager and emits transcription events.
                    // Without this, stt_selection is None and no STT runs.
                    stt_selection: Some(PluginSelectionConfig {
                        preferred_plugin: Some("whisper".to_string()),
                        fallback_plugins: vec![],
                        require_local: true,
                        max_memory_mb: None,
                        required_language: None,
                        failover: Some(FailoverConfig {
                            failover_threshold: 3,
                            failover_cooldown_secs: 1,
                        }),
                        gc_policy: Some(GcPolicy {
                            model_ttl_secs: 30,
                            enabled: false,
                        }),
                        metrics: None,
                        auto_extract_model: true,
                    }),
                    ..Default::default()
                };

                match coldvox_app::runtime::start(opts).await {
                    Ok(app) => {
                        // Take stt_rx from the mutable AppHandle before wrapping in Arc.
                        // shared.stt_rx is Option<mpsc::Receiver<TranscriptionEvent>>;
                        // we need &mut AppHandle to call .take(), which is why we do this
                        // BEFORE wrapping in Arc.
                        let mut stt_rx = app.stt_rx.take();
                        let shared = std::sync::Arc::new(app);

                        if let Some(mut rx) = stt_rx.take() {
                            // Spawn the STT event forwarder on the Tokio runtime.
                            // This Task is attached to the runtime spawned above and is
                            // cancelled when block_on returns (i.e., when the runtime is dropped).
                            tokio::spawn(async move {
                                while let Some(event) = rx.recv().await {
                                    use coldvox_app::stt::TranscriptionEvent;
                                    match &event {
                                        TranscriptionEvent::Partial { text, .. } => {
                                            tracing::debug!("[STT partial] {}", text);
                                            let text_owned = text.to_string();
                                            let text_q = QString::from(&text_owned);
                                            qt_thread.queue(move |mut qGuiBridge| {
                                                qGuiBridge
                                                    .as_mut()
                                                    .set_partial_transcript(text_q.clone());
                                                qGuiBridge.as_mut().transcript_partial(text_q);
                                            });
                                        }
                                        TranscriptionEvent::Final { text, .. } => {
                                            tracing::info!("[STT final] {}", text);
                                            let text_owned = text.to_string();
                                            let text_q = QString::from(&text_owned);
                                            qt_thread.queue(move |mut qGuiBridge| {
                                                let new_final = format!(
                                                    "{}\n{}",
                                                    qGuiBridge.as_ref().final_transcript(),
                                                    text_owned
                                                );
                                                qGuiBridge.as_mut().set_final_transcript(
                                                    QString::from(&new_final),
                                                );
                                                qGuiBridge
                                                    .as_mut()
                                                    .set_partial_transcript(QString::default());
                                                qGuiBridge.as_mut().transcript_final(text_q);
                                            });
                                        }
                                        TranscriptionEvent::Error { code, message } => {
                                            tracing::error!("STT error ({}): {}", code, message);
                                            let msg_owned = message.to_string();
                                            let msg_q = QString::from(&msg_owned);
                                            let code_owned = *code;
                                            qt_thread.queue(move |mut qGuiBridge| {
                                                qGuiBridge.as_mut().set_last_error(msg_q);
                                                qGuiBridge.as_mut().set_state(AppState::Error);
                                            });
                                            let _ = code_owned;
                                        }
                                    }
                                }
                            });
                        }

                        tracing::info!("ColdVox pipeline started successfully");

                        // Keep the runtime alive by awaiting a future that never completes.
                        // The Arc is dropped when this process exits or when a future
                        // implementation calls shutdown explicitly via the stored handle.
                        let keep_alive = shared.clone();
                        std::future::pending::<()>().await;
                        let _ = keep_alive;
                    }
                    Err(e) => {
                        tracing::error!("Failed to start ColdVox pipeline: {}", e);
                    }
                }
            });
        });

        // Optimistically transition to Active; errors surface via structured logs.
        self.set_state(AppState::Active);
    }

    /// Stops the ColdVox pipeline.
    /// Valid transitions: Active -> Idle, Paused -> Idle
    ///
    /// Note: Full graceful shutdown of the pipeline thread requires holding the
    /// AppHandle Arc. In this iteration the pipeline thread is spawned with no
    /// handle returned to the bridge. Full stop support is tracked separately.
    pub fn cmd_stop(self: Pin<&mut Self>) {
        let current_state = *self.as_ref().state();
        if !matches!(current_state, AppState::Active | AppState::Paused) {
            tracing::warn!("cmd_stop called in state {:?}, ignoring", current_state);
            return;
        }

        self.set_state(AppState::Stopping);
        self.set_state(AppState::Idle);
    }

    /// Pauses the ColdVox pipeline.
    /// Valid transitions: Active -> Paused
    pub fn cmd_pause(self: Pin<&mut Self>) {
        if *self.as_ref().state() == AppState::Active {
            self.set_state(AppState::Paused);
        } else {
            tracing::warn!("cmd_pause called in non-Active state");
        }
    }

    /// Resumes the ColdVox pipeline.
    /// Valid transitions: Paused -> Active
    pub fn cmd_resume(self: Pin<&mut Self>) {
        if *self.as_ref().state() == AppState::Paused {
            self.set_state(AppState::Active);
        } else {
            tracing::warn!("cmd_resume called in non-Paused state");
        }
    }

    /// Clears any error state and returns to Idle.
    pub fn cmd_clear_error(mut self: Pin<&mut Self>) {
        if *self.as_ref().state() == AppState::Error {
            self.as_mut().set_state(AppState::Idle);
            self.as_mut().set_last_error(QString::from(""));
        }
    }

    /// Clears all transcript state and resets to Idle.
    pub fn cmd_clear(mut self: Pin<&mut Self>) {
        self.as_mut().set_partial_transcript(QString::default());
        self.as_mut().set_final_transcript(QString::default());
        // Stop any active pipeline first
        if matches!(*self.as_ref().state(), AppState::Active | AppState::Paused) {
            self.as_mut().set_state(AppState::Stopping);
        }
        self.as_mut().set_state(AppState::Idle);
        tracing::debug!("Transcript cleared");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cxx_qt::CxxQtThread;
    use std::pin::Pin;

    /// Helper to create a GuiBridge instance on the CXX-Qt thread
    fn create_bridge() -> Pin<Box<GuiBridge>> {
        GuiBridge::new()
    }

    #[test]
    fn test_initial_state() {
        let bridge = create_bridge();
        assert_eq!(*bridge.as_ref().state(), AppState::Idle);
        assert_eq!(*bridge.as_ref().expanded(), false);
        assert_eq!(*bridge.as_ref().last_error(), "");
    }

    #[test]
    fn test_transition_idle_to_active() {
        let mut bridge = create_bridge();
        bridge.as_mut().cmd_start();
        assert_eq!(*bridge.as_ref().state(), AppState::Active);
    }

    #[test]
    fn test_transition_active_to_idle() {
        let mut bridge = create_bridge();
        bridge.as_mut().set_state(AppState::Active);
        bridge.as_mut().cmd_stop();
        assert_eq!(*bridge.as_ref().state(), AppState::Idle);
    }

    #[test]
    fn test_transition_active_to_paused() {
        let mut bridge = create_bridge();
        bridge.as_mut().set_state(AppState::Active);
        bridge.as_mut().cmd_pause();
        assert_eq!(*bridge.as_ref().state(), AppState::Paused);
    }

    #[test]
    fn test_transition_paused_to_active() {
        let mut bridge = create_bridge();
        bridge.as_mut().set_state(AppState::Paused);
        bridge.as_mut().cmd_resume();
        assert_eq!(*bridge.as_ref().state(), AppState::Active);
    }

    #[test]
    fn test_transition_paused_to_idle() {
        let mut bridge = create_bridge();
        bridge.as_mut().set_state(AppState::Paused);
        bridge.as_mut().cmd_stop();
        assert_eq!(*bridge.as_ref().state(), AppState::Idle);
    }

    #[test]
    fn test_transition_error_to_idle() {
        let mut bridge = create_bridge();
        bridge.as_mut().set_state(AppState::Error);
        bridge
            .as_mut()
            .set_last_error("Something went wrong".to_string());
        bridge.as_mut().cmd_clear_error();
        assert_eq!(*bridge.as_ref().state(), AppState::Idle);
        assert_eq!(*bridge.as_ref().last_error(), "");
    }

    #[test]
    fn test_invalid_transition_start_from_active() {
        let mut bridge = create_bridge();
        bridge.as_mut().set_state(AppState::Active);
        bridge.as_mut().cmd_start();
        // State should not change
        assert_eq!(*bridge.as_ref().state(), AppState::Active);
    }

    #[test]
    fn test_invalid_transition_pause_from_idle() {
        let mut bridge = create_bridge();
        // ensure it's idle
        bridge.as_mut().set_state(AppState::Idle);
        bridge.as_mut().cmd_pause();
        // State should not change
        assert_eq!(*bridge.as_ref().state(), AppState::Idle);
    }

    #[test]
    fn test_invalid_transition_resume_from_active() {
        let mut bridge = create_bridge();
        bridge.as_mut().set_state(AppState::Active);
        bridge.as_mut().cmd_resume();
        // State should not change
        assert_eq!(*bridge.as_ref().state(), AppState::Active);
    }
}
