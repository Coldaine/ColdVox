use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tokio::time::{self, Duration};

use super::capture::DeviceConfig;
use super::frame_reader::FrameReader;
use super::resampler::StreamResampler;
use coldvox_telemetry::{FpsTracker, PipelineMetrics, PipelineStage};

// AudioFrame will be defined in the VAD crate
#[derive(Debug, Clone)]
pub struct AudioFrame {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub timestamp: std::time::Instant,
}

#[derive(Debug, Clone, Copy)]
pub enum ResamplerQuality {
    Fast,     // Lower quality, lower CPU usage
    Balanced, // Default quality/performance balance
    Quality,  // Higher quality, higher CPU usage
}

pub struct ChunkerConfig {
    pub frame_size_samples: usize,
    pub sample_rate_hz: u32,
    pub resampler_quality: ResamplerQuality,
}

impl Default for ChunkerConfig {
    fn default() -> Self {
        Self {
            frame_size_samples: 512,
            sample_rate_hz: 16_000,
            resampler_quality: ResamplerQuality::Balanced,
        }
    }
}

pub struct AudioChunker {
    frame_reader: FrameReader,
    output_tx: broadcast::Sender<AudioFrame>,
    cfg: ChunkerConfig,
    running: Arc<AtomicBool>,
    metrics: Option<Arc<PipelineMetrics>>,
    device_cfg_rx: Option<broadcast::Receiver<DeviceConfig>>,
}

impl AudioChunker {
    pub fn new(
        frame_reader: FrameReader,
        output_tx: broadcast::Sender<AudioFrame>,
        cfg: ChunkerConfig,
    ) -> Self {
        Self {
            frame_reader,
            output_tx,
            cfg,
            running: Arc::new(AtomicBool::new(false)),
            metrics: None,
            device_cfg_rx: None,
        }
    }

    pub fn with_metrics(mut self, metrics: Arc<PipelineMetrics>) -> Self {
        self.metrics = Some(metrics);
        self
    }

    pub fn with_device_config(mut self, rx: broadcast::Receiver<DeviceConfig>) -> Self {
        self.device_cfg_rx = Some(rx);
        self
    }

    pub fn spawn(self) -> JoinHandle<()> {
        let mut worker = ChunkerWorker::new(
            self.frame_reader,
            self.output_tx,
            self.cfg,
            self.metrics,
            self.device_cfg_rx,
        );
        self.running.store(true, Ordering::SeqCst);
        let running = self.running.clone();

        tokio::spawn(async move {
            worker.run(running).await;
        })
    }
}

struct ChunkerWorker {
    frame_reader: FrameReader,
    output_tx: broadcast::Sender<AudioFrame>,
    cfg: ChunkerConfig,
    buffer: VecDeque<i16>,
    samples_emitted: u64,
    metrics: Option<Arc<PipelineMetrics>>,
    capture_fps_tracker: FpsTracker,
    chunker_fps_tracker: FpsTracker,
    // Resampling state
    resampler: Option<Arc<parking_lot::Mutex<StreamResampler>>>,
    current_input_rate: Option<u32>,
    current_input_channels: Option<u16>,
    device_cfg_rx: Option<broadcast::Receiver<DeviceConfig>>,
    start_time: std::time::Instant,
}

impl ChunkerWorker {
    fn new(
        frame_reader: FrameReader,
        output_tx: broadcast::Sender<AudioFrame>,
        cfg: ChunkerConfig,
        metrics: Option<Arc<PipelineMetrics>>,
        device_cfg_rx: Option<broadcast::Receiver<DeviceConfig>>,
    ) -> Self {
        let cap = cfg.frame_size_samples * 4;
        Self {
            frame_reader,
            output_tx,
            cfg,
            buffer: VecDeque::with_capacity(cap),
            samples_emitted: 0,
            metrics,
            capture_fps_tracker: FpsTracker::new(),
            chunker_fps_tracker: FpsTracker::new(),
            resampler: None,
            current_input_rate: None,
            current_input_channels: None,
            device_cfg_rx,
            start_time: std::time::Instant::now(),
        }
    }

    async fn run(&mut self, running: Arc<AtomicBool>) {
        tracing::info!("Audio chunker started");

        while running.load(Ordering::SeqCst) {
            // Apply device config updates if any
            if let Some(rx) = &mut self.device_cfg_rx {
                while let Ok(cfg) = rx.try_recv() {
                    self.frame_reader
                        .update_device_config(cfg.sample_rate, cfg.channels);
                }
            }
            if let Some(frame) = self.frame_reader.read_frame(4096) {
                if let Some(m) = &self.metrics {
                    m.increment_capture_frames();
                    if let Some(fps) = self.capture_fps_tracker.tick() {
                        m.update_capture_fps(fps);
                    }
                    m.update_audio_level(&frame.samples);
                    m.mark_stage_active(PipelineStage::Capture);
                }

                // Check if device configuration has changed
                if self.current_input_rate != Some(frame.sample_rate)
                    || self.current_input_channels != Some(frame.channels)
                {
                    self.reconfigure_for_device(&frame);
                }

                // Process the frame (resampling and channel conversion)
                let processed_samples = self.process_frame(&frame);
                self.buffer.extend(processed_samples);
                self.flush_ready_frames().await;
            } else {
                // Sleep for 25ms when no data available. At 16kHz with 512-sample chunks,
                // new chunks arrive every 32ms. Polling at 40Hz (25ms) ensures we check
                // at least once per chunk period while reducing CPU usage by ~96% compared
                // to 1ms polling. Could use event-driven design in future, but this works well.
                time::sleep(Duration::from_millis(25)).await;
            }
        }

        tracing::info!("Audio chunker stopped");
    }

    async fn flush_ready_frames(&mut self) {
        let fs = self.cfg.frame_size_samples;
        while self.buffer.len() >= fs {
            let mut out = Vec::with_capacity(fs);
            for _ in 0..fs {
                out.push(self.buffer.pop_front().unwrap());
            }

            // Calculate timestamp based on samples emitted
            let timestamp_ms =
                (self.samples_emitted as u128 * 1000 / self.cfg.sample_rate_hz as u128) as u64;
            let timestamp = self.start_time + std::time::Duration::from_millis(timestamp_ms);

            let vf = AudioFrame {
                samples: out
                    .into_iter()
                    .map(|s| s as f32 / i16::MAX as f32)
                    .collect(),
                sample_rate: self.cfg.sample_rate_hz,
                timestamp,
            };

            // A send on a broadcast channel can fail if there are no receivers.
            // This is not a critical error for us; it just means no one is listening.
            match self.output_tx.send(vf) {
                Ok(num_receivers) => {
                    tracing::trace!("Chunker: Frame sent to {} receivers", num_receivers);
                }
                Err(_) => {
                    tracing::warn!("No active listeners for audio frames.");
                }
            }

            self.samples_emitted += fs as u64;

            if let Some(m) = &self.metrics {
                m.increment_chunker_frames();
                if let Some(fps) = self.chunker_fps_tracker.tick() {
                    m.update_chunker_fps(fps);
                }
                m.mark_stage_active(PipelineStage::Chunker);
            }
        }
    }

    fn reconfigure_for_device(&mut self, frame: &super::capture::AudioFrame) {
        let needs_resampling = frame.sample_rate != self.cfg.sample_rate_hz;

        if needs_resampling {
            tracing::info!(
                "Configuring resampler: {}Hz {} ch -> {}Hz mono",
                frame.sample_rate,
                frame.channels,
                self.cfg.sample_rate_hz
            );

            let resampler = StreamResampler::new_with_quality(
                frame.sample_rate,
                self.cfg.sample_rate_hz,
                self.cfg.resampler_quality,
            );

            self.resampler = Some(Arc::new(parking_lot::Mutex::new(resampler)));
        } else {
            tracing::info!(
                "Device already at target rate {}Hz, no resampling needed",
                frame.sample_rate
            );
            self.resampler = None;
        }

        self.current_input_rate = Some(frame.sample_rate);
        self.current_input_channels = Some(frame.channels);
    }

    fn process_frame(&mut self, frame: &super::capture::AudioFrame) -> Vec<i16> {
        // First, handle channel conversion if needed
        let mono_samples = if frame.channels == 1 {
            frame.samples.clone()
        } else {
            // Convert multi-channel to mono by averaging
            let channels = frame.channels as usize;
            frame
                .samples
                .chunks_exact(channels)
                .map(|chunk| {
                    let sum: i32 = chunk.iter().map(|&s| s as i32).sum();
                    (sum / channels as i32) as i16
                })
                .collect()
        };

        // Then, apply resampling if needed
        if let Some(resampler) = &self.resampler {
            resampler.lock().process(&mono_samples)
        } else {
            mono_samples
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::AudioFrame as CapFrame;
    use crate::ring_buffer::AudioRingBuffer;
    use std::time::Instant;

    #[test]
    fn reconfigure_resampler_on_rate_change() {
        let rb = AudioRingBuffer::new(1024);
        let (_prod, cons) = rb.split();
        let reader = FrameReader::new(cons, 48_000, 2, 1024, None);
        let (tx, _rx) = broadcast::channel::<AudioFrame>(8);
        let cfg = ChunkerConfig {
            frame_size_samples: 512,
            sample_rate_hz: 16_000,
            resampler_quality: ResamplerQuality::Balanced,
        };
        let mut worker = ChunkerWorker::new(reader, tx, cfg, None, None);

        // First frame at 48kHz stereo -> resampler should be created
        let frame1 = CapFrame {
            samples: vec![0i16; 480],
            timestamp: Instant::now(),
            sample_rate: 48_000,
            channels: 2,
        };
        worker.reconfigure_for_device(&frame1);
        assert!(worker.resampler.is_some());

        // Frame at 16k mono -> resampler not needed
        let frame2 = CapFrame {
            samples: vec![0i16; 160],
            timestamp: Instant::now(),
            sample_rate: 16_000,
            channels: 1,
        };
        worker.reconfigure_for_device(&frame2);
        assert!(worker.resampler.is_none());
    }

    #[test]
    fn stereo_to_mono_averaging() {
        let rb = AudioRingBuffer::new(1024);
        let (_prod, cons) = rb.split();
        let reader = FrameReader::new(cons, 16_000, 2, 1024, None);
        let (tx, _rx) = broadcast::channel::<AudioFrame>(8);
        let cfg = ChunkerConfig {
            frame_size_samples: 512,
            sample_rate_hz: 16_000,
            resampler_quality: ResamplerQuality::Balanced,
        };
        let mut worker = ChunkerWorker::new(reader, tx, cfg, None, None);

        let samples = vec![1000i16, -1000, 900, -900, 800, -800, 700, -700];
        let frame = CapFrame {
            samples,
            timestamp: Instant::now(),
            sample_rate: 16_000,
            channels: 2,
        };
        worker.reconfigure_for_device(&frame);
        let out = worker.process_frame(&frame);
        // Each pair averaged -> zeros
        assert_eq!(out, vec![0, 0, 0, 0]);
    }
}
