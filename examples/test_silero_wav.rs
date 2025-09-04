use coldvox_app::audio::vad_adapter::VadAdapter;
use coldvox_vad::{UnifiedVadConfig, VadMode, FRAME_SIZE_SAMPLES, SAMPLE_RATE_HZ};
use hound::WavReader;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let wav_path = std::env::var("TEST_WAV").unwrap_or_else(|_| "test_audio_16k.wav".to_string());

    println!("Loading WAV: {}", wav_path);

    // Load WAV file directly
    let mut reader = WavReader::open(&wav_path)?;
    let spec = reader.spec();

    println!(
        "WAV spec: {} Hz, {} channels, {} bits",
        spec.sample_rate, spec.channels, spec.bits_per_sample
    );

    // Read all samples
    let samples: Vec<i16> = reader.samples::<i16>().collect::<Result<Vec<_>, _>>()?;

    println!("Loaded {} samples", samples.len());

    // Convert to mono if stereo
    let mono_samples = if spec.channels == 2 {
        println!("Converting stereo to mono...");
        samples
            .chunks(2)
            .map(|chunk| ((chunk[0] as i32 + chunk[1] as i32) / 2) as i16)
            .collect()
    } else {
        samples
    };

    println!("Mono samples: {}", mono_samples.len());

    // Ensure we have 16kHz audio
    let samples_16k = if spec.sample_rate != 16000 {
        println!("Resampling from {} Hz to 16000 Hz...", spec.sample_rate);
        // Simple resampling (not high quality, but good enough for testing)
        let ratio = 16000.0 / spec.sample_rate as f32;
        let new_len = (mono_samples.len() as f32 * ratio) as usize;
        let mut resampled = Vec::with_capacity(new_len);

        for i in 0..new_len {
            let src_idx = i as f32 / ratio;
            let idx = src_idx as usize;
            if idx < mono_samples.len() {
                resampled.push(mono_samples[idx]);
            }
        }
        resampled
    } else {
        mono_samples
    };

    println!("Final samples: {} at 16kHz", samples_16k.len());

    // Configure VAD
    let mut config = UnifiedVadConfig::default();
    config.mode = VadMode::Silero;
    config.silero.threshold = 0.2; // Lower threshold for testing
    config.frame_size_samples = FRAME_SIZE_SAMPLES;
    config.sample_rate_hz = SAMPLE_RATE_HZ;

    println!("\nVAD Config:");
    println!("  Mode: Silero");
    println!("  Threshold: {}", config.silero.threshold);
    println!("  Frame size: {} samples", config.frame_size_samples);

    // Create adapter
    let mut adapter = VadAdapter::new(config)?;

    let mut events = Vec::new();
    let mut frames_processed = 0;

    println!("\nProcessing audio...");

    // Process frames
    for (i, chunk) in samples_16k.chunks(FRAME_SIZE_SAMPLES).enumerate() {
        if chunk.len() == FRAME_SIZE_SAMPLES {
            match adapter.process(chunk) {
                Ok(Some(event)) => {
                    let time_ms = i * FRAME_SIZE_SAMPLES * 1000 / SAMPLE_RATE_HZ as usize;
                    println!("  [{}ms] Event: {:?}", time_ms, event);
                    events.push(event);
                }
                Ok(None) => {
                    // No event
                }
                Err(e) => {
                    eprintln!("Error at frame {}: {}", i, e);
                }
            }
            frames_processed += 1;
        }
    }

    println!("\n=== Summary ===");
    println!("Frames processed: {}", frames_processed);
    println!("Events detected: {}", events.len());

    if events.is_empty() {
        println!("\nNo speech detected. Possible issues:");
        println!("  - Threshold too high (current: {})", 0.2);
        println!("  - Audio file contains no speech");
        println!("  - Frame buffering issue in adapter");
    }

    Ok(())
}
