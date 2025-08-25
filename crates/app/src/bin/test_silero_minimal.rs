use coldvox_app::vad::{
    config::{UnifiedVadConfig, VadMode},
    engine::VadEngine,
    silero_wrapper::SileroEngine,
};
use coldvox_app::audio::vad_adapter::VadAdapter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing Silero VAD with simple audio");
    
    // Create a simple test signal: silence, then tone, then silence
    let mut samples = Vec::new();
    
    // 1 second of silence
    for _ in 0..16000 {
        samples.push(0i16);
    }
    
    // 2 seconds of speech-like signal (varying amplitude)
    for i in 0..32000 {
        let t = i as f32 / 16000.0;
        let amplitude = (4000.0 * (t * 2.0).sin()) as i16;
        samples.push(amplitude);
    }
    
    // 1 second of silence
    for _ in 0..16000 {
        samples.push(0i16);
    }
    
    println!("Created {} samples ({} seconds)", samples.len(), samples.len() / 16000);
    
    // Configure VAD for Silero
    let mut config = UnifiedVadConfig::default();
    config.mode = VadMode::Silero;
    config.silero.threshold = 0.3;
    config.frame_size_samples = 320;  // System default
    config.sample_rate_hz = 16000;
    
    // Create adapter
    let mut adapter = VadAdapter::new(config)?;
    
    let mut events = Vec::new();
    let mut frames_processed = 0;
    
    // Process in 320-sample frames
    for (i, chunk) in samples.chunks(320).enumerate() {
        if chunk.len() == 320 {
            match adapter.process(chunk) {
                Ok(Some(event)) => {
                    println!("Frame {}: Event: {:?}", i, event);
                    events.push((i, event));
                }
                Ok(None) => {
                    // Frame buffered or no event
                }
                Err(e) => {
                    eprintln!("Error at frame {}: {}", i, e);
                }
            }
            frames_processed += 1;
            
            if frames_processed % 50 == 0 {
                println!("Processed {} frames...", frames_processed);
            }
        }
    }
    
    println!("\nSummary:");
    println!("Total frames: {}", frames_processed);
    println!("Events detected: {}", events.len());
    for (frame_num, event) in &events {
        println!("  Frame {}: {:?}", frame_num, event);
    }
    
    Ok(())
}