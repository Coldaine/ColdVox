use coldvox_app::vad::{
    config::{UnifiedVadConfig, VadMode},
    silero_wrapper::SileroEngine,
    engine::VadEngine,
};
use coldvox_app::audio::vad_adapter::VadAdapter;
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
    config.frame_size_samples = 320;  // System uses 320
    config.sample_rate_hz = 16000;
    
    // Create adapter (it will handle 320->512 conversion)
    let mut adapter = VadAdapter::new(config)?;
    
    let mut speech_frames = 0;
    let mut total_frames = 0;
    
    // Process in 320-sample frames
    for (i, chunk) in mono_samples.chunks(320).enumerate() {
        if chunk.len() == 320 {
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