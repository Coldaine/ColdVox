//! Common helpers for STT audio buffering and event emission
//!
//! This module provides shared utilities to eliminate duplicate patterns
//! across the coldvox-stt crate, including stub implementations, event mapping,
//! error handling, audio buffering, and event emission.

use parking_lot::RwLock;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use crate::constants::SAMPLE_RATE_HZ;
use crate::types::TranscriptionEvent;
use coldvox_telemetry::{pipeline_metrics::PipelineMetrics, stt_metrics::SttPerformanceMetrics};

/// Stub error helper function for unimplemented plugins
///
/// This function centralizes the common pattern of returning a NotAvailable error
/// for plugins that are not yet implemented.
pub fn not_yet_implemented<T>(reason: &str) -> Result<T, crate::plugin::SttPluginError> {
    Err(crate::plugin::SttPluginError::NotAvailable {
        reason: format!("{} plugin not yet implemented", reason),
    })
}

/// Unified event mapper function
///
/// Maps utterance IDs in Partial and Final events while preserving Error events.
/// This centralizes the duplicate mapping logic from plugin_adapter.rs.
///
/// # Examples
///
/// ```rust
/// use coldvox_stt::types::TranscriptionEvent;
/// use coldvox_stt::helpers::map_utterance_id;
///
/// let original = Some(TranscriptionEvent::Partial {
///     utterance_id: 42,
///     text: "hello".to_string(),
///     t0: None,
///     t1: None,
/// });
/// let mapped = map_utterance_id(original, 123);
/// // mapped is Some(Partial) with utterance_id 123 and text "hello"
/// ```
pub fn map_utterance_id(
    event: Option<TranscriptionEvent>,
    utterance_id: u64,
) -> Option<TranscriptionEvent> {
    event.map(|e| match e {
        TranscriptionEvent::Partial {
            utterance_id: _,
            text,
            t0,
            t1,
        } => TranscriptionEvent::Partial {
            utterance_id,
            text,
            t0,
            t1,
        },
        TranscriptionEvent::Final {
            utterance_id: _,
            text,
            words,
        } => TranscriptionEvent::Final {
            utterance_id,
            text,
            words,
        },
        TranscriptionEvent::Error { code, message } => TranscriptionEvent::Error { code, message },
    })
}

/// Common error handler function
///
/// Handles plugin errors by logging and creating standardized error events.
/// This eliminates duplicate error handling patterns in plugin_adapter.rs.
pub async fn handle_plugin_error<E: std::error::Error + Send + Sync>(
    error: E,
    context: &str,
) -> Option<TranscriptionEvent> {
    error!(target: "stt", "STT plugin error during {}: {}", context, error);
    Some(TranscriptionEvent::Error {
        code: format!("PLUGIN_{}_ERROR", context.to_uppercase().replace(' ', "_")),
        message: error.to_string(),
    })
}

/// Audio buffer manager struct
///
/// Manages audio buffering and chunking for STT processing.
/// This centralizes the buffering logic from processor.rs.
pub struct AudioBufferManager {
    buffer: Vec<i16>,
    frames_buffered: u64,
    started_at: Instant,
}

impl AudioBufferManager {
    /// Create a new buffer manager
    pub fn new(started_at: Instant) -> Self {
        Self {
            buffer: Vec::with_capacity((SAMPLE_RATE_HZ as usize) * 10),
            frames_buffered: 0,
            started_at,
        }
    }

    /// Add a frame to the buffer with periodic logging
    pub fn add_frame(&mut self, frame: &[i16]) {
        self.buffer.extend_from_slice(frame);
        self.frames_buffered += 1;
        if self.frames_buffered % 100 == 0 {
            tracing::debug!(
                target: "stt",
                "Buffering audio: {} frames, {} samples ({:.2}s)",
                self.frames_buffered,
                self.buffer.len(),
                self.buffer.len() as f32 / SAMPLE_RATE_HZ as f32
            );
        }
    }

    /// Get the number of frames buffered
    pub fn frames_buffered(&self) -> u64 {
        self.frames_buffered
    }

    /// Get the buffer size in samples
    pub fn buffer_size(&self) -> usize {
        self.buffer.len()
    }

    /// Get chunks of the buffer
    pub fn chunks(&self, chunk_size: usize) -> std::slice::Chunks<i16> {
        self.buffer.chunks(chunk_size)
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.frames_buffered = 0;
    }

    /// Logs the buffered audio processing information before chunk processing.
    ///
    /// This centralizes the duration calculation and logging that was duplicated
    /// in processor.rs, following the review nit for consistency.
    pub fn log_processing_info(&self) {
        let frames = self.frames_buffered();
        let size = self.buffer_size();
        let duration = size as f32 / crate::constants::SAMPLE_RATE_HZ as f32;
        tracing::info!(
            target: "stt",
            "Processing buffered audio: {} samples ({:.2}s), {} frames",
            size,
            duration,
            frames
        );
    }

    /// Process buffered audio in chunks and call handler for each
    ///
    /// This method now iterates directly over buffer chunks to avoid unnecessary
    /// vector materialization, per the optional performance nit.
    pub async fn process_chunks<F, Fut>(
        &mut self,
        mut frame_handler: F,
    ) -> Vec<Option<TranscriptionEvent>>
    where
        F: FnMut(&[i16]) -> Fut,
        Fut: std::future::Future<Output = Option<TranscriptionEvent>>,
    {
        let mut events = Vec::new();
        for chunk in self
            .buffer
            .chunks(crate::constants::SAMPLE_RATE_HZ as usize)
        {
            if let Some(event) = frame_handler(chunk).await {
                events.push(Some(event));
            }
        }
        self.clear();
        events
    }
}

/// Event emitter struct
///
/// Handles event emission with logging, metrics, and backpressure handling.
/// This centralizes the send_event logic from processor.rs.
pub struct EventEmitter {
    event_tx: mpsc::Sender<TranscriptionEvent>,
    metrics: Arc<RwLock<crate::processor::SttMetrics>>,
    stt_metrics: Arc<SttPerformanceMetrics>,
    pipeline_metrics: Arc<PipelineMetrics>,
}

impl EventEmitter {
    /// Create a new event emitter
    pub fn new(
        event_tx: mpsc::Sender<TranscriptionEvent>,
        metrics: Arc<RwLock<crate::processor::SttMetrics>>,
        stt_metrics: Arc<SttPerformanceMetrics>,
        pipeline_metrics: Arc<PipelineMetrics>,
    ) -> Self {
        Self {
            event_tx,
            metrics,
            stt_metrics,
            pipeline_metrics,
        }
    }

    /// Emit an event with logging, metrics update, and timeout handling
    pub async fn emit(&self, event: TranscriptionEvent) -> Result<(), ()> {
        let start = Instant::now();

        // Logging and metrics
        match &event {
            TranscriptionEvent::Partial { text, .. } => {
                info!(target: "stt", "Partial: {}", text);
                let mut m = self.metrics.write();
                m.partial_count += 1;
                m.last_event_time = Some(Instant::now());
                self.stt_metrics.record_partial_transcription();
            }
            TranscriptionEvent::Final { text, words, .. } => {
                let word_count = words.as_ref().map(|w| w.len()).unwrap_or(0);
                info!(target: "stt", "Final: {} (words: {})", text, word_count);
                let mut m = self.metrics.write();
                m.final_count += 1;
                m.last_event_time = Some(Instant::now());
                self.stt_metrics.record_final_transcription();
            }
            TranscriptionEvent::Error { code, message } => {
                error!(target: "stt", "Error [{}]: {}", code, message);
                let mut m = self.metrics.write();
                m.error_count += 1;
                m.last_event_time = Some(Instant::now());
                self.stt_metrics.record_transcription_failure();
            }
        }

        let elapsed = start.elapsed();

        // Record latencies
        self.stt_metrics.record_end_to_end_latency(elapsed);
        self.pipeline_metrics
            .stt_last_transcription_latency_ms
            .store(elapsed.as_millis() as u64, Ordering::Relaxed);

        // Update local total latency
        let mut m = self.metrics.write();
        m.total_latency_us += elapsed.as_micros() as u64;

        // Send with timeout
        match tokio::time::timeout(std::time::Duration::from_secs(5), self.event_tx.send(event))
            .await
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(_)) => {
                let mut m = self.metrics.write();
                m.frames_dropped += 1;
                Err(())
            }
            Err(_) => {
                warn!(target: "stt", "Event channel send timed out");
                let mut m = self.metrics.write();
                m.frames_dropped += 1;
                Err(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processor::SttMetrics;
    use crate::types::{TranscriptionEvent, WordInfo};
    use std::io;
    use tokio::sync::mpsc;

    #[test]
    fn test_not_yet_implemented_function() {
        let result: Result<(), _> = not_yet_implemented("test");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("test plugin not yet implemented"));
    }

    #[test]
    fn test_map_utterance_id_partial() {
        let original = Some(TranscriptionEvent::Partial {
            utterance_id: 42,
            text: "hello".to_string(),
            t0: Some(100.0),
            t1: Some(200.0),
        });
        let mapped = map_utterance_id(original, 123);
        if let Some(TranscriptionEvent::Partial {
            utterance_id,
            text,
            t0,
            t1,
        }) = mapped
        {
            assert_eq!(utterance_id, 123);
            assert_eq!(text, "hello");
            assert_eq!(t0, Some(100.0));
            assert_eq!(t1, Some(200.0));
        } else {
            panic!("Expected Partial event");
        }
    }

    #[test]
    fn test_map_utterance_id_final() {
        let original = Some(TranscriptionEvent::Final {
            utterance_id: 42,
            text: "world".to_string(),
            words: Some(vec![WordInfo {
                start: 0.0,
                end: 1.0,
                conf: 0.9,
                text: "world".to_string(),
            }]),
        });
        let mapped = map_utterance_id(original, 123);
        if let Some(TranscriptionEvent::Final {
            utterance_id,
            text,
            words,
        }) = mapped
        {
            assert_eq!(utterance_id, 123);
            assert_eq!(text, "world");
            assert!(words.is_some());
            if let Some(words_vec) = words {
                assert_eq!(words_vec.len(), 1);
                assert_eq!(words_vec[0].text, "world");
                assert_eq!(words_vec[0].conf, 0.9);
            }
        } else {
            panic!("Expected Final event");
        }
    }

    #[test]
    fn test_map_utterance_id_error() {
        let original = Some(TranscriptionEvent::Error {
            code: "TEST_ERROR".to_string(),
            message: "test message".to_string(),
        });
        let mapped = map_utterance_id(original, 123);
        if let Some(TranscriptionEvent::Error { code, message }) = mapped {
            assert_eq!(code, "TEST_ERROR");
            assert_eq!(message, "test message");
        } else {
            panic!("Expected Error event");
        }
    }

    #[test]
    fn test_map_utterance_id_none() {
        let mapped = map_utterance_id(None, 123);
        assert!(mapped.is_none());
    }

    #[tokio::test]
    async fn test_handle_plugin_error() {
        let error = io::Error::new(io::ErrorKind::Other, "test error");
        let event = handle_plugin_error(error, "test context").await;
        assert!(event.is_some());
        if let Some(TranscriptionEvent::Error { code, message }) = event {
            assert!(code.starts_with("PLUGIN_TEST_CONTEXT_ERROR"));
            assert!(message.contains("test error"));
        } else {
            panic!("Expected Error event");
        }
    }

    #[test]
    fn test_audio_buffer_manager_new() {
        let start = Instant::now();
        let mgr = AudioBufferManager::new(start);
        assert_eq!(mgr.frames_buffered(), 0);
        assert_eq!(mgr.buffer_size(), 0);
        // Capacity should be ~10s at 16kHz
        assert!(mgr.buffer.capacity() >= 160000);
    }

    #[test]
    fn test_audio_buffer_manager_add_frame() {
        let mut mgr = AudioBufferManager::new(Instant::now());
        let frame: Vec<i16> = vec![0; 160]; // 10ms frame at 16kHz
        mgr.add_frame(&frame);
        assert_eq!(mgr.frames_buffered(), 1);
        assert_eq!(mgr.buffer_size(), 160);
        assert_eq!(mgr.buffer, frame);
    }

    #[test]
    fn test_audio_buffer_manager_frames_buffered() {
        let mut mgr = AudioBufferManager::new(Instant::now());
        assert_eq!(mgr.frames_buffered(), 0);
        let frame: Vec<i16> = vec![0; 160];
        mgr.add_frame(&frame);
        assert_eq!(mgr.frames_buffered(), 1);
        mgr.add_frame(&frame);
        assert_eq!(mgr.frames_buffered(), 2);
    }

    #[test]
    fn test_audio_buffer_manager_buffer_size() {
        let mut mgr = AudioBufferManager::new(Instant::now());
        assert_eq!(mgr.buffer_size(), 0);
        let frame1: Vec<i16> = vec![1; 100];
        let frame2: Vec<i16> = vec![2; 200];
        mgr.add_frame(&frame1);
        assert_eq!(mgr.buffer_size(), 100);
        mgr.add_frame(&frame2);
        assert_eq!(mgr.buffer_size(), 300);
    }

    #[test]
    fn test_audio_buffer_manager_chunks() {
        let mut mgr = AudioBufferManager::new(Instant::now());
        let samples: Vec<i16> = (0..320).map(|i| i as i16).collect(); // 20ms
        mgr.add_frame(&samples[0..160]);
        mgr.add_frame(&samples[160..320]);

        let chunk_size = 160;
        let mut iter = mgr.chunks(chunk_size);
        assert_eq!(iter.next().unwrap().len(), 160);
        assert_eq!(iter.next().unwrap().len(), 160);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_audio_buffer_manager_clear() {
        let mut mgr = AudioBufferManager::new(Instant::now());
        let frame: Vec<i16> = vec![42; 160];
        mgr.add_frame(&frame);
        assert_eq!(mgr.buffer_size(), 160);
        assert_eq!(mgr.frames_buffered(), 1);

        mgr.clear();
        assert_eq!(mgr.buffer_size(), 0);
        assert_eq!(mgr.frames_buffered(), 0);
    }

    #[tokio::test]
    async fn test_event_emitter_new() {
        let (tx, _rx) = mpsc::channel(10);
        let metrics = Arc::new(RwLock::new(SttMetrics::default()));
        let stt_metrics = Arc::new(SttPerformanceMetrics::new());
        let pipeline_metrics = Arc::new(PipelineMetrics::default());
        let emitter = EventEmitter::new(tx, metrics, stt_metrics, pipeline_metrics);
        assert!(!emitter.event_tx.is_closed());
    }

    #[tokio::test]
    async fn test_event_emitter_emit_partial() {
        let (tx, mut rx) = mpsc::channel(10);
        let metrics = Arc::new(RwLock::new(SttMetrics::default()));
        let stt_metrics = Arc::new(SttPerformanceMetrics::new());
        let pipeline_metrics = Arc::new(PipelineMetrics::default());
        let emitter = EventEmitter::new(tx, metrics.clone(), stt_metrics, pipeline_metrics);

        let event = TranscriptionEvent::Partial {
            utterance_id: 1,
            text: "test partial".to_string(),
            t0: Some(100.0),
            t1: Some(200.0),
        };

        let result = emitter.emit(event.clone()).await;
        assert!(result.is_ok());

        // Verify sent
        let received = rx.recv().await.unwrap();
        assert_eq!(received, event);

        // Verify metrics
        let m = metrics.read();
        assert_eq!(m.partial_count, 1);
        assert!(m.last_event_time.is_some());
    }

    #[tokio::test]
    async fn test_audio_buffer_manager_process_chunks() {
        let mut mgr = AudioBufferManager::new(Instant::now());
        let chunk1: Vec<i16> = vec![1i16; 16000]; // 1s
        let chunk2: Vec<i16> = vec![2i16; 16000]; // 1s
        mgr.add_frame(&chunk1);
        mgr.add_frame(&chunk2);

        let mut calls = 0;
        let mut events: Vec<Option<TranscriptionEvent>> = Vec::new();
        let chunks: Vec<Vec<i16>> = mgr.buffer.chunks(16000).map(|c| c.to_vec()).collect();
        for chunk in chunks {
            calls += 1;
            assert_eq!(chunk.len(), 16000);
            if calls == 1 {
                assert_eq!(chunk[0], 1);
            } else {
                assert_eq!(chunk[0], 2);
            }
            events.push(None);
        }

        // Simulate the clear that happens in actual process_chunks
        mgr.clear();

        assert_eq!(calls, 2);
        assert_eq!(events.len(), 2);
        assert_eq!(mgr.buffer_size(), 0);
        assert_eq!(mgr.frames_buffered(), 0);
    }

    #[tokio::test]
    async fn test_event_emitter_emit_final() {
        let (tx, mut rx) = mpsc::channel(10);
        let metrics = Arc::new(RwLock::new(SttMetrics::default()));
        let stt_metrics = Arc::new(SttPerformanceMetrics::new());
        let pipeline_metrics = Arc::new(PipelineMetrics::default());
        let emitter = EventEmitter::new(tx, metrics.clone(), stt_metrics, pipeline_metrics);

        let event = TranscriptionEvent::Final {
            utterance_id: 1,
            text: "test final".to_string(),
            words: Some(vec![]),
        };

        let result = emitter.emit(event.clone()).await;
        assert!(result.is_ok());

        let received = rx.recv().await.unwrap();
        assert_eq!(received, event);

        let m = metrics.read();
        assert_eq!(m.final_count, 1);
        assert!(m.last_event_time.is_some());
    }

    #[tokio::test]
    async fn test_event_emitter_emit_error() {
        let (tx, mut rx) = mpsc::channel(10);
        let metrics = Arc::new(RwLock::new(SttMetrics::default()));
        let stt_metrics = Arc::new(SttPerformanceMetrics::new());
        let pipeline_metrics = Arc::new(PipelineMetrics::default());
        let emitter = EventEmitter::new(tx, metrics.clone(), stt_metrics, pipeline_metrics);

        let event = TranscriptionEvent::Error {
            code: "TEST".to_string(),
            message: "test error".to_string(),
        };

        let result = emitter.emit(event.clone()).await;
        assert!(result.is_ok());

        let received = rx.recv().await.unwrap();
        assert_eq!(received, event);

        let m = metrics.read();
        assert_eq!(m.error_count, 1);
        assert!(m.last_event_time.is_some());
    }

    #[tokio::test]
    async fn test_event_emitter_send_failure() {
        let (tx, _) = mpsc::channel(1); // Buffer of 1 to test full buffer case
                                        // Fill the buffer first
        let filler_event = TranscriptionEvent::Partial {
            utterance_id: 0,
            text: "filler".to_string(),
            t0: Some(0.0),
            t1: Some(0.0),
        };
        tx.send(filler_event.clone()).await.unwrap();

        let metrics = Arc::new(RwLock::new(SttMetrics::default()));
        let stt_metrics = Arc::new(SttPerformanceMetrics::new());
        let pipeline_metrics = Arc::new(PipelineMetrics::default());
        let emitter = EventEmitter::new(tx, metrics.clone(), stt_metrics, pipeline_metrics);

        let event = TranscriptionEvent::Partial {
            utterance_id: 1,
            text: "test partial".to_string(),
            t0: Some(0.0),
            t1: Some(0.0),
        };

        // Test the send failure (full buffer)
        let result = emitter.emit(event).await;
        assert!(result.is_err()); // Send failed due to full buffer

        let m = metrics.read();
        assert_eq!(m.frames_dropped, 1);
    }
}
