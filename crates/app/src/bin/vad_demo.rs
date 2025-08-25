use coldvox_app::audio::vad_processor::{AudioFrame, VadProcessor};
use coldvox_app::vad::config::{UnifiedVadConfig, VadMode};
use coldvox_app::vad::types::VadEvent;
use crossbeam_channel::{bounded, Sender};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};
use tracing::{info, error};
use hound::WavReader;
use dasp::{signal, ring_buffer, Frame, Signal};
use dasp::interpolate::sinc::Sinc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();
    
    info!("Starting VAD demo from WAV file");
    
    let (audio_tx, audio_rx) = bounded::<AudioFrame>(100);
    let (event_tx, event_rx) = bounded::<VadEvent>(100);
    let shutdown = Arc::new(AtomicBool::new(false));
    
    let mut vad_config = UnifiedVadConfig::default();
    
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("silero");
    
    vad_config.mode = match mode {
        "silero" => {
            info!("Using Silero VAD engine");
            if let Some(threshold_str) = args.get(2) {
                if let Ok(threshold) = threshold_str.parse::<f32>() {
                    info!("Setting Silero threshold to: {}", threshold);
                    vad_config.silero.threshold = threshold;
                }
            }
            VadMode::Silero
        }
        "level3" => {
            info!("Using Level3 VAD engine (enabling it for this demo)");
            vad_config.level3.enabled = true;  // Enable Level3 for testing
            VadMode::Level3
        }
        _ => {
            info!("Unknown mode '{}', defaulting to Silero", mode);
            VadMode::Silero
        }
    };
    
    let vad_handle = VadProcessor::spawn(
        vad_config.clone(),
        audio_rx,
        event_tx,
        shutdown.clone(),
    )?;
    
    let generator_shutdown = shutdown.clone();
    let audio_file_path = "crates/app/test_audio_16k.wav".to_string();
    let generator_handle = thread::spawn(move || {
        if let Err(e) = generate_audio_from_wav(audio_tx, generator_shutdown, vad_config.frame_size_samples, &audio_file_path) {
            error!("Audio generator failed: {}", e);
        }
    });
    
    let event_shutdown = shutdown.clone();
    let event_handle = thread::spawn(move || {
        handle_vad_events(event_rx, event_shutdown);
    });
    
    // Wait for the generator to finish, then wait a little longer for events to process
    generator_handle.join().expect("Generator thread panicked");
    thread::sleep(Duration::from_secs(2));
    
    info!("Shutting down...");
    shutdown.store(true, Ordering::Relaxed);
    
    vad_handle.join().expect("VAD thread panicked");
    event_handle.join().expect("Event handler thread panicked");
    
    info!("Demo completed");
    Ok(())
}

fn generate_audio_from_wav(
    tx: Sender<AudioFrame>,
    shutdown: Arc<AtomicBool>,
    frame_size: usize,
    file_path: &str,
) -> Result<(), String> {
    info!("Reading audio from: {}", file_path);
    let mut reader = WavReader::open(file_path).map_err(|e| format!("Failed to open WAV file: {}", e))?;
    let spec = reader.spec();
    info!("WAV spec: {:?}, duration: {}ms", spec, reader.duration() as f32 / spec.sample_rate as f32 * 1000.0);

    let samples_f32: Vec<f32> = reader.samples::<i16>().map(|s| s.unwrap() as f32 / i16::MAX as f32).collect();

    let source_signal = signal::from_iter(samples_f32.into_iter().map(|s| [s]));

    let sinc = Sinc::new(ring_buffer::Fixed::from(vec![[0.0]; 128]));

    let original_rate = spec.sample_rate as f64;
    let new_rate = 16000.0;
    let mut converter = signal::interpolate::Converter::from_hz_to_hz(
        source_signal,
        sinc,
        original_rate,
        new_rate,
    );

    let mut timestamp_ms = 0u64;
    let frame_duration_ms = (frame_size as f32 * 1000.0 / 16000.0) as u64;

    while !converter.is_exhausted() {
        if shutdown.load(Ordering::Relaxed) {
            break;
        }

        let mut frame_f32 = Vec::with_capacity(frame_size);
        for _ in 0..frame_size {
            let frame = converter.next();
            frame_f32.push(frame[0]);
        }

        let frame_i16: Vec<i16> = frame_f32.iter().map(|&s| (s * i16::MAX as f32) as i16).collect();

        let mut frame = frame_i16;
        if frame.len() < frame_size {
            frame.resize(frame_size, 0);
        }

        let audio_frame = AudioFrame {
            data: frame,
            timestamp_ms,
        };

        if let Err(e) = tx.send(audio_frame) {
            if !shutdown.load(Ordering::Relaxed) {
                error!("Failed to send audio frame: {}", e);
            }
            break;
        }

        timestamp_ms += frame_duration_ms;
        thread::sleep(Duration::from_millis(frame_duration_ms));
    }

    info!("Audio generator stopped");
    Ok(())
}

fn handle_vad_events(rx: crossbeam_channel::Receiver<VadEvent>, shutdown: Arc<AtomicBool>) {
    let start = Instant::now();
    let mut speech_segments = 0u64;
    let mut total_speech_ms = 0u64;
    
    while !shutdown.load(Ordering::Relaxed) {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(event) => {
                match event {
                    VadEvent::SpeechStart { timestamp_ms, energy_db } => {
                        speech_segments += 1;
                        info!(
                            "[{:6.2}s] Speech START - Energy: {:.2} dB",
                            timestamp_ms as f32 / 1000.0,
                            energy_db
                        );
                    }
                    VadEvent::SpeechEnd { timestamp_ms, duration_ms, energy_db } => {
                        total_speech_ms += duration_ms;
                        info!(
                            "[{:6.2}s] Speech END   - Duration: {} ms, Energy: {:.2} dB",
                            timestamp_ms as f32 / 1000.0,
                            duration_ms,
                            energy_db
                        );
                    }
                }
            }
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => continue,
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
        }
    }
    
    let elapsed = start.elapsed();
    info!(
        "Event handler stopped. Total: {} speech segments, {:.2}s of speech in {:.2}s",
        speech_segments,
        total_speech_ms as f32 / 1000.0,
        elapsed.as_secs_f32()
    );
}

fn simple_random() -> f32 {
    use std::cell::Cell;
    use std::num::Wrapping;
    
    thread_local! {
        static SEED: Cell<Wrapping<u32>> = Cell::new(Wrapping(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as u32
        ));
    }
    
    SEED.with(|seed| {
        let mut s = seed.get();
        s = s * Wrapping(1103515245) + Wrapping(12345);
        seed.set(s);
        (s.0 >> 16) as f32 / 65536.0
    })
}
