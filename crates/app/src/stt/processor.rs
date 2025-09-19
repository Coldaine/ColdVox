// ---
// Unified STT Processor
//
// This file contains the unified STT processor for ColdVox. It replaces the
// previous dual-architecture system (legacy batch vs. streaming) with a single,
// plugin-based processor.
//
// Key Design Principles:
// - Single `run` loop using `tokio::select!` for handling multiple event sources.
// - Abstracted session lifecycle via `SessionEvent` (from VAD or Hotkey).
// - Non-blocking finalization: When an utterance ends, a background `tokio::task`
//   is spawned to handle the potentially slow process of finalizing the
//   transcription with the STT plugin. This prevents the main loop from blocking
//   and dropping audio frames from a subsequent utterance.
// - State management via a `parking_lot::Mutex` to allow safe concurrent access
//   from the main loop and spawned tasks.
// ---

use crate::stt::{
    session::{HotkeyBehavior, SessionEvent, Settings},
    TranscriptionConfig, TranscriptionEvent,
};
use coldvox_audio::{SharedAudioFrame, chunker::AudioFrame};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{broadcast, mpsc};

/// Represents the current state of the STT processor's utterance handling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UtteranceState {
    /// Waiting for an utterance to begin.
    Idle,
    /// An utterance is actively being processed (either streaming or buffering).
    SpeechActive,
    /// The last utterance is being finalized. New audio is ignored until this completes.
    Finalizing,
}

/// A snapshot of performance and activity metrics for the STT processor.
#[derive(Debug, Clone, Default)]
pub struct SttMetrics {
    pub frames_in: u64,
    pub partial_count: u64,
    pub final_count: u64,
    pub error_count: u64,
    pub last_event_time: Option<Instant>,
}

// A safety guard to prevent the audio buffer from growing indefinitely in batch mode.
// 30 seconds of 16kHz 16-bit mono audio.
const BUFFER_CEILING_SAMPLES: usize = 16000 * 30;

/// The primary STT processor, designed to be unified and extensible.
/// It uses the plugin manager to delegate STT work and handles different
/// activation and processing strategies defined by `Settings`.
#[cfg(feature = "vosk")]
pub struct PluginSttProcessor {
    audio_rx: broadcast::Receiver<SharedAudioFrame>,
    session_event_rx: mpsc::Receiver<SessionEvent>,
    event_tx: mpsc::Sender<TranscriptionEvent>,
    plugin_manager: Arc<tokio::sync::RwLock<crate::stt::plugin_manager::SttPluginManager>>,
    state: Arc<parking_lot::Mutex<State>>,
    metrics: Arc<parking_lot::RwLock<SttMetrics>>,
    config: TranscriptionConfig,
    settings: Settings,
}

/// The internal, mutable state of the processor, protected by a Mutex.
#[cfg(feature = "vosk")]
struct State {
    pub state: UtteranceState,
    pub source: crate::stt::session::SessionSource,
    pub buffer: Vec<Arc<[i16]>>,
    pub local_frame_count: u64,
}

#[cfg(feature = "vosk")]
impl PluginSttProcessor {
    fn record_frame_in(&self) {
        let mut state = self.state.lock();
        state.local_frame_count += 1;
        if state.local_frame_count % 10 == 0 {
            let mut metrics = self.metrics.write();
            metrics.frames_in += 10;
        }
    }
    /// Creates a new instance of the unified STT processor.
    pub fn new(
        audio_rx: broadcast::Receiver<SharedAudioFrame>,
        session_event_rx: mpsc::Receiver<SessionEvent>,
        event_tx: mpsc::Sender<TranscriptionEvent>,
        plugin_manager: Arc<tokio::sync::RwLock<crate::stt::plugin_manager::SttPluginManager>>,
        config: TranscriptionConfig,
        settings: Settings,
    ) -> Self {
        let internal_state = State {
            state: UtteranceState::Idle,
            source: crate::stt::session::SessionSource::Vad, // Default
            buffer: Vec::with_capacity(10),
            local_frame_count: 0,
        };

        Self {
            audio_rx,
            session_event_rx,
            event_tx,
            plugin_manager,
            state: Arc::new(parking_lot::Mutex::new(internal_state)),
            metrics: Arc::new(parking_lot::RwLock::new(SttMetrics::default())),
            config,
            settings,
        }
    }

    /// The main run loop for the processor. It uses `tokio::select!` to concurrently
    /// listen for session lifecycle events and incoming audio frames.
    pub async fn run(mut self) {
        tracing::info!(
            target: "stt",
            "Unified STT processor starting (behavior: {:?}, partials: {})",
            self.settings.hotkey_behavior,
            self.config.partial_results,
        );

        // Clear any stale audio frames from the channel before starting.
        while self.audio_rx.try_recv().is_ok() {}

        // Ensure the active plugin is initialized with the desired transcription config
        {
            let mut pm = self.plugin_manager.write().await;
            if let Err(e) = pm.apply_transcription_config(self.config.clone()).await {
                tracing::warn!(target: "stt", "Failed to apply transcription config to plugin: {}", e);
            }
        }

        loop {
            tokio::select! {
                Some(event) = self.session_event_rx.recv() => {
                    self.handle_session_event(event).await;
                }
                Ok(frame) = self.audio_rx.recv() => {
                    self.handle_audio_frame(frame).await;
                }
                else => {
                    tracing::info!(target: "stt", "Unified STT processor shutting down");
                    break;
                }
            }
        }
    }

    /// Handles session lifecycle events (Start, End, Abort).
    async fn handle_session_event(&self, event: SessionEvent) {
        let handoff_start = Instant::now();

        let mut state = self.state.lock();
        match event {
            SessionEvent::Start(source, _instant) => {
                if state.state == UtteranceState::Idle {
                    tracing::info!(target: "stt", "Session started via {:?}", source);
                    state.source = source;
                    state.state = UtteranceState::SpeechActive;
                    state.buffer.clear();
                    let pm = self.plugin_manager.clone();
                    tokio::spawn(async move {
                        if let Err(e) = pm.write().await.begin_utterance().await {
                            tracing::error!(target: "stt", "Plugin begin_utterance failed: {}", e);
                        }
                    });
                }
            }
            SessionEvent::End(source, _instant) => {
                self.handle_session_end(source, false, &mut state);
            }
            SessionEvent::Abort(source, reason) => {
                tracing::warn!(target: "stt", "Session aborted from {:?}: {}", source, reason);
                self.handle_session_end(source, true, &mut state);
            }
        }

    let _handoff_latency_ms = handoff_start.elapsed().as_millis() as u64;
        // Assume pipeline_metrics is available, update max
        // Note: Add pipeline_metrics: Arc<PipelineMetrics> to PluginSttProcessor if not present, but since it's in runtime, pass via new or global
        // For now, assume it's added; update
        // self.pipeline_metrics.update_vad_to_stt_handoff_latency(handoff_latency_ms); // If added
    }

    /// Handles the end of an utterance. This is a critical path that spawns a
    /// non-blocking task to finalize the transcription, ensuring the main loop
    /// can immediately start processing the next utterance.
    fn handle_session_end(
        &self,
        _source: crate::stt::session::SessionSource,
        is_abort: bool,
        state: &mut parking_lot::MutexGuard<'_, State>,
    ) {
        if state.state != UtteranceState::SpeechActive {
            return;
        }

        // Flush batched metrics
        let mut metrics = self.metrics.write();
        metrics.frames_in += state.local_frame_count % 10;
        state.local_frame_count = 0;

        if is_abort {
            state.state = UtteranceState::Idle;
            state.buffer.clear();
            let pm = self.plugin_manager.clone();
            tokio::spawn(async move {
                if let Err(e) = pm.write().await.cancel_utterance().await {
                    tracing::error!(target: "stt", "Plugin cancel_utterance failed: {}", e);
                }
            });
            return;
        }

        state.state = UtteranceState::Finalizing;

        let pm = self.plugin_manager.clone();
        let event_tx = self.event_tx.clone();
        let metrics_clone = self.metrics.clone();
        let behavior = self.settings.hotkey_behavior.clone();
        let buffer_arcs = state.buffer.drain(..).collect::<Vec<_>>();
        let state_arc = self.state.clone();

        tokio::spawn(async move {
            // For batch, concat Arcs to Vec<i16> once
            let _audio_buffer = if behavior != HotkeyBehavior::Incremental && !buffer_arcs.is_empty() {
                let mut full_buffer = Vec::with_capacity(512 * buffer_arcs.len());
                for arc in buffer_arcs {
                    full_buffer.extend_from_slice(&*arc);
                }
                if let Err(e) = pm.write().await.process_audio(&full_buffer).await {
                    tracing::error!(target: "stt", "Plugin batch processing error: {}", e);
                }
                Some(full_buffer)
            } else {
                None
            };

            let finalize_result = pm.write().await.finalize().await;

            match finalize_result {
                Ok(Some(event)) => {
                    Self::send_event_static(&event_tx, &metrics_clone, event).await;
                }
                Ok(None) => {}
                Err(e) => {
                    let err_event = TranscriptionEvent::Error {
                        code: "FINALIZE_FAILED".to_string(),
                        message: e,
                    };
                    Self::send_event_static(&event_tx, &metrics_clone, err_event).await;
                }
            }

            let mut final_state = state_arc.lock();
            final_state.state = UtteranceState::Idle;
            final_state.buffer.clear();
        });
    }

    /// Handles an incoming chunk of audio frames.
    async fn handle_audio_frame(&self, frame: SharedAudioFrame) {
        let behavior = self.settings.hotkey_behavior.clone();
        let i16_slice = &*frame.samples;

        if behavior != HotkeyBehavior::Incremental {
            // Batch mode: collect Arc, no copy
            let mut state = self.state.lock();
            if state.state == UtteranceState::SpeechActive {
                state.buffer.push(Arc::clone(&frame.samples));
                state.local_frame_count += 1;
                if state.local_frame_count % 10 == 0 {
                    let mut metrics = self.metrics.write();
                    metrics.frames_in += 10;
                }
                if state.buffer.len() > 300 {
                    tracing::warn!(target: "stt", "Audio frame ceiling reached. Defensively finalizing.");
                    self.handle_session_end(state.source, false, &mut state);
                }
            }
        } else {
            // Incremental mode
            let should_process = {
                let state = self.state.lock();
                state.state == UtteranceState::SpeechActive
            };

            if should_process {
                self.record_frame_in();

                tracing::debug!(target: "stt", "Dispatching {} samples to plugin.process_audio()", i16_slice.len());
                match self.plugin_manager.write().await.process_audio(i16_slice).await {
                    Ok(Some(event)) => {
                        Self::send_event_static(&self.event_tx, &self.metrics, event).await;
                    }
                    Ok(None) => {}
                    Err(e) => {
                        let err_event = TranscriptionEvent::Error {
                            code: "PLUGIN_PROCESS_ERROR".to_string(),
                            message: e,
                        };
                        Self::send_event_static(&self.event_tx, &self.metrics, err_event).await;
                    }
                }
            }
        }
    }

    /// A static helper to send transcription events and update metrics, callable
    /// from spawned tasks.
    async fn send_event_static(
        event_tx: &mpsc::Sender<TranscriptionEvent>,
        metrics_arc: &Arc<parking_lot::RwLock<SttMetrics>>,
        event: TranscriptionEvent,
    ) {
        {
            let mut metrics = metrics_arc.write();
            match &event {
                TranscriptionEvent::Partial { .. } => metrics.partial_count += 1,
                TranscriptionEvent::Final { .. } => metrics.final_count += 1,
                TranscriptionEvent::Error { .. } => metrics.error_count += 1,
            }
            metrics.last_event_time = Some(Instant::now());
        }

        if event_tx.send(event).await.is_err() {
            tracing::warn!(target: "stt", "Failed to send transcription event: channel closed");
        }
    }
}

/// A stub implementation of the processor for when the `vosk` feature is disabled.
#[cfg(not(feature = "vosk"))]
pub struct PluginSttProcessor;

#[cfg(not(feature = "vosk"))]
impl PluginSttProcessor {
    pub fn new(
        _audio_rx: broadcast::Receiver<SharedAudioFrame>,
        _session_event_rx: mpsc::Receiver<SessionEvent>,
        _event_tx: mpsc::Sender<TranscriptionEvent>,
        _plugin_manager: Arc<tokio::sync::RwLock<crate::stt::plugin_manager::SttPluginManager>>,
        _config: TranscriptionConfig,
        _settings: Settings,
    ) -> Self {
        Self
    }
    pub async fn run(self) {
        tracing::info!("STT processor stub running - no actual processing (Vosk feature disabled)");
    }
}
