use coldvox_audio::ring_buffer::AudioProducer;
use coldvox_audio::SharedAudioFrame;
use std::sync::Arc;

use parking_lot::Mutex;
use tokio::signal;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio::task::JoinHandle;
use tracing::{debug, error, info};

use coldvox_audio::{
    AudioCaptureThread, AudioChunker, AudioRingBuffer, ChunkerConfig, FrameReader, ResamplerQuality,
};
use coldvox_foundation::AudioConfig;
use coldvox_stt::TranscriptionEvent;
use coldvox_telemetry::PipelineMetrics;
use coldvox_vad::config::SileroConfig;
use coldvox_vad::{UnifiedVadConfig, VadEvent, VadMode, FRAME_SIZE_SAMPLES, SAMPLE_RATE_HZ};

use crate::hotkey::spawn_hotkey_listener;
use crate::stt::plugin_manager::SttPluginManager;
#[cfg(feature = "whisper")]
use crate::stt::processor::PluginSttProcessor;
#[cfg(feature = "whisper")]
use crate::stt::session::Settings;
#[cfg(feature = "whisper")]
use coldvox_stt::TranscriptionConfig;

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
    pub allow_kdotool: bool,
    pub allow_enigo: bool,
    pub inject_on_unknown_focus: bool,
    pub max_total_latency_ms: Option<u64>,
    pub per_method_timeout_ms: Option<u64>,
    pub cooldown_initial_ms: Option<u64>,
    /// If true, exit immediately if all injection methods fail.
    pub fail_fast: bool,
}

/// Options for starting the ColdVox runtime
#[derive(Clone)]
pub struct AppRuntimeOptions {
    pub device: Option<String>,
    pub resampler_quality: ResamplerQuality,
    pub activation_mode: ActivationMode,
    /// Optional VAD configuration override (uses defaults if None)
    pub vad_config: Option<coldvox_vad::config::UnifiedVadConfig>,
    /// STT plugin selection configuration
    pub stt_selection: Option<coldvox_stt::plugin::PluginSelectionConfig>,
    #[cfg(feature = "text-injection")]
    pub injection: Option<InjectionOptions>,
    /// Whether to poll for device hotplug events (ALSA/CPAL enumeration)
    pub enable_device_monitor: bool,
    /// Capture ring buffer capacity in samples
    pub capture_buffer_samples: usize,
    pub test_device_config: Option<coldvox_audio::DeviceConfig>,
    pub test_capture_to_dummy: bool,
    pub test_injection_sink: Option<Arc<dyn crate::text_injection::TextInjector>>,
    pub transcription_config: Option<coldvox_stt::TranscriptionConfig>,
}

impl std::fmt::Debug for AppRuntimeOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppRuntimeOptions")
            .field("device", &self.device)
            .field("resampler_quality", &self.resampler_quality)
            .field("activation_mode", &self.activation_mode)
            .field("stt_selection", &self.stt_selection)
            .field("injection", &self.injection)
            .field("enable_device_monitor", &self.enable_device_monitor)
            .field("capture_buffer_samples", &self.capture_buffer_samples)
            .field("test_device_config", &self.test_device_config)
            .field("test_capture_to_dummy", &self.test_capture_to_dummy)
            .field(
                "test_injection_sink",
                &self.test_injection_sink.as_ref().map(|_| "Some(...)"),
            )
            .field("transcription_config", &self.transcription_config)
            .finish()
    }
}

impl Default for AppRuntimeOptions {
    fn default() -> Self {
        Self {
            device: None,
            resampler_quality: ResamplerQuality::Balanced,
            activation_mode: ActivationMode::Vad,
            vad_config: None, // Use VAD defaults
            stt_selection: None,
            #[cfg(feature = "text-injection")]
            injection: None,
            enable_device_monitor: false,
            capture_buffer_samples: 65_536,
            test_device_config: None,
            test_capture_to_dummy: false,
            test_injection_sink: None,
            transcription_config: None,
        }
    }
}

/// Handle to the running application pipeline
pub struct AppHandle {
    pub metrics: Arc<PipelineMetrics>,
    vad_tx: broadcast::Sender<VadEvent>,
    raw_vad_tx: mpsc::Sender<VadEvent>,
    audio_tx: broadcast::Sender<SharedAudioFrame>,
    current_mode: std::sync::Arc<RwLock<ActivationMode>>,
    #[cfg(feature = "whisper")]
    pub stt_rx: Option<mpsc::Receiver<TranscriptionEvent>>,
    #[cfg(feature = "whisper")]
    pub plugin_manager: Option<Arc<tokio::sync::RwLock<SttPluginManager>>>,

    audio_capture: AudioCaptureThread,
    pub audio_producer: Arc<Mutex<AudioProducer>>,
    chunker_handle: JoinHandle<()>,
    trigger_handle: Arc<Mutex<JoinHandle<()>>>,
    vad_fanout_handle: JoinHandle<()>,
    #[cfg(feature = "whisper")]
    stt_handle: Option<JoinHandle<()>>,
    #[cfg(feature = "whisper")]
    stt_forward_handle: Option<JoinHandle<()>>,
    #[cfg(feature = "text-injection")]
    injection_handle: Option<JoinHandle<()>>,
}

impl AppHandle {
    /// Subscribe to VAD events (multiple subscribers supported)
    pub fn subscribe_vad(&self) -> broadcast::Receiver<VadEvent> {
        self.vad_tx.subscribe()
    }

    /// Subscribe to raw audio frames (16kHz mono i16 samples via SharedAudioFrame)
    pub fn subscribe_audio(&self) -> broadcast::Receiver<SharedAudioFrame> {
        self.audio_tx.subscribe()
    }

    /// Gracefully stop the pipeline and wait for shutdown
    pub async fn shutdown(self: Arc<Self>) {
        debug!("Shutting down ColdVox runtime...");
        // Caller and runtime logs both emit at debug to reduce noisy shutdown info-level logs.

        // Try to unwrap the Arc to get ownership
        let this = match Arc::try_unwrap(self) {
            Ok(handle) => handle,
            Err(_) => {
                error!("Cannot shutdown: AppHandle still has multiple references");
                return;
            }
        };
        // Ensure tqdm is enabled to avoid buggy 'disabled_tqdm' stub in some Python envs
        std::env::set_var("TQDM_DISABLE", "0");

        // Stop audio capture first to quiesce the source
        this.audio_capture.stop();

        // Abort async tasks
        this.chunker_handle.abort();
        {
            let trigger_guard = this.trigger_handle.lock();
            trigger_guard.abort();
        }
        this.vad_fanout_handle.abort();
        #[cfg(feature = "whisper")]
        if let Some(h) = &this.stt_handle {
            h.abort();
        }
        #[cfg(feature = "whisper")]
        if let Some(h) = &this.stt_forward_handle {
            h.abort();
        }
        #[cfg(feature = "text-injection")]
        if let Some(h) = &this.injection_handle {
            h.abort();
        }

        // Stop plugin manager tasks
        #[cfg(feature = "whisper")]
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
        #[cfg(feature = "whisper")]
        if let Some(h) = this.stt_handle {
            let _ = h.await;
        }
        #[cfg(feature = "text-injection")]
        if let Some(h) = this.injection_handle {
            let _ = h.await;
        }

        debug!("ColdVox runtime shutdown complete");
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
        #[cfg(feature = "whisper")]
        if let Some(ref pm) = self.plugin_manager {
            info!("Unloading STT plugins before activation mode switch");
            let _ = pm.read().await.unload_all_plugins().await;
        }

        {
            let trigger_guard = self.trigger_handle.lock();
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
            let mut trigger_guard = self.trigger_handle.lock();
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

    info!("Starting ColdVox runtime with unified STT architecture");

    // 1) Audio capture
    let audio_config = AudioConfig {
        silence_threshold: 100,
        capture_buffer_samples: opts.capture_buffer_samples,
    };
    let ring_buffer = AudioRingBuffer::new(audio_config.capture_buffer_samples);
    let (audio_producer, audio_consumer) = ring_buffer.split();
    let audio_producer = Arc::new(Mutex::new(audio_producer));

    let (audio_capture, device_cfg, device_config_rx, _device_event_rx) = if opts
        .test_capture_to_dummy
    {
        // In test "dummy" mode, avoid opening any real audio device to prevent ALSA spam.
        // Construct a no-op capture thread and synthesize device config + channels.
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::thread;
        let shutdown = std::sync::Arc::new(AtomicBool::new(true));
        let shutdown_clone = shutdown.clone();
        let handle = thread::Builder::new()
            .name("audio-capture-dummy".to_string())
            .spawn(move || {
                while shutdown_clone.load(Ordering::Relaxed) {
                    thread::sleep(std::time::Duration::from_millis(50));
                }
            })
            .map_err(Box::<dyn std::error::Error + Send + Sync>::from)
            .unwrap();

        // Use provided test device config if available; else fall back to a sane default.
        let initial_dc = if let Some(dc) = opts.test_device_config.clone() {
            dc
        } else {
            coldvox_audio::DeviceConfig {
                sample_rate: SAMPLE_RATE_HZ,
                channels: 1,
            }
        };

        // Create broadcast channels and emit initial device config
        let (cfg_tx, cfg_rx) = tokio::sync::broadcast::channel::<coldvox_audio::DeviceConfig>(16);
        let (dev_evt_tx, dev_evt_rx) =
            tokio::sync::broadcast::channel::<coldvox_foundation::DeviceEvent>(32);
        let _ = dev_evt_tx; // not used in tests here

        // **Crucially, send the initial config immediately.**
        if let Err(e) = cfg_tx.send(initial_dc.clone()) {
            tracing::warn!("Failed to send initial dummy device config: {}", e);
        }

        // Build a dummy AudioCaptureThread
        let dummy_capture = AudioCaptureThread {
            handle,
            shutdown,
            device_monitor_handle: None,
        };

        (dummy_capture, initial_dc, cfg_rx, dev_evt_rx)
    } else {
        AudioCaptureThread::spawn(
            audio_config,
            audio_producer.clone(),
            opts.device.clone(),
            opts.enable_device_monitor,
        )?
    };

    // 2) Chunker (with resampler)
    let frame_reader = FrameReader::new(
        audio_consumer,
        device_cfg.sample_rate,
        device_cfg.channels,
        audio_config.capture_buffer_samples,
        Some(metrics.clone()),
    );
    let chunker_cfg = ChunkerConfig {
        frame_size_samples: FRAME_SIZE_SAMPLES,
        sample_rate_hz: SAMPLE_RATE_HZ,
        resampler_quality: opts.resampler_quality,
    };
    let (audio_tx, _) = broadcast::channel::<SharedAudioFrame>(200);
    // In tests, allow overriding the device config to match the injected WAV
    #[cfg(test)]
    let device_config_rx_for_chunker = if let Some(dc) = opts.test_device_config.clone() {
        let (tx, rx) = broadcast::channel::<coldvox_audio::DeviceConfig>(8);
        let _ = tx.send(dc);
        rx
    } else {
        device_config_rx.resubscribe()
    };

    #[cfg(not(test))]
    let device_config_rx_for_chunker = device_config_rx.resubscribe();

    let chunker = AudioChunker::new(frame_reader, audio_tx.clone(), chunker_cfg)
        .with_metrics(metrics.clone())
        .with_device_config(device_config_rx_for_chunker);
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
            let vad_cfg = opts.vad_config.unwrap_or(UnifiedVadConfig {
                mode: VadMode::Silero,
                frame_size_samples: FRAME_SIZE_SAMPLES,
                sample_rate_hz: SAMPLE_RATE_HZ,
                silero: SileroConfig {
                    threshold: 0.1,
                    min_speech_duration_ms: 100,
                    min_silence_duration_ms: 500,
                    window_size_samples: FRAME_SIZE_SAMPLES,
                },
            });
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
    let plugin_manager: Option<Arc<tokio::sync::RwLock<SttPluginManager>>> =
        if opts.stt_selection.is_some() {
            let metrics_clone = metrics.clone();
            let mut manager = SttPluginManager::new().with_metrics_sink(metrics_clone);
            if let Some(config) = opts.stt_selection.clone() {
                manager.set_selection_config(config).await;
            }
            // Initialize the plugin manager; enforce fail-fast semantics when no STT plugin is available
            match manager.initialize().await {
                Ok(plugin_id) => {
                    info!(
                        "STT plugin manager initialized successfully with plugin: {}",
                        plugin_id
                    );
                    Some(Arc::new(tokio::sync::RwLock::new(manager)))
                }
                Err(e) => {
                    // Fail startup when STT cannot be initialized (no-op/disabled STT not allowed)
                    error!("Failed to initialize STT plugin manager: {}", e);
                    return Err(Box::new(e));
                }
            }
        } else {
            None
        };

    // Create transcription event channels
    #[cfg(feature = "whisper")]
    let (stt_tx, stt_rx) = mpsc::channel::<TranscriptionEvent>(100);
    #[cfg(not(feature = "whisper"))]
    let (_stt_tx, _stt_rx) = mpsc::channel::<TranscriptionEvent>(100);

    // Text injection channel
    #[cfg(feature = "text-injection")]
    let (text_injection_tx, text_injection_rx) = mpsc::channel::<TranscriptionEvent>(100);
    #[cfg(not(feature = "text-injection"))]
    let (_text_injection_tx, _text_injection_rx) = mpsc::channel::<TranscriptionEvent>(100);

    // 6) STT Processor and Fanout - Unified Path
    #[cfg(feature = "whisper")]
    let mut stt_forward_handle: Option<JoinHandle<()>> = None;
    #[allow(unused_variables)]
    let (stt_handle, vad_fanout_handle) = if let Some(pm) = plugin_manager.clone() {
    // This is the single, unified path for STT processing.
    #[cfg(feature = "whisper")]
    let (session_tx, session_rx) = mpsc::channel::<SessionEvent>(100);
    let stt_audio_rx = audio_tx.subscribe();

        #[cfg(feature = "whisper")]
        let (stt_pipeline_tx, stt_pipeline_rx) = mpsc::channel::<TranscriptionEvent>(100);

        #[cfg(feature = "whisper")]
        let stt_config = opts
            .transcription_config
            .clone()
            .unwrap_or_else(|| TranscriptionConfig {
                // This `streaming` flag is now legacy. Behavior is controlled by `Settings`.
                enabled: true,
                streaming: true,
                ..Default::default()
            });

        #[cfg(feature = "whisper")]
        let processor = PluginSttProcessor::new(
            stt_audio_rx,
            session_rx,
            stt_pipeline_tx.clone(),
            pm,
            stt_config,
            Settings::default(), // Use default settings for now
        );

    let vad_bcast_tx_clone = vad_bcast_tx.clone();
    let activation_mode = opts.activation_mode;

        // This task is the new "translator" from VAD/Hotkey events to generic SessionEvents.
        let vad_fanout_handle = tokio::spawn(async move {
            let mut rx = raw_vad_rx;
            while let Some(ev) = rx.recv().await {
                // Forward the raw VAD event for UI purposes
                let _ = vad_bcast_tx_clone.send(ev);

                // Translate to SessionEvent for the STT processor (only in whisper builds)
                #[cfg(feature = "whisper")]
                {
                    let session_event = match ev {
                        VadEvent::SpeechStart { .. } => {
                            let source = match activation_mode {
                                ActivationMode::Vad => SessionSource::Vad,
                                ActivationMode::Hotkey => SessionSource::Hotkey,
                            };
                            Some(SessionEvent::Start(source, Instant::now()))
                        }
                        VadEvent::SpeechEnd { .. } => {
                            let source = match activation_mode {
                                ActivationMode::Vad => SessionSource::Vad,
                                ActivationMode::Hotkey => SessionSource::Hotkey,
                            };
                            Some(SessionEvent::End(source, Instant::now()))
                        }
                    };

                    if let Some(event) = session_event {
                        if session_tx.send(event).await.is_err() {
                            // STT processor channel closed, probably shutting down.
                            // Continue forwarding VAD events for UI rather than exiting.
                            continue;
                        }
                    }
                }
            }
        });

        #[cfg(feature = "whisper")]
        let stt_handle = Some(tokio::spawn(async move {
            processor.run().await;
        }));
        #[cfg(not(feature = "whisper"))]
        let stt_handle: Option<JoinHandle<()>> = None;

        #[cfg(feature = "whisper")]
        {
            let mut pipeline_rx = stt_pipeline_rx;
            let stt_tx_forward = stt_tx.clone();
            #[cfg(feature = "text-injection")]
            let mut text_injection_tx_forwarder = text_injection_tx.clone();
            #[cfg(feature = "text-injection")]
            let mut injection_active = true;

            // Test-only: If a mock sink is provided, spawn a task to drain events to it.
            // Note: We don't use #[cfg(test)] here because integration tests in tests/
            // need this code, and they compile the library without cfg(test).
            if let Some(mock_sink) = opts.test_injection_sink.clone() {
                let (mock_tx, mut mock_rx) = mpsc::channel::<TranscriptionEvent>(100);
                let _mock_handle = tokio::spawn(async move {
                    while let Some(event) = mock_rx.recv().await {
                        if let TranscriptionEvent::Final { text, .. } = event {
                            let _ = mock_sink.inject_text(&text, None).await;
                        }
                    }
                });
                // Overwrite the forwarder to send to our mock channel instead
                text_injection_tx_forwarder = mock_tx;
            }

            stt_forward_handle = Some(tokio::spawn(async move {
                while let Some(event) = pipeline_rx.recv().await {
                    #[cfg(feature = "text-injection")]
                    let mut injection_closed_this_event = false;

                    #[cfg(feature = "text-injection")]
                    {
                        if injection_active
                            && text_injection_tx_forwarder
                                .send(event.clone())
                                .await
                                .is_err()
                        {
                            tracing::debug!(
                                "Text injection channel closed; continuing without injection"
                            );
                            injection_closed_this_event = true;
                            injection_active = false;
                        }
                    }

                    if stt_tx_forward.send(event).await.is_err() {
                        tracing::debug!("STT receiver dropped; continuing without UI consumer");
                        #[cfg(feature = "text-injection")]
                        {
                            if !injection_active {
                                break;
                            }
                        }
                        continue;
                    }

                    #[cfg(feature = "text-injection")]
                    if injection_closed_this_event {
                        tracing::debug!("Text injection receiver unavailable; UI forward only");
                    }
                }
            }));
        }

        (stt_handle, vad_fanout_handle)
    } else {
        // No STT, just fanout VAD events for UI
        let vad_bcast_tx_clone = vad_bcast_tx.clone();
        let vad_fanout_handle = tokio::spawn(async move {
            let mut rx = raw_vad_rx;
            while let Some(ev) = rx.recv().await {
                let _ = vad_bcast_tx_clone.send(ev);
            }
        });

        #[cfg(feature = "whisper")]
        let stt_handle = None;
        #[cfg(not(feature = "whisper"))]
        let stt_handle: Option<JoinHandle<()>> = None;

        (stt_handle, vad_fanout_handle)
    };

    // Optional text-injection
    #[cfg(feature = "text-injection")]
    let injection_handle = {
        let inj_opts = opts.injection.clone();
        if let Some(inj) = inj_opts {
            if inj.enable {
                let mut config = crate::text_injection::InjectionConfig {
                    allow_kdotool: inj.allow_kdotool,
                    allow_enigo: inj.allow_enigo,
                    inject_on_unknown_focus: inj.inject_on_unknown_focus,
                    // clipboard restore is always enabled by the text-injection crate
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
                // NOTE: fail_fast is currently not a field on InjectionConfig
                // This mapping may need to be re-added once the field is available
                // config.fail_fast = inj.fail_fast
                //     || std::env::var("COLDVOX_FAIL_FAST")
                //         .map(|v| v == "1" || v.to_lowercase() == "true")
                //         .unwrap_or(false);

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
        #[cfg(feature = "whisper")]
        stt_rx: Some(stt_rx),
        #[cfg(feature = "whisper")]
        plugin_manager,
        audio_capture,
        audio_producer,
        chunker_handle,
        trigger_handle: Arc::new(Mutex::new(trigger_handle)),
        vad_fanout_handle,
        #[cfg(feature = "whisper")]
        stt_handle,
        #[cfg(feature = "whisper")]
        stt_forward_handle,
        #[cfg(feature = "text-injection")]
        injection_handle,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    
    
    use coldvox_stt::plugin::{FailoverConfig, GcPolicy, PluginSelectionConfig};
    use coldvox_stt::TranscriptionEvent;
    

    /// Helper to create default runtime options for testing.
    fn test_opts(activation_mode: ActivationMode) -> AppRuntimeOptions {
        AppRuntimeOptions {
            device: None,
            resampler_quality: ResamplerQuality::Balanced,
            activation_mode,
            vad_config: None,
            stt_selection: Some(PluginSelectionConfig {
                preferred_plugin: Some("whisper".to_string()),
                // Do not allow fallback to NoOp in tests; fail loudly if whisper unavailable
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
                    enabled: false, // Disable GC for test
                }),
                metrics: None,
                auto_extract_model: true,
            }),
            #[cfg(feature = "text-injection")]
            injection: None,
            enable_device_monitor: false,
            capture_buffer_samples: 65_536,
            test_device_config: None,
            test_capture_to_dummy: true,
            test_injection_sink: None,
            transcription_config: None,
        }
    }

    fn summarize_event(event: &TranscriptionEvent) -> String {
        match event {
            TranscriptionEvent::Partial { text, .. } => {
                format!("Partial(len={}, preview={:?})", text.len(), preview(text))
            }
            TranscriptionEvent::Final { text, .. } => {
                format!("Final(len={}, preview={:?})", text.len(), preview(text))
            }
            TranscriptionEvent::Error { code, message } => {
                format!("Error(code={code}, message={:?})", preview(message))
            }
        }
    }

    fn preview(text: &str) -> String {
        const MAX_PREVIEW: usize = 48;
        if text.len() <= MAX_PREVIEW {
            text.to_string()
        } else {
            format!("{}…", &text[..MAX_PREVIEW])
        }
    }

    #[cfg(feature = "whisper")]
    #[tokio::test]
    async fn test_unified_stt_pipeline_vad_mode() {
        // Ensure tqdm is enabled to avoid buggy 'disabled_tqdm' stub in some Python envs
        std::env::set_var("TQDM_DISABLE", "0");
        // Accelerate playback to shorten test duration
        std::env::set_var("COLDVOX_PLAYBACK_MODE", "accelerated");
        std::env::set_var("COLDVOX_PLAYBACK_SPEED_MULTIPLIER", "2.0");

        // Prepare WAV and configure device override before starting
        let mut wav_loader = WavFileLoader::new("test_data/test_11.wav").unwrap();
        let mut opts = test_opts(ActivationMode::Vad);
        opts.test_device_config = Some(DeviceConfig {
            sample_rate: wav_loader.sample_rate(),
            channels: wav_loader.channels(),
        });
        let mut app = start(opts).await.expect("Failed to start app");
        let mut stt_rx = app.stt_rx.take().expect("STT receiver should be available");

        // Give tasks time to start
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Stream WAV into ring buffer
        let audio_producer = app.audio_producer.clone();
        tokio::spawn(async move {
            wav_loader
                .stream_to_ring_buffer_locked(audio_producer)
                .await
                .unwrap();
        });

        // Simulate VAD start/end to drive session lifecycle deterministically
        tokio::time::sleep(Duration::from_millis(300)).await;
        app.raw_vad_tx
            .send(VadEvent::SpeechStart {
                timestamp_ms: 0,
                energy_db: -18.0,
            })
            .await
            .expect("Failed to send VAD SpeechStart");

        tokio::time::sleep(Duration::from_millis(1200)).await;
        app.raw_vad_tx
            .send(VadEvent::SpeechEnd {
                timestamp_ms: 1500,
                duration_ms: 1200,
                energy_db: -22.0,
            })
            .await
            .expect("Failed to send VAD SpeechEnd");

        // Wait for transcription events (expecting partial and final)
        let mut received_events = Vec::new();
        let timeout = Duration::from_secs(20);
        let wait_started = Instant::now();
        let mut final_received = false;

        while !final_received {
            match tokio::time::timeout(timeout, stt_rx.recv()).await {
                Ok(Some(event)) => {
                    if matches!(&event, TranscriptionEvent::Final { .. }) {
                        final_received = true;
                    }
                    received_events.push(event);
                }
                Ok(None) | Err(_) => {
                    let summaries: Vec<String> =
                        received_events.iter().map(summarize_event).collect();
                    panic!(
                        "Timed out after {:?} waiting for transcription events in VAD mode. Received {} events: {:?}",
                        wait_started.elapsed(),
                        summaries.len(),
                        summaries
                    );
                }
            }
        }

        assert!(!received_events.is_empty(), "Should receive events");
        // Partial events are optional depending on plugin behavior; require at least a final
        let has_partial = received_events
            .iter()
            .any(|e| matches!(e, TranscriptionEvent::Partial { .. }));
        if !has_partial {
            tracing::warn!(
                "No partial events observed; continuing as long as a final was produced"
            );
        }
        assert!(
            received_events
                .iter()
                .any(|e| matches!(e, TranscriptionEvent::Final { .. })),
            "Should receive a final event"
        );

        // Clean shutdown
        Arc::new(app).shutdown().await;
    }

    #[cfg(feature = "whisper")]
    #[tokio::test]
    async fn test_unified_stt_pipeline_hotkey_mode() {
        // Allow opting into this end-to-end test explicitly to avoid environment-specific Python/tqdm issues
        if std::env::var("COLDVOX_RUN_HOTKEY_E2E").ok().as_deref() != Some("1") {
            eprintln!("Skipping hotkey E2E test (set COLDVOX_RUN_HOTKEY_E2E=1 to run)");
            return;
        }
        // Skip in CI/headless environments to avoid brittle dependency on Python/tqdm in non-dev setups
        let is_ci = std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok();
        let headless = std::env::var("DISPLAY").is_err();
        if is_ci || headless {
            eprintln!(
                "Skipping hotkey end-to-end STT test (is_ci={}, headless={}). This test is intended for dev machines.",
                is_ci, headless
            );
            return;
        }
        // Accelerate playback to shorten test duration
        std::env::set_var("COLDVOX_PLAYBACK_MODE", "accelerated");
        std::env::set_var("COLDVOX_PLAYBACK_SPEED_MULTIPLIER", "2.0");

        // Prepare WAV and configure device override before starting
        let mut wav_loader = WavFileLoader::new("test_data/test_11.wav").unwrap();
        let mut opts = test_opts(ActivationMode::Hotkey);
        opts.test_device_config = Some(DeviceConfig {
            sample_rate: wav_loader.sample_rate(),
            channels: wav_loader.channels(),
        });
        let mut app = start(opts).await.expect("Failed to start app");
        let mut stt_rx = app.stt_rx.take().expect("STT receiver should be available");

        // Give tasks time to start
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Stream WAV into ring buffer
        let audio_producer = app.audio_producer.clone();
        tokio::spawn(async move {
            wav_loader
                .stream_to_ring_buffer_locked(audio_producer)
                .await
                .unwrap();
        });

        // Allow some audio to flow before simulating hotkey start
        tokio::time::sleep(Duration::from_millis(300)).await;

        // Simulate Hotkey Press (emits SpeechStart)
        app.raw_vad_tx
            .send(VadEvent::SpeechStart {
                timestamp_ms: 1000,
                energy_db: -20.0,
            })
            .await
            .expect("Failed to send Hotkey press event");

        // Let the system process some audio incrementally before ending
        tokio::time::sleep(Duration::from_millis(800)).await;

        // Simulate Hotkey Release (emits SpeechEnd)
        app.raw_vad_tx
            .send(VadEvent::SpeechEnd {
                timestamp_ms: 2000,
                duration_ms: 1000,
                energy_db: -20.0,
            })
            .await
            .expect("Failed to send Hotkey release event");

        // Wait for a final transcription event
        let timeout = Duration::from_secs(20);
        let wait_started = Instant::now();
        let mut received_events = Vec::new();
        let mut received_final = false;
        loop {
            match tokio::time::timeout(timeout, stt_rx.recv()).await {
                Ok(Some(event)) => {
                    if matches!(&event, TranscriptionEvent::Final { .. }) {
                        received_final = true;
                        break;
                    }
                    received_events.push(event);
                }
                Ok(None) => break,
                Err(_) => {
                    let summaries: Vec<String> =
                        received_events.iter().map(summarize_event).collect();
                    panic!(
                        "Timed out after {:?} waiting for transcription events in hotkey mode. Received {} events: {:?}",
                        wait_started.elapsed(),
                        summaries.len(),
                        summaries
                    );
                }
            }
        }

        assert!(
            received_final,
            "Should receive a final event in hotkey mode"
        );

        // Clean shutdown
        Arc::new(app).shutdown().await;
    }
}
