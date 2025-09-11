use std::sync::Arc;

use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;

use coldvox_audio::{
    AudioCaptureThread, AudioChunker, AudioRingBuffer, ChunkerConfig, FrameReader, ResamplerQuality,
};
use coldvox_foundation::AudioConfig;
use coldvox_telemetry::PipelineMetrics;
use coldvox_vad::config::SileroConfig;
use coldvox_vad::{UnifiedVadConfig, VadEvent, VadMode, FRAME_SIZE_SAMPLES, SAMPLE_RATE_HZ};

use crate::hotkey::spawn_hotkey_listener;
use crate::stt::plugin_manager::{SttPluginManager, FailoverConfig};
use coldvox_stt::plugin::{PluginSelectionConfig};
use coldvox_stt::plugin_adapter::PluginAdapter;

#[cfg(feature = "vosk")]
use crate::stt::{processor::SttProcessor, TranscriptionEvent};
#[cfg(feature = "vosk")]
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
    
    // Legacy Vosk options (for backward compatibility)
    /// Optional model path override for Vosk; when None, uses env/defaults
    #[cfg(feature = "vosk")]
    pub vosk_model_path: Option<String>,
    /// Optional explicit enable for STT (overrides auto-detect)
    #[cfg(feature = "vosk")]
    pub stt_enabled: Option<bool>,

    // New STT plugin options
    pub stt_backend: Option<String>,
    pub stt_fallback: Vec<String>,
    pub stt_max_mem_mb: Option<u64>,
    pub stt_max_retries: usize,

    #[cfg(feature = "whisper")]
    pub whisper_model_path: Option<String>,
    #[cfg(feature = "whisper")]
    pub whisper_mode: String,
    #[cfg(feature = "whisper")]
    pub whisper_quant: String,

    #[cfg(feature = "text-injection")]
    pub injection: Option<InjectionOptions>,
}

impl Default for AppRuntimeOptions {
    fn default() -> Self {
        Self {
            device: None,
            resampler_quality: ResamplerQuality::Balanced,
            activation_mode: ActivationMode::Vad,
            
            #[cfg(feature = "vosk")]
            vosk_model_path: None,
            #[cfg(feature = "vosk")]
            stt_enabled: None,

            // New STT options with defaults
            stt_backend: Some("auto".to_string()),
            stt_fallback: vec!["whisper".to_string(), "vosk".to_string(), "mock".to_string(), "noop".to_string()],
            stt_max_mem_mb: None,
            stt_max_retries: 3,

            #[cfg(feature = "whisper")]
            whisper_model_path: None,
            #[cfg(feature = "whisper")]
            whisper_mode: "balanced".to_string(),
            #[cfg(feature = "whisper")]
            whisper_quant: "q5_1".to_string(),

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
    pub stt_rx: Option<mpsc::Receiver<TranscriptionEvent>>,

    audio_capture: AudioCaptureThread,
    chunker_handle: JoinHandle<()>,
    trigger_handle: JoinHandle<()>,
    vad_fanout_handle: JoinHandle<()>,
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
        // Stop audio capture first to quiesce the source
        self.audio_capture.stop();

        // Abort async tasks
        self.chunker_handle.abort();
        self.trigger_handle.abort();
        self.vad_fanout_handle.abort();
        if let Some(h) = &self.stt_handle {
            h.abort();
        }
        #[cfg(feature = "text-injection")]
        if let Some(h) = &self.injection_handle {
            h.abort();
        }

        // Await tasks to ensure clean termination
        let _ = self.chunker_handle.await;
        let _ = self.trigger_handle.await;
        let _ = self.vad_fanout_handle.await;
        if let Some(h) = self.stt_handle {
            let _ = h.await;
        }
        #[cfg(feature = "text-injection")]
        if let Some(h) = self.injection_handle {
            let _ = h.await;
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

    // 5) STT processor using plugin manager
    let mut stt_transcription_rx_opt: Option<mpsc::Receiver<TranscriptionEvent>> = None;
    let mut stt_handle_opt: Option<JoinHandle<()>> = None;
    let stt_vad_tx_opt: Option<mpsc::Sender<VadEvent>> = {
        // Determine if STT is enabled and create plugin manager
        let stt_enabled = should_enable_stt(&opts);

        if stt_enabled {
            match create_stt_processor(&opts, &metrics, audio_tx.subscribe()).await {
                Ok((stt_transcription_rx, stt_vad_tx, stt_handle)) => {
                    stt_transcription_rx_opt = Some(stt_transcription_rx);
                    stt_handle_opt = Some(stt_handle);
                    Some(stt_vad_tx)
                }
                Err(e) => {
                    tracing::warn!("Failed to initialize STT: {}. Continuing without STT.", e);
                    None
                }
            }
        } else {
            tracing::info!("STT disabled");
            None
        }
    };

    // Fanout task
    let vad_bcast_tx_clone = vad_bcast_tx.clone();
    let stt_vad_tx_clone = stt_vad_tx_opt.clone();
    let vad_fanout_handle = tokio::spawn(async move {
        let mut rx = raw_vad_rx;
        while let Some(ev) = rx.recv().await {
            tracing::debug!("Fanout: Received VAD event: {:?}", ev);
            let _ = vad_bcast_tx_clone.send(ev);
            tracing::debug!("Fanout: Forwarded to broadcast channel");
            if let Some(stt_tx) = &stt_vad_tx_clone {
                if let Err(e) = stt_tx.send(ev).await {
                    tracing::warn!("Fanout: Failed to send to STT: {}", e);
                } else {
                    tracing::debug!("Fanout: Forwarded to STT channel");
                }
            }
        }
    });

    // Optional text-injection
    #[cfg(feature = "text-injection")]
    let injection_handle = {
        let inj_opts = opts.injection.clone();
        if let (Some(inj), Some(stt_rx)) = (inj_opts, stt_transcription_rx_opt.take()) {
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
                    stt_rx,
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
        stt_handle_opt.is_some()
    );

    Ok(AppHandle {
        metrics,
        vad_tx: vad_bcast_tx,
        raw_vad_tx,
        audio_tx,
        current_mode: std::sync::Arc::new(parking_lot::RwLock::new(opts.activation_mode)),
        stt_rx: stt_transcription_rx_opt,
        audio_capture,
        chunker_handle,
        trigger_handle,
        vad_fanout_handle,
        stt_handle: stt_handle_opt,
        #[cfg(feature = "text-injection")]
        injection_handle,
    })
}

/// Determine if STT should be enabled based on options
fn should_enable_stt(opts: &AppRuntimeOptions) -> bool {
    // Check legacy stt_enabled option first
    #[cfg(feature = "vosk")]
    if let Some(enabled) = opts.stt_enabled {
        if enabled {
            return true;
        }
    }

    // Check if backend is explicitly disabled
    if let Some(ref backend) = opts.stt_backend {
        if backend == "none" || backend == "noop" {
            return false;
        }
    }

    // Auto-enable logic: check if any STT backend/model is available
    #[cfg(feature = "vosk")]
    {
        let vosk_model_path = opts.vosk_model_path
            .clone()
            .or_else(|| std::env::var("VOSK_MODEL_PATH").ok())
            .unwrap_or_else(|| "models/vosk-model-small-en-us-0.15".to_string());
        
        if std::path::Path::new(&vosk_model_path).exists() {
            return true;
        }
    }

    #[cfg(feature = "whisper")]
    {
        if let Some(ref path) = opts.whisper_model_path {
            if std::path::Path::new(path).exists() {
                return true;
            }
        }
        
        if std::env::var("WHISPER_MODEL_PATH").is_ok() {
            return true;
        }
    }

    // Default: try to enable (will fall back to mock/noop if nothing works)
    true
}

/// Create STT processor with plugin manager
async fn create_stt_processor(
    opts: &AppRuntimeOptions,
    metrics: &Arc<PipelineMetrics>,
    audio_rx: broadcast::Receiver<coldvox_audio::AudioFrame>,
) -> Result<(mpsc::Receiver<TranscriptionEvent>, mpsc::Sender<VadEvent>, JoinHandle<()>), String> {
    // Create plugin selection config
    let mut plugin_selection = PluginSelectionConfig::default();
    
    // Set preferred backend
    if let Some(ref backend) = opts.stt_backend {
        if backend != "auto" {
            plugin_selection.preferred_plugin = Some(backend.clone());
        }
    }

    // Handle legacy vosk-model-path flag for backward compatibility
    #[cfg(feature = "vosk")]
    if opts.vosk_model_path.is_some() && opts.stt_backend.as_deref() == Some("auto") {
        tracing::warn!("--vosk-model-path is deprecated. Use --stt-backend=vosk instead.");
        plugin_selection.preferred_plugin = Some("vosk".to_string());
    }

    // Set fallback order
    plugin_selection.fallback_plugins = opts.stt_fallback.clone();
    
    // Set constraints
    if let Some(max_mem) = opts.stt_max_mem_mb {
        plugin_selection.max_memory_mb = Some(max_mem as u32);
    }

    // Create failover config
    let failover_config = FailoverConfig {
        max_retries: opts.stt_max_retries,
        failover_cooldown_secs: 2,
        model_ttl_seconds: 300, // 5 minutes
        max_memory_mb: opts.stt_max_mem_mb,
    };

    // Create plugin manager
    let mut plugin_manager = SttPluginManager::with_config(plugin_selection, failover_config);
    plugin_manager.set_metrics(metrics.clone());
    
    // Initialize plugin manager
    let active_backend = plugin_manager.initialize().await
        .map_err(|e| format!("Failed to initialize STT plugin manager: {}", e))?;

    tracing::info!("STT initialized with backend: {}", active_backend);
    
    // Update metrics with active backend
    metrics.set_stt_backend(active_backend);

    // Create transcription config
    let stt_config = create_stt_config(opts);

    // Get the selected plugin and wrap it in an adapter  
    let plugin = plugin_manager.current_plugin_instance().await
        .ok_or_else(|| "No STT plugin selected".to_string())?;
    
    let mut adapter = PluginAdapter::new(plugin);
    adapter.initialize(stt_config.clone()).await
        .map_err(|e| format!("Failed to initialize STT plugin: {}", e))?;

    // Create channels
    let (stt_transcription_tx, stt_transcription_rx) = mpsc::channel::<TranscriptionEvent>(100);
    let (stt_vad_tx, stt_vad_rx) = mpsc::channel::<VadEvent>(200);

    // Create processor with adapter
    let stt_processor = crate::stt::processor::SttProcessor::new(
        audio_rx,
        stt_vad_rx,
        stt_transcription_tx,
        adapter,
        stt_config,
    )?;

    // Spawn processor task
    let stt_handle = tokio::spawn(async move { 
        stt_processor.run().await 
    });

    Ok((stt_transcription_rx, stt_vad_tx, stt_handle))
}

/// Create transcription configuration from options
fn create_stt_config(opts: &AppRuntimeOptions) -> TranscriptionConfig {
    // Start with default model path logic
    let default_model_path = {
        #[cfg(feature = "vosk")]
        {
            opts.vosk_model_path
                .clone()
                .or_else(|| std::env::var("VOSK_MODEL_PATH").ok())
                .unwrap_or_else(|| "models/vosk-model-small-en-us-0.15".to_string())
        }
        
        #[cfg(not(feature = "vosk"))]
        {
            "models/vosk-model-small-en-us-0.15".to_string()
        }
    };

    TranscriptionConfig {
        enabled: true,
        model_path: default_model_path,
        partial_results: true,
        max_alternatives: 1,
        include_words: false,
        buffer_size_ms: 512,
        streaming: false, // Use batch mode by default
    }
}
