use std::env;
use std::sync::Arc;

use tokio::signal;
use tokio::sync::{broadcast, mpsc, Mutex, RwLock};
use tokio::task::JoinHandle;
use tracing::{error, info};

use coldvox_audio::{
    AudioCaptureThread, AudioChunker, AudioRingBuffer, ChunkerConfig, FrameReader, ResamplerQuality,
};
use coldvox_foundation::AudioConfig;
use coldvox_telemetry::PipelineMetrics;
use coldvox_vad::config::SileroConfig;
use coldvox_vad::{UnifiedVadConfig, VadEvent, VadMode, FRAME_SIZE_SAMPLES, SAMPLE_RATE_HZ};

use crate::hotkey::spawn_hotkey_listener;

use crate::stt::plugin_manager::SttPluginManager;
#[cfg(feature = "vosk")]
use crate::stt::processor::PluginSttProcessor;
use coldvox_stt::plugin::PluginSelectionConfig;
use coldvox_stt::{TranscriptionConfig, TranscriptionEvent};

#[cfg(feature = "vosk")]
use crate::stt::streaming_adapter::ManagerStreamingAdapter;
#[cfg(feature = "vosk")]
use coldvox_stt::streaming_processor::StreamingSttProcessor;
use coldvox_stt::{StreamingAudioFrame, StreamingVadEvent};

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
    current_mode: std::sync::Arc<RwLock<ActivationMode>>,
    #[cfg(feature = "vosk")]
    pub stt_rx: Option<mpsc::Receiver<TranscriptionEvent>>,
    #[cfg(feature = "vosk")]
    pub plugin_manager: Option<Arc<tokio::sync::RwLock<SttPluginManager>>>,

    audio_capture: AudioCaptureThread,
    chunker_handle: JoinHandle<()>,
    trigger_handle: Arc<Mutex<JoinHandle<()>>>,
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

    /// Subscribe to raw audio frames (16kHz mono f32 samples)
    pub fn subscribe_audio(&self) -> broadcast::Receiver<coldvox_audio::AudioFrame> {
        self.audio_tx.subscribe()
    }

    /// Gracefully stop the pipeline and wait for shutdown
    pub async fn shutdown(self: Arc<Self>) {
        info!("Shutting down ColdVox runtime...");

        // Try to unwrap the Arc to get ownership
        let this = match Arc::try_unwrap(self) {
            Ok(handle) => handle,
            Err(_) => {
                error!("Cannot shutdown: AppHandle still has multiple references");
                return;
            }
        };

        // Stop audio capture first to quiesce the source
        this.audio_capture.stop();

        // Abort async tasks
        this.chunker_handle.abort();
        {
            let trigger_guard = this.trigger_handle.lock().await;
            trigger_guard.abort();
        }
        this.vad_fanout_handle.abort();
        #[cfg(feature = "vosk")]
        if let Some(h) = &this.stt_handle {
            h.abort();
        }
        #[cfg(feature = "text-injection")]
        if let Some(h) = &this.injection_handle {
            h.abort();
        }

        // Stop plugin manager tasks
        #[cfg(feature = "vosk")]
        if let Some(pm) = &this.plugin_manager {
            // Unload all plugins before stopping tasks
            let _ = pm.read().await.unload_all_plugins().await;
            let _ = pm.read().await.stop_gc_task().await;
            let _ = pm.read().await.stop_metrics_task().await;
        }

        // Await tasks to ensure clean termination
        let _ = this.chunker_handle.await;
        let trigger_handle = Arc::try_unwrap(this.trigger_handle)
            .expect("trigger_handle should have no other references")
            .into_inner();
        let _ = trigger_handle.await;
        let _ = this.vad_fanout_handle.await;
        #[cfg(feature = "vosk")]
        if let Some(h) = this.stt_handle {
            let _ = h.await;
        }
        #[cfg(feature = "text-injection")]
        if let Some(h) = this.injection_handle {
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
        &self,
        mode: ActivationMode,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut old = self.current_mode.write().await;
        if *old == mode {
            return Ok(());
        }

        info!("Switching activation mode from {:?} to {:?}", *old, mode);

        // Unload STT plugins before switching modes to ensure clean state
        #[cfg(feature = "vosk")]
        if let Some(ref pm) = self.plugin_manager {
            info!("Unloading STT plugins before activation mode switch");
            let _ = pm.read().await.unload_all_plugins().await;
        }

        {
            let trigger_guard = self.trigger_handle.lock().await;
            trigger_guard.abort();
        }
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
                        threshold: 0.1,
                        min_speech_duration_ms: 100,
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
        {
            let mut trigger_guard = self.trigger_handle.lock().await;
            *trigger_guard = new_handle;
        }
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

    let stt_arch = env::var("COLDVOX_STT_ARCH").unwrap_or_else(|_| "batch".to_string());
    info!("STT architecture: {}", stt_arch);

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
                    threshold: 0.1,
                    min_speech_duration_ms: 100,
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

    // 5) STT Plugin Manager - conditional based on architecture
    let plugin_manager: Option<Arc<tokio::sync::RwLock<SttPluginManager>>> = {
        let metrics_clone = metrics.clone();
        let create_manager = |stt_config: Option<PluginSelectionConfig>| {
            let metrics = metrics_clone.clone();
            async move {
                let mut manager = SttPluginManager::new().with_metrics_sink(metrics);
                if let Some(config) = stt_config {
                    manager.set_selection_config(config).await;
                } // else use defaults
                manager.initialize().await?;
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>(Arc::new(
                    tokio::sync::RwLock::new(manager),
                ))
            }
        };

        match stt_arch.as_str() {
            "streaming" => Some(create_manager(opts.stt_selection.clone()).await?),
            "batch" => {
                if opts.stt_selection.is_some() {
                    Some(create_manager(opts.stt_selection.clone()).await?)
                } else {
                    None
                }
            }
            _ => None,
        }
    };

    // Create transcription event channels
    #[cfg(feature = "vosk")]
    let (stt_tx, stt_rx) = mpsc::channel::<TranscriptionEvent>(100);
    #[cfg(not(feature = "vosk"))]
    let (_stt_tx, _stt_rx) = mpsc::channel::<TranscriptionEvent>(100); // stt_rx not used
    let (_text_injection_tx, text_injection_rx) = mpsc::channel::<TranscriptionEvent>(100);

    // 6) STT Processor and Fanout - branched by architecture
    #[allow(unused_variables)]
    let (stt_handle, vad_fanout_handle) = if let Some(ref _pm) = plugin_manager {
        if stt_arch == "batch" {
            // Batch path: existing PluginSttProcessor
            #[allow(unused_variables)]
            let (stt_vad_tx, stt_vad_rx) = mpsc::channel::<VadEvent>(100);
            #[allow(unused_variables)]
            let stt_audio_rx = audio_tx.subscribe();
            #[cfg(feature = "vosk")]
            let stt_config = TranscriptionConfig {
                streaming: false,
                ..Default::default()
            };
            #[cfg(not(feature = "vosk"))]
            let _stt_config = TranscriptionConfig::default();
            #[cfg(feature = "vosk")]
            let processor = PluginSttProcessor::new(
                stt_audio_rx,
                stt_vad_rx,
                stt_tx.clone(),
                plugin_manager
                    .clone()
                    .expect("Plugin manager should be initialized for batch STT path"),
                stt_config,
            );
            let stt_vad_tx_clone = stt_vad_tx.clone();
            let vad_bcast_tx_clone = vad_bcast_tx.clone();
            let vad_fanout_handle = tokio::spawn(async move {
                let mut rx = raw_vad_rx;
                while let Some(ev) = rx.recv().await {
                    let _ = vad_bcast_tx_clone.send(ev);
                    let _ = stt_vad_tx_clone.send(ev).await;
                }
            });
            #[cfg(feature = "vosk")]
            let stt_handle = Some(tokio::spawn(async move {
                processor.run().await;
            }));
            #[cfg(not(feature = "vosk"))]
            let stt_handle: Option<JoinHandle<()>> = None;
            (stt_handle, vad_fanout_handle)
        } else {
            // Streaming path
            let (stream_audio_tx, _) = broadcast::channel::<StreamingAudioFrame>(200);
            // Spawn audio forwarder task (background, not stored in handle)
            {
                let audio_rx = audio_tx.subscribe();
                let stream_audio_tx = stream_audio_tx.clone();
                let mut next_timestamp_ms = 0u64;
                tokio::spawn(async move {
                    let mut rx = audio_rx;
                    while let Ok(frame) = rx.recv().await {
                        let i16_samples: Vec<i16> = frame
                            .samples
                            .iter()
                            .map(|&s| ((s.clamp(-1.0, 1.0) * 32767.0) as i16))
                            .collect();
                        let stream_frame = StreamingAudioFrame {
                            data: i16_samples,
                            sample_rate: 16000,
                            timestamp_ms: next_timestamp_ms,
                        };
                        let _ = stream_audio_tx.send(stream_frame);
                        next_timestamp_ms += 32; // 512 samples @ 16kHz = 32ms
                    }
                });
            }
            #[allow(unused_variables)]
            let stream_audio_rx = stream_audio_tx.subscribe();
            #[allow(unused_variables)]
            let (stream_vad_tx, stream_vad_rx) = mpsc::channel::<StreamingVadEvent>(100);
            let vad_bcast_tx_clone = vad_bcast_tx.clone();
            let stream_vad_tx_clone = stream_vad_tx.clone();
            let vad_fanout_handle = tokio::spawn(async move {
                let mut rx = raw_vad_rx;
                while let Some(ev) = rx.recv().await {
                    let _ = vad_bcast_tx_clone.send(ev);
                    match ev {
                        VadEvent::SpeechStart { timestamp_ms, .. } => {
                            let stream_ev = StreamingVadEvent::SpeechStart { timestamp_ms };
                            let _ = stream_vad_tx_clone.send(stream_ev).await;
                        }
                        VadEvent::SpeechEnd {
                            timestamp_ms,
                            duration_ms,
                            ..
                        } => {
                            let stream_ev = StreamingVadEvent::SpeechEnd {
                                timestamp_ms,
                                duration_ms,
                            };
                            let _ = stream_vad_tx_clone.send(stream_ev).await;
                        }
                    }
                }
            });
            #[cfg(feature = "vosk")]
            let stt_config = TranscriptionConfig {
                streaming: true,
                ..Default::default()
            };
            #[cfg(not(feature = "vosk"))]
            let _stt_config = TranscriptionConfig {
                streaming: true,
                ..Default::default()
            };
            #[cfg(feature = "vosk")]
            let adapter = ManagerStreamingAdapter::new(
                plugin_manager.clone().expect("plugin manager missing"),
            );
            #[cfg(feature = "vosk")]
            let processor = StreamingSttProcessor::new(
                stream_audio_rx,
                stream_vad_rx,
                stt_tx.clone(),
                adapter,
                stt_config,
            );
            #[cfg(feature = "vosk")]
            let stt_handle = Some(tokio::spawn(async move {
                processor.run().await;
            }));
            #[cfg(not(feature = "vosk"))]
            let stt_handle = None;
            (stt_handle, vad_fanout_handle)
        }
    } else {
        // No STT, just fanout normally
        let vad_bcast_tx_clone = vad_bcast_tx.clone();
        let vad_fanout_handle = tokio::spawn(async move {
            let mut rx = raw_vad_rx;
            while let Some(ev) = rx.recv().await {
                tracing::debug!("Fanout: Received VAD event: {:?}", ev);
                let _ = vad_bcast_tx_clone.send(ev);
                tracing::debug!("Fanout: Forwarded to broadcast channel");
            }
        });
        #[cfg(feature = "vosk")]
        let stt_handle = None;
        #[cfg(not(feature = "vosk"))]
        let stt_handle = None;
        (stt_handle, vad_fanout_handle)
    };

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
        current_mode: std::sync::Arc::new(RwLock::new(opts.activation_mode)),
        #[cfg(feature = "vosk")]
        stt_rx: Some(stt_rx),
        #[cfg(feature = "vosk")]
        plugin_manager,
        audio_capture,
        chunker_handle,
        trigger_handle: Arc::new(Mutex::new(trigger_handle)),
        vad_fanout_handle,
        #[cfg(feature = "vosk")]
        stt_handle,
        #[cfg(feature = "text-injection")]
        injection_handle,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::time::Duration;
    use coldvox_vad::VadEvent;

    #[cfg(feature = "vosk")]
    use coldvox_stt::plugin::{FailoverConfig, GcPolicy, PluginSelectionConfig};
    #[cfg(feature = "vosk")]
    use coldvox_stt::TranscriptionEvent;

    #[cfg(feature = "vosk")]
    #[tokio::test]
    async fn end_to_end_batch_stt_pipeline() {
        env::set_var("COLDVOX_STT_ARCH", "batch");

        // Create runtime options with STT enabled
        let opts = AppRuntimeOptions {
            device: None,
            resampler_quality: ResamplerQuality::Balanced,
            activation_mode: ActivationMode::Vad,
            stt_selection: Some(PluginSelectionConfig {
                preferred_plugin: Some("mock".to_string()),
                fallback_plugins: vec!["noop".to_string()],
                require_local: true,
                max_memory_mb: None,
                required_language: None,
                failover: Some(FailoverConfig {
                    failover_threshold: 3,
                    failover_cooldown_secs: 1,
                }),
                gc_policy: Some(GcPolicy {
                    model_ttl_secs: 30,
                    enabled: false, // Disable GC for test
                }),
                metrics: None,
            }),
            #[cfg(feature = "text-injection")]
            injection: None,
        };

        // Start the app
        let mut app = start(opts).await.expect("Failed to start app");

        // Give tasks time to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Get STT receiver
        let mut stt_rx = app.stt_rx.take().expect("STT receiver should be available");

        // Send mock VAD speech start event
        let speech_start = VadEvent::SpeechStart {
            timestamp_ms: 1000,
            energy_db: -20.0,
        };
        app.raw_vad_tx
            .send(speech_start)
            .await
            .expect("Failed to send VAD event");

        // Send multiple dummy audio frames (5 frames to trigger mock transcription)
        for i in 0..5 {
            let dummy_samples = vec![0.0f32; 512];
            let audio_frame = coldvox_audio::AudioFrame {
                samples: dummy_samples,
                sample_rate: 16000,
                timestamp: std::time::Instant::now()
                    + std::time::Duration::from_millis(i as u64 * 32), // 32ms per frame
            };
            app.audio_tx
                .send(audio_frame)
                .expect("Failed to send audio frame");
            // Small delay to allow processing
            tokio::time::sleep(Duration::from_millis(1)).await;
        }

        // Send mock VAD speech end event
        let speech_end = VadEvent::SpeechEnd {
            timestamp_ms: 2000,
            duration_ms: 1000,
            energy_db: -20.0,
        };
        app.raw_vad_tx
            .send(speech_end)
            .await
            .expect("Failed to send VAD event");

        // Wait for transcription events with timeout
        let mut received_events = Vec::new();
        let timeout_duration = Duration::from_secs(5);
        let start_time = std::time::Instant::now();

        while start_time.elapsed() < timeout_duration {
            match tokio::time::timeout(Duration::from_millis(100), stt_rx.recv()).await {
                Ok(Some(event)) => {
                    received_events.push(event);
                    if received_events.len() >= 2 {
                        // Expect at least partial and final events
                        break;
                    }
                }
                Ok(None) => break,  // Channel closed
                Err(_) => continue, // Timeout, try again
            }
        }

        // Verify we received transcription events
        assert!(
            !received_events.is_empty(),
            "Should receive at least one transcription event"
        );

        // Check that we got the expected event types
        let has_partial = received_events
            .iter()
            .any(|e| matches!(e, TranscriptionEvent::Partial { .. }));
        let has_final = received_events
            .iter()
            .any(|e| matches!(e, TranscriptionEvent::Final { .. }));

        assert!(
            has_partial || has_final,
            "Should receive either partial or final transcription events"
        );

        // Clean shutdown
        Arc::new(app).shutdown().await;
    }

    #[cfg(feature = "vosk")]
    #[tokio::test]
    async fn end_to_end_streaming_stt_pipeline() {
        env::set_var("COLDVOX_STT_ARCH", "streaming");

        // Create runtime options with STT enabled (mock plugin)
        let opts = AppRuntimeOptions {
            device: None,
            resampler_quality: ResamplerQuality::Balanced,
            activation_mode: ActivationMode::Vad,
            stt_selection: Some(PluginSelectionConfig {
                preferred_plugin: Some("mock".to_string()),
                fallback_plugins: vec!["noop".to_string()],
                require_local: true,
                max_memory_mb: None,
                required_language: None,
                failover: Some(FailoverConfig {
                    failover_threshold: 3,
                    failover_cooldown_secs: 1,
                }),
                gc_policy: Some(GcPolicy {
                    model_ttl_secs: 30,
                    enabled: false, // Disable GC for test
                }),
                metrics: None,
            }),
            #[cfg(feature = "text-injection")]
            injection: None,
        };

        // Start the app
        let mut app = start(opts).await.expect("Failed to start app");

        // Give tasks time to start (longer for forwarder and streaming processor)
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Get STT receiver
        let mut stt_rx = app.stt_rx.take().expect("STT receiver should be available");

        // Send mock VAD speech start event
        let speech_start = VadEvent::SpeechStart {
            timestamp_ms: 1000,
            energy_db: -20.0,
        };
        app.raw_vad_tx
            .send(speech_start)
            .await
            .expect("Failed to send VAD event");

        // Send multiple dummy audio frames (5 frames to trigger mock transcription)
        for i in 0..5 {
            let dummy_samples = vec![0.0f32; 512];
            let audio_frame = coldvox_audio::AudioFrame {
                samples: dummy_samples,
                sample_rate: 16000,
                timestamp: std::time::Instant::now()
                    + std::time::Duration::from_millis(i as u64 * 32), // 32ms per frame
            };
            app.audio_tx
                .send(audio_frame)
                .expect("Failed to send audio frame");
            // Small delay to allow processing and forwarding
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // Send mock VAD speech end event
        let speech_end = VadEvent::SpeechEnd {
            timestamp_ms: 2000,
            duration_ms: 1000,
            energy_db: -20.0,
        };
        app.raw_vad_tx
            .send(speech_end)
            .await
            .expect("Failed to send VAD event");

        // Wait for transcription events with timeout (longer for streaming)
        let mut received_events = Vec::new();
        let timeout_duration = Duration::from_secs(10);
        let start_time = std::time::Instant::now();

        while start_time.elapsed() < timeout_duration {
            match tokio::time::timeout(Duration::from_millis(200), stt_rx.recv()).await {
                Ok(Some(event)) => {
                    received_events.push(event);
                    if received_events.len() >= 1 {
                        // At least one final event for mock
                        break;
                    }
                }
                Ok(None) => break,  // Channel closed
                Err(_) => continue, // Timeout, try again
            }
        }

        // Verify we received at least one transcription event
        assert!(
            !received_events.is_empty(),
            "Should receive at least one transcription event"
        );

        let has_final = received_events
            .iter()
            .any(|e| matches!(e, TranscriptionEvent::Final { .. }));
        assert!(
            has_final,
            "Should receive a final transcription event in streaming mode"
        );

        // Clean shutdown
        Arc::new(app).shutdown().await;
    }
}
