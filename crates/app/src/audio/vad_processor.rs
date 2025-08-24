use crate::audio::vad_adapter::VadAdapter;
use crate::vad::{
    config::UnifiedVadConfig,
    types::VadEvent,
};
use crossbeam_channel::{Receiver, Sender};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone)]
pub struct AudioFrame {
    pub data: Vec<i16>,
    pub timestamp_ms: u64,
}

pub struct VadProcessor {
    adapter: VadAdapter,
    audio_rx: Receiver<AudioFrame>,
    event_tx: Sender<VadEvent>,
    shutdown: Arc<AtomicBool>,
    frames_processed: u64,
    events_generated: u64,
}

impl VadProcessor {
    pub fn new(
        config: UnifiedVadConfig,
        audio_rx: Receiver<AudioFrame>,
        event_tx: Sender<VadEvent>,
        shutdown: Arc<AtomicBool>,
    ) -> Result<Self, String> {
        let adapter = VadAdapter::new(config)?;
        
        Ok(Self {
            adapter,
            audio_rx,
            event_tx,
            shutdown,
            frames_processed: 0,
            events_generated: 0,
        })
    }
    
    pub fn run(mut self) {
        info!("VAD processor thread started");
        
        while !self.shutdown.load(Ordering::Relaxed) {
            match self.audio_rx.recv_timeout(Duration::from_millis(100)) {
                Ok(frame) => {
                    self.process_frame(frame);
                }
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                    continue;
                }
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                    warn!("Audio channel disconnected, VAD processor shutting down");
                    break;
                }
            }
        }
        
        info!(
            "VAD processor thread shutting down. Frames processed: {}, Events generated: {}",
            self.frames_processed, self.events_generated
        );
    }
    
    fn process_frame(&mut self, frame: AudioFrame) {
        match self.adapter.process(&frame.data) {
            Ok(Some(event)) => {
                self.events_generated += 1;
                
                match &event {
                    VadEvent::SpeechStart { timestamp_ms, energy_db } => {
                        debug!(
                            "Speech started at {}ms with energy {:.2}dB",
                            timestamp_ms, energy_db
                        );
                    }
                    VadEvent::SpeechEnd { timestamp_ms, duration_ms, energy_db } => {
                        debug!(
                            "Speech ended at {}ms (duration {}ms) with energy {:.2}dB",
                            timestamp_ms, duration_ms, energy_db
                        );
                    }
                }
                
                if let Err(e) = self.event_tx.send(event) {
                    error!("Failed to send VAD event: {}", e);
                }
            }
            Ok(None) => {
            }
            Err(e) => {
                error!("VAD processing error: {}", e);
            }
        }
        
        self.frames_processed += 1;
        
        if self.frames_processed % 1000 == 0 {
            debug!(
                "VAD processor: {} frames processed, {} events generated, current state: {:?}",
                self.frames_processed,
                self.events_generated,
                self.adapter.current_state()
            );
        }
    }
    
    pub fn spawn(
        config: UnifiedVadConfig,
        audio_rx: Receiver<AudioFrame>,
        event_tx: Sender<VadEvent>,
        shutdown: Arc<AtomicBool>,
    ) -> Result<thread::JoinHandle<()>, String> {
        let processor = VadProcessor::new(config, audio_rx, event_tx, shutdown)?;
        
        let handle = thread::Builder::new()
            .name("vad-processor".to_string())
            .spawn(move || processor.run())
            .map_err(|e| format!("Failed to spawn VAD processor thread: {}", e))?;
        
        Ok(handle)
    }
}

pub struct VadProcessorBuilder {
    config: Option<UnifiedVadConfig>,
    audio_rx: Option<Receiver<AudioFrame>>,
    event_tx: Option<Sender<VadEvent>>,
    shutdown: Option<Arc<AtomicBool>>,
}

impl VadProcessorBuilder {
    pub fn new() -> Self {
        Self {
            config: None,
            audio_rx: None,
            event_tx: None,
            shutdown: None,
        }
    }
    
    pub fn config(mut self, config: UnifiedVadConfig) -> Self {
        self.config = Some(config);
        self
    }
    
    pub fn audio_receiver(mut self, rx: Receiver<AudioFrame>) -> Self {
        self.audio_rx = Some(rx);
        self
    }
    
    pub fn event_sender(mut self, tx: Sender<VadEvent>) -> Self {
        self.event_tx = Some(tx);
        self
    }
    
    pub fn shutdown_signal(mut self, shutdown: Arc<AtomicBool>) -> Self {
        self.shutdown = Some(shutdown);
        self
    }
    
    pub fn build(self) -> Result<VadProcessor, String> {
        let config = self.config.ok_or("VAD config not provided")?;
        let audio_rx = self.audio_rx.ok_or("Audio receiver not provided")?;
        let event_tx = self.event_tx.ok_or("Event sender not provided")?;
        let shutdown = self.shutdown.ok_or("Shutdown signal not provided")?;
        
        VadProcessor::new(config, audio_rx, event_tx, shutdown)
    }
    
    pub fn spawn(self) -> Result<thread::JoinHandle<()>, String> {
        let config = self.config.ok_or("VAD config not provided")?;
        let audio_rx = self.audio_rx.ok_or("Audio receiver not provided")?;
        let event_tx = self.event_tx.ok_or("Event sender not provided")?;
        let shutdown = self.shutdown.ok_or("Shutdown signal not provided")?;
        
        VadProcessor::spawn(config, audio_rx, event_tx, shutdown)
    }
}