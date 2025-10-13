use parking_lot::RwLock;
use std::sync::atomic::{AtomicBool, AtomicI16, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Shared metrics for cross-thread pipeline monitoring
#[derive(Clone)]
pub struct PipelineMetrics {
    // Audio level monitoring
    pub current_peak: Arc<AtomicI16>, // Peak sample value in current window
    pub current_rms: Arc<AtomicU64>,  // RMS * 1000 for precision
    pub audio_level_db: Arc<AtomicI16>, // Current level in dB * 10

    // Pipeline stage tracking
    pub stage_capture: Arc<AtomicBool>, // Data reached capture stage
    pub stage_chunker: Arc<AtomicBool>, // Data reached chunker stage
    pub stage_vad: Arc<AtomicBool>,     // Data reached VAD stage
    pub stage_output: Arc<AtomicBool>,  // Data reached output stage

    // Buffer monitoring
    pub capture_buffer_fill: Arc<AtomicUsize>, // Capture buffer fill %
    pub chunker_buffer_fill: Arc<AtomicUsize>, // Chunker buffer fill %
    pub vad_buffer_fill: Arc<AtomicUsize>,     // VAD buffer fill %

    // Frame rate tracking
    pub capture_fps: Arc<AtomicU64>, // Frames per second * 10
    pub chunker_fps: Arc<AtomicU64>, // Chunks per second * 10
    pub vad_fps: Arc<AtomicU64>,     // VAD frames per second * 10

    // Event counters
    pub capture_frames: Arc<AtomicU64>,
    pub chunker_frames: Arc<AtomicU64>,

    // Latency tracking
    pub capture_to_chunker_ms: Arc<AtomicU64>, // Latency in ms
    pub chunker_to_vad_ms: Arc<AtomicU64>,     // Latency in ms
    pub end_to_end_ms: Arc<AtomicU64>,         // Total pipeline latency

    // Activity indicators
    pub is_speaking: Arc<AtomicBool>, // Currently in speech
    pub last_speech_time: Arc<RwLock<Option<Instant>>>,
    pub speech_segments_count: Arc<AtomicU64>,

    // Error tracking
    pub capture_errors: Arc<AtomicU64>,
    pub chunker_errors: Arc<AtomicU64>,

    // STT metrics (plugin manager)
    pub stt_failover_count: Arc<AtomicU64>,
    pub stt_total_errors: Arc<AtomicU64>,
    pub stt_last_failover_secs: Arc<AtomicU64>,
    pub stt_unload_count: Arc<AtomicU64>,
    pub stt_unload_errors: Arc<AtomicU64>,
    pub stt_load_count: Arc<AtomicU64>,
    pub stt_load_errors: Arc<AtomicU64>,
    pub stt_init_success: Arc<AtomicU64>,
    pub stt_init_failures: Arc<AtomicU64>,
    pub stt_active_plugins: Arc<AtomicUsize>,
    pub stt_transcription_requests: Arc<AtomicU64>,
    pub stt_transcription_success: Arc<AtomicU64>,
    pub stt_transcription_failures: Arc<AtomicU64>,
    pub stt_last_transcription_latency_ms: Arc<AtomicU64>,
    pub stt_last_load_duration_ms: Arc<AtomicU64>,
    pub stt_last_init_duration_ms: Arc<AtomicU64>,
    pub stt_last_unload_duration_ms: Arc<AtomicU64>,
    pub stt_audio_fps: Arc<AtomicU64>,
    pub stt_gc_runs: Arc<AtomicU64>,
    pub vad_detection_latency_ms: Arc<AtomicU64>,
    pub vad_to_stt_handoff_latency_ms: Arc<AtomicU64>,
}

impl Default for PipelineMetrics {
    fn default() -> Self {
        Self {
            current_peak: Arc::new(AtomicI16::new(0)),
            current_rms: Arc::new(AtomicU64::new(0)),
            audio_level_db: Arc::new(AtomicI16::new(-900)),

            stage_capture: Arc::new(AtomicBool::new(false)),
            stage_chunker: Arc::new(AtomicBool::new(false)),
            stage_vad: Arc::new(AtomicBool::new(false)),
            stage_output: Arc::new(AtomicBool::new(false)),

            capture_buffer_fill: Arc::new(AtomicUsize::new(0)),
            chunker_buffer_fill: Arc::new(AtomicUsize::new(0)),
            vad_buffer_fill: Arc::new(AtomicUsize::new(0)),

            capture_fps: Arc::new(AtomicU64::new(0)),
            chunker_fps: Arc::new(AtomicU64::new(0)),
            vad_fps: Arc::new(AtomicU64::new(0)),

            capture_frames: Arc::new(AtomicU64::new(0)),
            chunker_frames: Arc::new(AtomicU64::new(0)),

            capture_to_chunker_ms: Arc::new(AtomicU64::new(0)),
            chunker_to_vad_ms: Arc::new(AtomicU64::new(0)),
            end_to_end_ms: Arc::new(AtomicU64::new(0)),

            is_speaking: Arc::new(AtomicBool::new(false)),
            last_speech_time: Arc::new(RwLock::new(None)),
            speech_segments_count: Arc::new(AtomicU64::new(0)),

            capture_errors: Arc::new(AtomicU64::new(0)),
            chunker_errors: Arc::new(AtomicU64::new(0)),

            stt_failover_count: Arc::new(AtomicU64::new(0)),
            stt_total_errors: Arc::new(AtomicU64::new(0)),
            stt_last_failover_secs: Arc::new(AtomicU64::new(0)),
            stt_unload_count: Arc::new(AtomicU64::new(0)),
            stt_unload_errors: Arc::new(AtomicU64::new(0)),
            stt_load_count: Arc::new(AtomicU64::new(0)),
            stt_load_errors: Arc::new(AtomicU64::new(0)),
            stt_init_success: Arc::new(AtomicU64::new(0)),
            stt_init_failures: Arc::new(AtomicU64::new(0)),
            stt_active_plugins: Arc::new(AtomicUsize::new(0)),
            stt_transcription_requests: Arc::new(AtomicU64::new(0)),
            stt_transcription_success: Arc::new(AtomicU64::new(0)),
            stt_transcription_failures: Arc::new(AtomicU64::new(0)),
            stt_last_transcription_latency_ms: Arc::new(AtomicU64::new(0)),
            stt_last_load_duration_ms: Arc::new(AtomicU64::new(0)),
            stt_last_init_duration_ms: Arc::new(AtomicU64::new(0)),
            stt_last_unload_duration_ms: Arc::new(AtomicU64::new(0)),
            stt_audio_fps: Arc::new(AtomicU64::new(0)),
            stt_gc_runs: Arc::new(AtomicU64::new(0)),
            vad_detection_latency_ms: Arc::new(AtomicU64::new(0)),
            vad_to_stt_handoff_latency_ms: Arc::new(AtomicU64::new(0)),
        }
    }
}

impl PipelineMetrics {
    pub fn update_audio_level(&self, samples: &[i16]) {
        if samples.is_empty() {
            return;
        }

        let peak = samples.iter().map(|&s| s.abs()).max().unwrap_or(0);
        self.current_peak.store(peak, Ordering::Relaxed);

        let sum: i64 = samples.iter().map(|&s| s as i64 * s as i64).sum();
        let rms = ((sum as f64 / samples.len() as f64).sqrt() * 1000.0) as u64;
        self.current_rms.store(rms, Ordering::Relaxed);

        let db = if peak > 0 {
            (20.0 * (peak as f64 / 32768.0).log10() * 10.0) as i16
        } else {
            -900
        };
        self.audio_level_db.store(db, Ordering::Relaxed);
    }

    pub fn mark_stage_active(&self, stage: PipelineStage) {
        match stage {
            PipelineStage::Capture => self.stage_capture.store(true, Ordering::Relaxed),
            PipelineStage::Chunker => self.stage_chunker.store(true, Ordering::Relaxed),
            PipelineStage::Vad => self.stage_vad.store(true, Ordering::Relaxed),
            PipelineStage::Output => self.stage_output.store(true, Ordering::Relaxed),
        }
    }

    pub fn decay_stages(&self) {
        self.stage_capture.store(false, Ordering::Relaxed);
        self.stage_chunker.store(false, Ordering::Relaxed);
        self.stage_vad.store(false, Ordering::Relaxed);
        self.stage_output.store(false, Ordering::Relaxed);
    }

    pub fn update_buffer_fill(&self, buffer: BufferType, fill_percent: usize) {
        let fill = fill_percent.min(100);
        match buffer {
            BufferType::Capture => self.capture_buffer_fill.store(fill, Ordering::Relaxed),
            BufferType::Chunker => self.chunker_buffer_fill.store(fill, Ordering::Relaxed),
            BufferType::Vad => self.vad_buffer_fill.store(fill, Ordering::Relaxed),
        }
    }

    pub fn update_capture_fps(&self, fps: f64) {
        self.capture_fps.store((fps * 10.0) as u64, Ordering::Relaxed);
    }

    pub fn update_chunker_fps(&self, fps: f64) {
        self.chunker_fps.store((fps * 10.0) as u64, Ordering::Relaxed);
    }

    pub fn update_vad_fps(&self, fps: f64) {
        self.vad_fps.store((fps * 10.0) as u64, Ordering::Relaxed);
    }

    pub fn increment_capture_frames(&self) {
        self.capture_frames.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_chunker_frames(&self) {
        self.chunker_frames.fetch_add(1, Ordering::Relaxed);
    }

    pub fn update_vad_detection_latency(&self, latency_ms: u64) {
        let current = self.vad_detection_latency_ms.load(Ordering::Relaxed);
        if latency_ms > current {
            self.vad_detection_latency_ms.store(latency_ms, Ordering::Relaxed);
        }
    }

    pub fn update_vad_to_stt_handoff_latency(&self, latency_ms: u64) {
        let current = self.vad_to_stt_handoff_latency_ms.load(Ordering::Relaxed);
        if latency_ms > current {
            self.vad_to_stt_handoff_latency_ms.store(latency_ms, Ordering::Relaxed);
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PipelineStage {
    Capture,
    Chunker,
    Vad,
    Output,
}

#[derive(Debug, Clone, Copy)]
pub enum BufferType {
    Capture,
    Chunker,
    Vad,
}

#[derive(Debug)]
pub struct FpsTracker {
    last_update: Instant,
    frame_count: u64,
}

impl FpsTracker {
    pub fn new() -> Self {
        Self {
            last_update: Instant::now(),
            frame_count: 0,
        }
    }

    pub fn tick(&mut self) -> Option<f64> {
        self.frame_count += 1;
        let elapsed = self.last_update.elapsed();

        if elapsed >= Duration::from_secs(1) {
            let fps = self.frame_count as f64 / elapsed.as_secs_f64();
            self.last_update = Instant::now();
            self.frame_count = 0;
            Some(fps)
        } else {
            None
        }
    }
}

impl Default for FpsTracker {
    fn default() -> Self {
        Self::new()
    }
}
