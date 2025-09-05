use std::sync::Arc;

use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;

use coldvox_audio::{
    AudioCaptureThread, AudioChunker, AudioRingBuffer, ChunkerConfig, FrameReader, ResamplerQuality,
};
use coldvox_foundation::AudioConfig;
use coldvox_telemetry::PipelineMetrics;
use coldvox_vad::{UnifiedVadConfig, VadEvent, VadMode, FRAME_SIZE_SAMPLES, SAMPLE_RATE_HZ};

use crate::hotkey::spawn_hotkey_listener;

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
    /// Optional model path override for Vosk; when None, uses env/defaults
    #[cfg(feature = "vosk")]
    pub vosk_model_path: Option<String>,
    /// Optional explicit enable for STT (overrides auto-detect)
    #[cfg(feature = "vosk")]
    pub stt_enabled: Option<bool>,
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
                let vad_cfg = UnifiedVadConfig {
                    mode: VadMode::Silero,
                    frame_size_samples: FRAME_SIZE_SAMPLES,
                    sample_rate_hz: SAMPLE_RATE_HZ,
                    ..Default::default()
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
    let (audio_capture, device_cfg, device_config_rx) =
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
            let vad_cfg = UnifiedVadConfig {
                mode: VadMode::Silero,
                frame_size_samples: FRAME_SIZE_SAMPLES,
                sample_rate_hz: SAMPLE_RATE_HZ,
                ..Default::default()
            };
            let vad_audio_rx = audio_tx.subscribe();
            crate::audio::vad_processor::VadProcessor::spawn(
                vad_cfg,
                vad_audio_rx,
                raw_vad_tx.clone(),
                Some(metrics.clone()),
            )?
        }
        ActivationMode::Hotkey => spawn_hotkey_listener(raw_vad_tx.clone()),
    };

    // 4) Fan-out raw VAD mpsc -> broadcast for UI, and to STT when enabled
    let (vad_bcast_tx, _) = broadcast::channel::<VadEvent>(256);

    // 5) Optional STT processor
    #[cfg(feature = "vosk")]
    let mut stt_transcription_rx_opt: Option<mpsc::Receiver<TranscriptionEvent>> = None;
    #[cfg(feature = "vosk")]
    let mut stt_handle_opt: Option<JoinHandle<()>> = None;
    #[cfg(feature = "vosk")]
    let stt_vad_tx_opt: Option<mpsc::Sender<VadEvent>> = {
        // Determine model path and whether STT is enabled
        let model_path = if let Some(p) = &opts.vosk_model_path {
            p.clone()
        } else {
            std::env::var("VOSK_MODEL_PATH")
                .unwrap_or_else(|_| "models/vosk-model-small-en-us-0.15".to_string())
        };
        let autodetect_enabled = std::path::Path::new(&model_path).exists();
        let stt_enabled = opts.stt_enabled.unwrap_or(autodetect_enabled);

        if stt_enabled {
            let stt_audio_rx = audio_tx.subscribe();
            let (stt_transcription_tx, stt_transcription_rx) =
                mpsc::channel::<TranscriptionEvent>(100);
            let (stt_vad_tx, stt_vad_rx) = mpsc::channel::<VadEvent>(200);

            let stt_config = TranscriptionConfig {
                enabled: true,
                model_path,
                partial_results: true,
                max_alternatives: 1,
                include_words: false,
                buffer_size_ms: 512,
            };
            let stt_processor =
                SttProcessor::new(stt_audio_rx, stt_vad_rx, stt_transcription_tx, stt_config)
                    .map_err(|e| format!("Failed to create STT: {}", e))?;
            let stt_handle = tokio::spawn(async move { stt_processor.run().await });

            stt_transcription_rx_opt = Some(stt_transcription_rx);
            stt_handle_opt = Some(stt_handle);
            Some(stt_vad_tx)
        } else {
            None
        }
    };
    #[cfg(not(feature = "vosk"))]
    let _stt_vad_tx_opt: Option<mpsc::Sender<VadEvent>> = None;

    // Fanout task
    let vad_bcast_tx_clone = vad_bcast_tx.clone();
    #[cfg(feature = "vosk")]
    let stt_vad_tx_clone = stt_vad_tx_opt.clone();
    let vad_fanout_handle = tokio::spawn(async move {
        let mut rx = raw_vad_rx;
        while let Some(ev) = rx.recv().await {
            let _ = vad_bcast_tx_clone.send(ev);
            #[cfg(feature = "vosk")]
            if let Some(stt_tx) = &stt_vad_tx_clone {
                let _ = stt_tx.send(ev).await;
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

    Ok(AppHandle {
        metrics,
        vad_tx: vad_bcast_tx,
        raw_vad_tx,
        audio_tx,
        current_mode: std::sync::Arc::new(parking_lot::RwLock::new(opts.activation_mode)),
        #[cfg(feature = "vosk")]
        stt_rx: stt_transcription_rx_opt,
        audio_capture,
        chunker_handle,
        trigger_handle,
        vad_fanout_handle,
        #[cfg(feature = "vosk")]
        stt_handle: stt_handle_opt,
        #[cfg(feature = "text-injection")]
        injection_handle,
    })
}
