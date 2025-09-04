use coldvox_vad::{
    config::{UnifiedVadConfig, VadMode},
    constants::{FRAME_SIZE_SAMPLES, SAMPLE_RATE_HZ},
    silero_wrapper::SileroEngine,
    engine::VadEngine,
};
use coldvox_audio::vad_adapter::VadAdapter;
use hound::WavReader;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Simple Silero VAD test");

    // Load WAV file
    let wav_path = "test_audio_16k.wav";
    let mut reader = WavReader::open(wav_path)?;
    let spec = reader.spec();
    println!("WAV: {} Hz, {} channels", spec.sample_rate, spec.channels);

    // Collect all samples
    let samples: Vec<i16> = reader.samples::<i16>()
        .collect::<Result<Vec<_>, _>>()?;

    // Convert to mono if needed
    let mono_samples: Vec<i16> = if spec.channels == 2 {
        samples.chunks(2)
            .map(|chunk| ((chunk[0] as i32 + chunk[1] as i32) / 2) as i16)
            .collect()
    } else {
        samples
    };

    println!("Loaded {} samples", mono_samples.len());

    // Configure VAD
    let mut config = UnifiedVadConfig::default();
    config.mode = VadMode::Silero;
    config.silero.threshold = 0.3;
    config.frame_size_samples = FRAME_SIZE_SAMPLES;  // Now uses 512 directly
    config.sample_rate_hz = SAMPLE_RATE_HZ;

    // Create adapter (direct 512-sample processing)
    let mut adapter = VadAdapter::new(config)?;

    let mut speech_frames = 0;
    let mut total_frames = 0;

    // Process in 512-sample frames (~32ms)
    for (i, chunk) in mono_samples.chunks(FRAME_SIZE_SAMPLES).enumerate() {
        if chunk.len() == FRAME_SIZE_SAMPLES {
            match adapter.process(chunk) {
                Ok(Some(event)) => {
                    println!("Frame {}: Event: {:?}", i, event);
                    speech_frames += 1;
                }
                Ok(None) => {
                    // No event, frame buffered or silence
                }
                Err(e) => {
                    eprintln!("Error at frame {}: {}", i, e);
                }
            }
            total_frames += 1;
        }
    }

    println!("\nProcessed {} frames, {} events", total_frames, speech_frames);

    Ok(())
}
