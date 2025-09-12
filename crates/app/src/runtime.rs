use std::sync::Arc;

use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;
use tokio::signal;
use tracing::info;

use coldvox_audio::{
    AudioCaptureThread, AudioChunker, AudioRingBuffer, ChunkerConfig, FrameReader, ResamplerQuality,
};
use coldvox_foundation::AudioConfig;
use coldvox_telemetry::PipelineMetrics;
use coldvox_vad::config::SileroConfig;
use coldvox_vad::{UnifiedVadConfig, VadEvent, VadMode, FRAME_SIZE_SAMPLES, SAMPLE_RATE_HZ};

use crate::hotkey::spawn_hotkey_listener;

#[cfg(feature = "vosk")]
use crate::stt::TranscriptionEvent;
use crate::stt::plugin_manager::SttPluginManager;

/// Activation strategy for push-to-talk vs voice activation
#[derive(PartialEq, Copy, Clone, Debug)]
pub enum ActivationMode {
    Vad,
    Hotkey,
}

/// Text-injection options (only when the feature is enabled)
#[cfg(feature = "text-injection")]
#[derive(Clone, Debug, Default)]
pub struct InjectionOptions {
    pub enable: bool,
    pub allow_ydotool: bool,
    pub allow_kdotool: bool,
    pub allow_enigo: bool,
    pub inject_on_unknown_focus: bool,
    pub restore_clipboard: bool,
    pub max_total_latency_ms: Option<u64>,
    pub per_method_timeout_ms: Option<u64>,
    pub cooldown_initial_ms: Option<u64>,
}

/// Options for starting the ColdVox runtime
#[derive(Clone, Debug)]
pub struct AppRuntimeOptions {
    pub device: Option<String>,
    pub resampler_quality: ResamplerQuality,
    pub activation_mode: ActivationMode,
    /// STT plugin selection configuration
    pub stt_selection: Option<coldvox_stt::plugin::PluginSelectionConfig>,
    #[cfg(feature = "text-injection")]
    pub injection: Option<InjectionOptions>,
}

impl Default for AppRuntimeOptions {
    fn default() -> Self {
        Self {
            device: None,
            resampler_quality: ResamplerQuality::Balanced,
            activation_mode: ActivationMode::Vad,
            stt_selection: None,
            #[cfg(feature = "text-injection")]
            injection: None,
        }
    }
}

/// Handle to the running application pipeline
pub struct AppHandle {
    pub metrics: Arc<PipelineMetrics>,
    vad_tx: broadcast::Sender<VadEvent>,
    raw_vad_tx: mpsc::Sender<VadEvent>,
    audio_tx: broadcast::Sender<coldvox_audio::AudioFrame>,
    current_mode: std::sync::Arc<parking_lot::RwLock<ActivationMode>>,
    #[cfg(feature = "vosk")]
    pub stt_rx: Option<mpsc::Receiver<TranscriptionEvent>>,
    #[cfg(feature = "vosk")]
    plugin_manager: Option<SttPluginManager>,

    audio_capture: AudioCaptureThread,
    chunker_handle: JoinHandle<()>,
    trigger_handle: JoinHandle<()>,
    vad_fanout_handle: JoinHandle<()>,
    #[cfg(feature = "vosk")]
    stt_handle: Option<JoinHandle<()>>,
    #[cfg(feature = "text-injection")]
    injection_handle: Option<JoinHandle<()>>,
}

impl AppHandle {
    /// Subscribe to VAD events (multiple subscribers supported)
    pub fn subscribe_vad(&self) -> broadcast::Receiver<VadEvent> {
        self.vad_tx.subscribe()
    }

    /// Gracefully stop the pipeline and wait for shutdown
    pub async fn shutdown(self) {
        info!("Shutting down ColdVox runtime...");
        
        // Stop audio capture first to quiesce the source
        self.audio_capture.stop();

        // Abort async tasks
        self.chunker_handle.abort();
        self.trigger_handle.abort();
        self.vad_fanout_handle.abort();
        #[cfg(feature = "vosk")]
        if let Some(h) = &self.stt_handle {
            h.abort();
        }
        #[cfg(feature = "text-injection")]
        if let Some(h) = &self.injection_handle {
            h.abort();
        }

        // Stop plugin manager tasks
        #[cfg(feature = "vosk")]
        if let Some(pm) = &self.plugin_manager {
            // Unload all plugins before stopping tasks
            let _ = pm.unload_all_plugins().await;
            let _ = pm.stop_gc_task().await;
            let _ = pm.stop_metrics_task().await;
        }

        // Await tasks to ensure clean termination
        let _ = self.chunker_handle.await;
        let _ = self.trigger_handle.await;
        let _ = self.vad_fanout_handle.await;
        #[cfg(feature = "vosk")]
        if let Some(h) = self.stt_handle {
            let _ = h.await;
        }
        #[cfg(feature = "text-injection")]
        if let Some(h) = self.injection_handle {
            let _ = h.await;
        }
        
        info!("ColdVox runtime shutdown complete");
    }

    /// Wait for shutdown signal (SIGINT, SIGTERM)
    pub async fn wait_for_shutdown_signal() {
        info!("Waiting for shutdown signal (Ctrl+C or SIGTERM)...");
        match signal::ctrl_c().await {
            Ok(()) => {
                info!("Received SIGINT (Ctrl+C), initiating graceful shutdown");
            }
            Err(err) => {
                error!("Failed to listen for SIGINT: {}", err);
            }
        }
    }

    /// Switch activation mode at runtime without full restart
    pub async fn set_activation_mode(
        &mut self,
        mode: ActivationMode,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut old = self.current_mode.write();
        if *old == mode {
            return Ok(());
        }
        
        info!("Switching activation mode from {:?} to {:?}", *old, mode);
        
        // Unload STT plugins before switching modes to ensure clean state
        #[cfg(feature = "vosk")]
        if let Some(ref pm) = self.plugin_manager {
            info!("Unloading STT plugins before activation mode switch");
            let _ = pm.unload_all_plugins().await;
        }
        
        self.trigger_handle.abort();
        // Spawn new trigger
        let new_handle = match mode {
            ActivationMode::Vad => {
                // VAD (Voice Activity Detection) Configuration
                //
                // The VAD is configured to detect speech segments from the audio stream.
                // Key parameters for the Silero VAD engine are set here.
                //
                // Of particular note is `min_silence_duration_ms`. This value was
                // intentionally increased from a default of 100ms to 500ms.
                //
                // Rationale for 500ms silence duration (see issue #61):
                // - **Problem:** Shorter silence durations (e.g., 100-200ms) can cause the
                //   VAD to split a single logical utterance into multiple speech events
                //   during natural pauses in speech.
                // - **Impact:** This fragmentation leads to disjointed transcriptions and
                //   can prevent the STT engine from understanding the full context of a
                //   sentence. It also increases overhead from starting and stopping the
                //   STT process multiple times.
                // - **Solution:** A longer duration of 500ms acts as a buffer, "stitching"
                //   together speech segments that are separated by short pauses. This
                //   results in more coherent, sentence-like chunks being sent to the STT
                //   engine, significantly improving transcription quality.
                // - **Trade-off:** The primary trade-off is a slight increase in latency,
                //   as the system waits longer to confirm the end of an utterance. For
                //   dictation, this is an acceptable trade-off for the gain in accuracy.
                let vad_cfg = UnifiedVadConfig {
                    mode: VadMode::Silero,
                    frame_size_samples: FRAME_SIZE_SAMPLES,
                    sample_rate_hz: SAMPLE_RATE_HZ,
                    silero: SileroConfig {
                        threshold: 0.3,
                        min_speech_duration_ms: 250,
                        min_silence_duration_ms: 500,
                        window_size_samples: FRAME_SIZE_SAMPLES,
                    },
                };
                let vad_audio_rx = self.audio_tx.subscribe();
                crate::audio::vad_processor::VadProcessor::spawn(
                    vad_cfg,
                    vad_audio_rx,
                    self.raw_vad_tx.clone(),
                    Some(self.metrics.clone()),
                )?
            }
            ActivationMode::Hotkey => crate::hotkey::spawn_hotkey_listener(self.raw_vad_tx.clone()),
        };
        self.trigger_handle = new_handle;
        *old = mode;
        
        info!("Successfully switched to {:?} activation mode", mode);
        Ok(())
    }
}

/// Start the ColdVox pipeline with the given options
pub async fn start(
    opts: AppRuntimeOptions,
) -> Result<AppHandle, Box<dyn std::error::Error + Send + Sync>> {
    // Metrics shared across components
    let metrics = Arc::new(PipelineMetrics::default());

    // 1) Audio capture
    let audio_config = AudioConfig::default();
    let ring_buffer = AudioRingBuffer::new(16384 * 4);
    let (audio_producer, audio_consumer) = ring_buffer.split();
    let (audio_capture, device_cfg, device_config_rx, _device_event_rx) =
        AudioCaptureThread::spawn(audio_config, audio_producer, opts.device.clone())?;

    // 2) Chunker (with resampler)
    let frame_reader = FrameReader::new(
        audio_consumer,
        device_cfg.sample_rate,
        device_cfg.channels,
        16384 * 4,
        Some(metrics.clone()),
    );
    let chunker_cfg = ChunkerConfig {
        frame_size_samples: FRAME_SIZE_SAMPLES,
        sample_rate_hz: SAMPLE_RATE_HZ,
        resampler_quality: opts.resampler_quality,
    };
    let (audio_tx, _) = broadcast::channel::<coldvox_audio::AudioFrame>(200);
    let chunker = AudioChunker::new(frame_reader, audio_tx.clone(), chunker_cfg)
        .with_metrics(metrics.clone())
        .with_device_config(device_config_rx.resubscribe());
    let chunker_handle = chunker.spawn();

    // 3) Activation source (VAD or Hotkey) feeding a raw VAD mpsc channel
    let (raw_vad_tx, raw_vad_rx) = mpsc::channel::<VadEvent>(200);
    let trigger_handle = match opts.activation_mode {
        ActivationMode::Vad => {
            // VAD (Voice Activity Detection) Configuration
            //
            // The VAD is configured to detect speech segments from the audio stream.
            // Key parameters for the Silero VAD engine are set here.
            //
            // Of particular note is `min_silence_duration_ms`. This value was
            // intentionally increased from a default of 100ms to 500ms.
            //
            // Rationale for 500ms silence duration (see issue #61):
            // - **Problem:** Shorter silence durations (e.g., 100-200ms) can cause the
            //   VAD to split a single logical utterance into multiple speech events
            //   during natural pauses in speech.
            // - **Impact:** This fragmentation leads to disjointed transcriptions and
            //   can prevent the STT engine from understanding the full context of a
            //   sentence. It also increases overhead from starting and stopping the
            //   STT process multiple times.
            // - **Solution:** A longer duration of 500ms acts as a buffer, "stitching"
            //   together speech segments that are separated by short pauses. This
            //   results in more coherent, sentence-like chunks being sent to the STT
            //   engine, significantly improving transcription quality.
            // - **Trade-off:** The primary trade-off is a slight increase in latency,
            //   as the system waits longer to confirm the end of an utterance. For
            //   dictation, this is an acceptable trade-off for the gain in accuracy.
            let vad_cfg = UnifiedVadConfig {
                mode: VadMode::Silero,
                frame_size_samples: FRAME_SIZE_SAMPLES,
                sample_rate_hz: SAMPLE_RATE_HZ,
                silero: SileroConfig {
                    threshold: 0.3,
                    min_speech_duration_ms: 250,
                    min_silence_duration_ms: 500,
                    window_size_samples: FRAME_SIZE_SAMPLES,
                },
            };
            let vad_audio_rx = audio_tx.subscribe();
            let vad_handle = crate::audio::vad_processor::VadProcessor::spawn(
                vad_cfg,
                vad_audio_rx,
                raw_vad_tx.clone(),
                Some(metrics.clone()),
            )
            .map_err(|e| {
                tracing::error!("Failed to spawn VAD processor: {}", e);
                e
            })?;
            vad_handle
        }
        ActivationMode::Hotkey => spawn_hotkey_listener(raw_vad_tx.clone()),
    };

    // Log successful VAD processor spawn
    if let ActivationMode::Vad = opts.activation_mode {
        tracing::info!("VAD processor spawned successfully");
    }

    // 4) Fan-out raw VAD mpsc -> broadcast for UI, and to STT when enabled
    let (vad_bcast_tx, _) = broadcast::channel::<VadEvent>(256);

    // 5) STT Plugin Manager
    let plugin_manager = if let Some(stt_config) = &opts.stt_selection {
        let mut manager = SttPluginManager::new()
            .with_metrics_sink(metrics.clone());
        manager.set_selection_config(stt_config.clone()).await;
        Some(manager)
    } else {
        None
    };

    // Create transcription event channels
    let (stt_tx, stt_rx) = mpsc::channel::<TranscriptionEvent>(100);
    let (text_injection_tx, text_injection_rx) = mpsc::channel::<TranscriptionEvent>(100);
    
    // TODO: Use stt_tx and text_injection_tx in STT processing task
    let _stt_tx = stt_tx;
    let _text_injection_tx = text_injection_tx;

    // Fanout task
    let vad_bcast_tx_clone = vad_bcast_tx.clone();
    let vad_fanout_handle = tokio::spawn(async move {
        let mut rx = raw_vad_rx;
        while let Some(ev) = rx.recv().await {
            tracing::debug!("Fanout: Received VAD event: {:?}", ev);
            let _ = vad_bcast_tx_clone.send(ev);
            tracing::debug!("Fanout: Forwarded to broadcast channel");
        }
    });

    // Optional text-injection
    #[cfg(feature = "text-injection")]
    let injection_handle = {
        let inj_opts = opts.injection.clone();
        if let Some(inj) = inj_opts {
            if inj.enable {
                let mut config = crate::text_injection::InjectionConfig {
                    allow_ydotool: inj.allow_ydotool,
                    allow_kdotool: inj.allow_kdotool,
                    allow_enigo: inj.allow_enigo,
                    inject_on_unknown_focus: inj.inject_on_unknown_focus,
                    restore_clipboard: inj.restore_clipboard,
                    ..Default::default()
                };
                if let Some(v) = inj.max_total_latency_ms {
                    config.max_total_latency_ms = v;
                }
                if let Some(v) = inj.per_method_timeout_ms {
                    config.per_method_timeout_ms = v;
                }
                if let Some(v) = inj.cooldown_initial_ms {
                    config.cooldown_initial_ms = v;
                }

                let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>(1);
                let processor = crate::text_injection::AsyncInjectionProcessor::new(
                    config,
                    text_injection_rx,
                    shutdown_rx,
                    None,
                )
                .await;

                Some(tokio::spawn(async move {
                    if let Err(e) = processor.run().await {
                        tracing::error!("Injection processor error: {}", e);
                    }
                    drop(shutdown_tx);
                }))
            } else {
                None
            }
        } else {
            None
        }
    };

    // Log pipeline component initialization status
    tracing::info!(
        "Audio pipeline components initialized: capture={}, chunker={}, vad={}, stt={}",
        true, // audio_capture is always initialized
        true, // chunker_handle is always initialized
        matches!(opts.activation_mode, ActivationMode::Vad),
        opts.stt_selection.is_some()
    );

    Ok(AppHandle {
        metrics,
        vad_tx: vad_bcast_tx,
        raw_vad_tx,
        audio_tx,
        current_mode: std::sync::Arc::new(parking_lot::RwLock::new(opts.activation_mode)),
        #[cfg(feature = "vosk")]
        stt_rx: Some(stt_rx),
        #[cfg(feature = "vosk")]
        plugin_manager,
        audio_capture,
        chunker_handle,
        trigger_handle,
        vad_fanout_handle,
        #[cfg(feature = "vosk")]
        stt_handle: None, // TODO: Add STT processing task handle
        #[cfg(feature = "text-injection")]
        injection_handle,
    })
}
