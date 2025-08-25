use crossbeam_channel::{Receiver, Sender};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::audio::capture::AudioFrame as CaptureFrame;
use crate::audio::vad_processor::AudioFrame as VadFrame;

/// Fixed-size frame chunker for VAD engines (e.g., Silero requires 512 @ 16 kHz).
///
/// Contract
/// - Input: 16 kHz, mono PCM i16 in arbitrary slice sizes via CaptureFrame.
/// - Output: Non-overlapping frames of exactly `frame_size_samples`, delivered as VadFrame.
/// - Timestamps: Derived from sample cursor and `sample_rate_hz` (not from wall-clock).
/// - Sample rate: Expected to be `sample_rate_hz`; mismatch is warned and samples are still consumed as-is (no resampling here).
/// - Overlap: Not supported initially. Set hop_size == frame_size. If overlap is needed, update timestamp/hop logic accordingly.
pub struct ChunkerConfig {
    pub frame_size_samples: usize, // e.g., 512
    pub sample_rate_hz: u32,       // e.g., 16000
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
    input_rx: Receiver<CaptureFrame>,
    output_tx: Sender<VadFrame>,
    cfg: ChunkerConfig,
    running: Arc<AtomicBool>,
}

impl AudioChunker {
    pub fn new(
        input_rx: Receiver<CaptureFrame>,
        output_tx: Sender<VadFrame>,
        cfg: ChunkerConfig,
    ) -> Self {
        Self {
            input_rx,
            output_tx,
            cfg,
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Spawn the chunker on a dedicated thread and return a JoinHandle.
    pub fn spawn(self) -> ChunkerHandle {
        let mut worker = ChunkerWorker::new(self.input_rx, self.output_tx, self.cfg);
        let running = Arc::clone(&self.running);
        running.store(true, Ordering::SeqCst);
        let handle = thread::Builder::new()
            .name("audio-chunker".to_string())
            .spawn(move || {
                worker.run(running);
            })
            .expect("failed to spawn audio-chunker thread");
        ChunkerHandle { handle, running: self.running }
    }
}

/// Handle to a running chunker thread.
pub struct ChunkerHandle {
    handle: thread::JoinHandle<()>,
    running: Arc<AtomicBool>,
}

impl ChunkerHandle {
    /// Signal the chunker to stop.
    pub fn stop(&self) { self.running.store(false, Ordering::SeqCst); }
    /// Join the underlying thread.
    pub fn join(self) { let _ = self.handle.join(); }
}

struct ChunkerWorker {
    input_rx: Receiver<CaptureFrame>,
    output_tx: Sender<VadFrame>,
    cfg: ChunkerConfig,
    buffer: VecDeque<i16>,
    /// Total number of samples emitted in completed output frames (for timestamping).
    samples_emitted: u64,
}

impl ChunkerWorker {
    fn new(input_rx: Receiver<CaptureFrame>, output_tx: Sender<VadFrame>, cfg: ChunkerConfig) -> Self {
        let cap = cfg.frame_size_samples * 4;
        Self {
            input_rx,
            output_tx,
            cfg,
            buffer: VecDeque::with_capacity(cap),
            samples_emitted: 0,
        }
    }

    fn run(&mut self, running: Arc<AtomicBool>) {
        tracing::info!(
            frame_size = self.cfg.frame_size_samples,
            sr = self.cfg.sample_rate_hz,
            "Audio chunker started"
        );

        while running.load(Ordering::SeqCst) {
            match self.input_rx.recv_timeout(Duration::from_millis(100)) {
                Ok(frame) => {
                    if frame.sample_rate != self.cfg.sample_rate_hz {
                        // No resampling here; warn once per mismatch burst.
                        tracing::warn!(
                            input_sr = frame.sample_rate,
                            expected_sr = self.cfg.sample_rate_hz,
                            "Chunker received mismatched sample rate; proceeding without resample"
                        );
                    }
                    if frame.channels != 1 {
                        tracing::warn!(
                            channels = frame.channels,
                            "Chunker expects mono input; upstream should downmix"
                        );
                    }

                    // Ingest samples
                    self.buffer.extend(frame.samples);
                    self.flush_ready_frames();
                }
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                    // idle
                }
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                    tracing::warn!("Audio chunker input disconnected; shutting down");
                    break;
                }
            }
        }

        tracing::info!("Audio chunker stopped");
    }

    fn flush_ready_frames(&mut self) {
        let fs = self.cfg.frame_size_samples;
        while self.buffer.len() >= fs {
            // Pop exactly fs samples preserving order.
            let mut out = Vec::with_capacity(fs);
            for _ in 0..fs {
                // unwrap safe due to len check
                out.push(self.buffer.pop_front().unwrap());
            }

            let timestamp_ms = (self.samples_emitted as u128)
                .saturating_mul(1000)
                .checked_div(self.cfg.sample_rate_hz as u128)
                .unwrap_or(0) as u64;

            let vf = VadFrame {
                data: out,
                timestamp_ms,
            };

            if let Err(e) = self.output_tx.send(vf) {
                tracing::error!("Audio chunker failed to send frame: {}", e);
                // On backpressure or disconnect, we drop remaining ready frames to avoid stalls.
                break;
            }

            self.samples_emitted += fs as u64;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam_channel::bounded;
    use std::time::Instant;

    fn cap_frame(len: usize) -> CaptureFrame {
        CaptureFrame {
            samples: vec![1i16; len],
            timestamp: Instant::now(),
            sample_rate: 16_000,
            channels: 1,
        }
    }

    #[test]
    fn produces_exact_sized_frames() {
        // Setup channels
        let (tx_in, rx_in) = bounded::<CaptureFrame>(8);
        let (tx_out, rx_out) = bounded::<VadFrame>(8);

        let cfg = ChunkerConfig::default();
        let mut worker = ChunkerWorker::new(rx_in, tx_out, cfg);

        // Feed 300 + 300 + 300 = 900 samples; expect 1 full frame (512) and 388 buffered
        tx_in.send(cap_frame(300)).unwrap();
        tx_in.send(cap_frame(300)).unwrap();
        tx_in.send(cap_frame(300)).unwrap();

        // Manually run one iteration
        worker.flush_ready_frames(); // nothing yet since we didn't ingest via run()
        // Simulate run loop ingest
        worker.buffer.extend(vec![1i16; 900]);
        worker.flush_ready_frames();

        let first = rx_out.try_recv().expect("one frame expected");
        assert_eq!(first.data.len(), 512);
        assert_eq!(first.timestamp_ms, 0);
        assert!(rx_out.try_recv().is_err());

        // Now add 200 more to reach 588 -> expect second frame
        worker.buffer.extend(vec![1i16; 200]);
        worker.flush_ready_frames();
        let second = rx_out.try_recv().expect("second frame expected");
        assert_eq!(second.data.len(), 512);
        assert_eq!(second.timestamp_ms, 512 * 1000 / 16_000);
    }

    #[test]
    fn timestamp_monotonic_non_overlapping() {
        let (tx_in, rx_in) = bounded::<CaptureFrame>(8);
        let (tx_out, rx_out) = bounded::<VadFrame>(8);
        let cfg = ChunkerConfig::default();
        let mut worker = ChunkerWorker::new(rx_in, tx_out, cfg);

        // Feed exactly 1024 samples
        worker.buffer.extend(vec![1i16; 1024]);
        worker.flush_ready_frames();
        let a = rx_out.try_recv().unwrap();
        let b = rx_out.try_recv().unwrap();
        assert_eq!(a.timestamp_ms, 0);
        assert_eq!(b.timestamp_ms, (512u64 * 1000) / 16_000);
    }
}
