use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tokio::time::{self, Duration};

use crate::audio::frame_reader::FrameReader;
use crate::audio::resampler::StreamResampler;
use crate::audio::vad_processor::AudioFrame as VadFrame;
use crate::telemetry::pipeline_metrics::{FpsTracker, PipelineMetrics, PipelineStage};

#[derive(Debug, Clone, Copy)]
pub enum ResamplerQuality {
    Fast,      // Lower quality, lower CPU usage
    Balanced,  // Default quality/performance balance
    Quality,   // Higher quality, higher CPU usage
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
    output_tx: broadcast::Sender<VadFrame>,
    cfg: ChunkerConfig,
    running: Arc<AtomicBool>,
    metrics: Option<Arc<PipelineMetrics>>,
}

impl AudioChunker {
    pub fn new(
        frame_reader: FrameReader,
        output_tx: broadcast::Sender<VadFrame>,
        cfg: ChunkerConfig,
    ) -> Self {
        Self {
            frame_reader,
            output_tx,
            cfg,
            running: Arc::new(AtomicBool::new(false)),
            metrics: None,
        }
    }

    pub fn with_metrics(mut self, metrics: Arc<PipelineMetrics>) -> Self {
        self.metrics = Some(metrics);
        self
    }

    pub fn spawn(self) -> JoinHandle<()> {
        let mut worker =
            ChunkerWorker::new(self.frame_reader, self.output_tx, self.cfg, self.metrics);
        self.running.store(true, Ordering::SeqCst);
        let running = self.running.clone();

        tokio::spawn(async move {
            worker.run(running).await;
        })
    }
}

struct ChunkerWorker {
    frame_reader: FrameReader,
    output_tx: broadcast::Sender<VadFrame>,
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
}

impl ChunkerWorker {
    fn new(
        frame_reader: FrameReader,
        output_tx: broadcast::Sender<VadFrame>,
        cfg: ChunkerConfig,
        metrics: Option<Arc<PipelineMetrics>>,
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
        }
    }

    async fn run(&mut self, running: Arc<AtomicBool>) {
        tracing::info!("Audio chunker started");

        while running.load(Ordering::SeqCst) {
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
                    || self.current_input_channels != Some(frame.channels) {
                    self.reconfigure_for_device(&frame);
                }
                
                // Process the frame (resampling and channel conversion)
                let processed_samples = self.process_frame(&frame);
                self.buffer.extend(processed_samples);
                self.flush_ready_frames().await;
            } else {
                time::sleep(Duration::from_millis(1)).await;
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

            let timestamp_ms =
                (self.samples_emitted as u128 * 1000 / self.cfg.sample_rate_hz as u128) as u64;

            let vf = VadFrame {
                data: out,
                timestamp_ms,
            };

            // A send on a broadcast channel can fail if there are no receivers.
            // This is not a critical error for us; it just means no one is listening.
            if self.output_tx.send(vf).is_err() {
                tracing::warn!("No active listeners for audio frames.");
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
    
    fn reconfigure_for_device(&mut self, frame: &crate::audio::capture::AudioFrame) {
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
    
    fn process_frame(&mut self, frame: &crate::audio::capture::AudioFrame) -> Vec<i16> {
        // First, handle channel conversion if needed
        let mono_samples = if frame.channels == 1 {
            frame.samples.clone()
        } else {
            // Convert multi-channel to mono by averaging
            let channels = frame.channels as usize;
            frame.samples
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
    use crate::audio::ring_buffer::AudioRingBuffer;
    use crate::audio::capture::AudioFrame;
    use std::time::Instant;

    #[test]
    fn reconfigure_resampler_on_rate_change() {
        let rb = AudioRingBuffer::new(1024);
        let (_prod, cons) = rb.split();
        let reader = FrameReader::new(cons, 48_000, 2, 1024, None);
        let (tx, _rx) = broadcast::channel::<VadFrame>(8);
        let cfg = ChunkerConfig { frame_size_samples: 512, sample_rate_hz: 16_000, resampler_quality: ResamplerQuality::Balanced };
        let mut worker = ChunkerWorker::new(reader, tx, cfg, None);

        // First frame at 48kHz stereo -> resampler should be created
        let frame1 = AudioFrame { samples: vec![0i16; 480], timestamp: Instant::now(), sample_rate: 48_000, channels: 2 };
        worker.reconfigure_for_device(&frame1);
        assert!(worker.resampler.is_some());

        // Frame at 16k mono -> resampler not needed
        let frame2 = AudioFrame { samples: vec![0i16; 160], timestamp: Instant::now(), sample_rate: 16_000, channels: 1 };
        worker.reconfigure_for_device(&frame2);
        assert!(worker.resampler.is_none());
    }

    #[test]
    fn stereo_to_mono_averaging() {
        let rb = AudioRingBuffer::new(1024);
        let (_prod, cons) = rb.split();
        let reader = FrameReader::new(cons, 16_000, 2, 1024, None);
        let (tx, _rx) = broadcast::channel::<VadFrame>(8);
        let cfg = ChunkerConfig { frame_size_samples: 512, sample_rate_hz: 16_000, resampler_quality: ResamplerQuality::Balanced };
        let mut worker = ChunkerWorker::new(reader, tx, cfg, None);

        let samples = vec![1000i16, -1000, 900, -900, 800, -800, 700, -700];
        let frame = AudioFrame { samples, timestamp: Instant::now(), sample_rate: 16_000, channels: 2 };
        worker.reconfigure_for_device(&frame);
        let out = worker.process_frame(&frame);
        // Each pair averaged -> zeros
        assert_eq!(out, vec![0, 0, 0, 0]);
    }
}
