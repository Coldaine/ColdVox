use coldvox_app::audio::vad_adapter::VadAdapter;
use coldvox_vad::config::SileroConfig;
use coldvox_vad::{UnifiedVadConfig, VadMode, FRAME_SIZE_SAMPLES, SAMPLE_RATE_HZ};
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Determine WAV paths: prefer CLI args, else TEST_WAV env. No hardcoded fallbacks.
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    let wavs: Vec<String> = if !args.is_empty() {
        args.drain(..).collect()
    } else if let Ok(v) = env::var("TEST_WAV") {
        // Allow colon- or comma-separated list in env
        if v.contains(',') {
            v.split(',').map(|s| s.trim().to_string()).collect()
        } else if v.contains(':') {
            v.split(':').map(|s| s.trim().to_string()).collect()
        } else {
            vec![v]
        }
    } else {
        eprintln!("Usage: test_silero_wav <path1.wav> [path2.wav ...]  (or set TEST_WAV)");
        eprintln!("No WAV path provided. This example does not use synthetic audio.");
        std::process::exit(2);
    };

    let threshold: f32 = env::var("VAD_THRESHOLD")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.2);

    for wav_path in wavs {
        let mut reader = match hound::WavReader::open(&wav_path) {
            Ok(r) => {
                println!("\nLoading WAV: {}", wav_path);
                r
            }
            Err(e) => {
                eprintln!("Failed to open {}: {}", wav_path, e);
                continue;
            }
        };
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

        // Configure VAD using provided threshold and defaults
        let config = UnifiedVadConfig {
            mode: VadMode::Silero,
            silero: SileroConfig {
                threshold,
                ..Default::default()
            },
            frame_size_samples: FRAME_SIZE_SAMPLES,
            sample_rate_hz: SAMPLE_RATE_HZ,
        };

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

        println!("\n=== Summary ({} ) ===", wav_path);
        println!("Frames processed: {}", frames_processed);
        println!("Events detected: {}", events.len());

        if events.is_empty() {
            println!("\nNo speech detected. Possible issues:");
            println!("  - Threshold too high (current: {})", 0.2);
            println!("  - Audio file contains no speech");
            println!("  - Frame buffering issue in adapter");
        }
    }

    Ok(())
}
