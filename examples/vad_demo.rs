use coldvox_app::audio::{AudioFrame, VadProcessor};
use coldvox_app::vad::{UnifiedVadConfig, VadEvent, VadMode};
use dasp::interpolate::sinc::Sinc;
use dasp::{ring_buffer, signal, Signal};
use hound::WavReader;
use tokio::sync::{broadcast, mpsc};
use tokio::time::{sleep, Duration, Instant};
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    info!("Starting VAD demo from WAV file");

    let (audio_tx, _) = broadcast::channel::<AudioFrame>(100);
    let audio_rx = audio_tx.subscribe();
    let (event_tx, mut event_rx) = mpsc::channel::<VadEvent>(100);

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
            vad_config.level3.enabled = true; // Enable Level3 for testing
            VadMode::Level3
        }
        _ => {
            info!("Unknown mode '{}'", mode);
            VadMode::Silero
        }
    };

    let vad_handle = VadProcessor::spawn(vad_config.clone(), audio_rx, event_tx, None)
        .expect("failed to spawn VAD");

    // generator task: feed WAV into broadcast
    let gen_tx = audio_tx.clone();
    let audio_file_path = std::env::var("VAD_TEST_FILE")
        .unwrap_or_else(|_| "crates/app/test_audio_16k.wav".to_string());
    let frame_size = vad_config.frame_size_samples;
    let generator = tokio::spawn(async move {
        if let Err(e) = generate_audio_from_wav(gen_tx, frame_size, &audio_file_path).await {
            error!("Audio generator failed: {}", e);
        }
    });

    // event printer
    let event_printer = tokio::spawn(async move {
        handle_vad_events(&mut event_rx).await;
    });

    generator.await.ok();
    sleep(Duration::from_secs(2)).await;
    vad_handle.abort();
    event_printer.abort();
    info!("Demo completed");
    Ok(())
}

async fn generate_audio_from_wav(
    tx: broadcast::Sender<AudioFrame>,
    frame_size: usize,
    file_path: &str,
) -> Result<(), String> {
    info!("Reading audio from: {}", file_path);
    let mut reader =
        WavReader::open(file_path).map_err(|e| format!("Failed to open WAV file: {}", e))?;
    let spec = reader.spec();
    info!(
        "WAV spec: {:?}, duration: {}ms",
        spec,
        reader.duration() as f32 / spec.sample_rate as f32 * 1000.0
    );

    let samples_f32: Vec<f32> = reader
        .samples::<i16>()
        .map(|s| s.unwrap() as f32 / i16::MAX as f32)
        .collect();

    let source_signal = signal::from_iter(samples_f32.into_iter().map(|s| [s]));

    let sinc = Sinc::new(ring_buffer::Fixed::from(vec![[0.0]; 128]));

    let original_rate = spec.sample_rate as f64;
    let new_rate = 16000.0;
    let mut converter =
        signal::interpolate::Converter::from_hz_to_hz(source_signal, sinc, original_rate, new_rate);

    let mut timestamp_ms = 0u64;
    let frame_duration_ms = (frame_size as f32 * 1000.0 / 16000.0) as u64;

    while !converter.is_exhausted() {
        let mut frame_f32 = Vec::with_capacity(frame_size);
        for _ in 0..frame_size {
            let frame = converter.next();
            frame_f32.push(frame[0]);
        }

        let frame_i16: Vec<i16> = frame_f32
            .iter()
            .map(|&s| (s * i16::MAX as f32) as i16)
            .collect();

        let mut frame = frame_i16;
        if frame.len() < frame_size {
            frame.resize(frame_size, 0);
        }

        let audio_frame = AudioFrame {
            data: frame,
            timestamp_ms,
        };

        let _ = tx.send(audio_frame);

        timestamp_ms += frame_duration_ms;
        sleep(Duration::from_millis(frame_duration_ms)).await;
    }

    info!("Audio generator stopped");
    Ok(())
}
async fn handle_vad_events(rx: &mut mpsc::Receiver<VadEvent>) {
    let start = Instant::now();
    let mut speech_segments = 0u64;
    let mut total_speech_ms = 0u64;

    while let Some(event) = rx.recv().await {
        match event {
            VadEvent::SpeechStart {
                timestamp_ms,
                energy_db,
            } => {
                speech_segments += 1;
                info!(
                    "[{:6.2}s] Speech START - Energy: {:.2} dB",
                    timestamp_ms as f32 / 1000.0,
                    energy_db
                );
            }
            VadEvent::SpeechEnd {
                timestamp_ms,
                duration_ms,
                energy_db,
            } => {
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
