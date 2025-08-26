use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tokio::time::{self, Duration};

use crate::audio::frame_reader::FrameReader;
use crate::audio::vad_processor::AudioFrame as VadFrame;
use crate::telemetry::pipeline_metrics::{PipelineMetrics, PipelineStage};

pub struct ChunkerConfig {
    pub frame_size_samples: usize,
    pub sample_rate_hz: u32,
}

impl Default for ChunkerConfig {
    fn default() -> Self {
        Self {
            frame_size_samples: 512,
            sample_rate_hz: 16_000,
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
        }
    }

    async fn run(&mut self, running: Arc<AtomicBool>) {
        tracing::info!("Audio chunker started");

        while running.load(Ordering::SeqCst) {
            if let Some(frame) = self.frame_reader.read_frame(4096) {
                if let Some(m) = &self.metrics {
                    m.update_audio_level(&frame.samples);
                    m.mark_stage_active(PipelineStage::Capture);
                }
                self.buffer.extend(frame.samples);
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
                m.mark_stage_active(PipelineStage::Chunker);
            }
        }
    }
}
