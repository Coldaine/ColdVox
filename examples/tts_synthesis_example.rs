//! Simple TTS synthesis example

use coldvox_tts::{TtsEngine, TtsConfig, SynthesisOptions};
use coldvox_tts_espeak::EspeakEngine;
use tokio::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    println!("ColdVox TTS Synthesis Example");
    println!("==============================");
    
    // Create and initialize espeak engine
    let mut engine = EspeakEngine::new();
    
    println!("Engine: {} v{}", engine.name(), engine.version());
    
    // Check if espeak is available
    if !engine.is_available().await {
        eprintln!("Error: eSpeak is not available on this system");
        eprintln!("Please install espeak or espeak-ng");
        return Ok(());
    }
    
    println!("✓ eSpeak is available");
    
    // Initialize with default config
    let config = TtsConfig::default();
    engine.initialize(config).await?;
    println!("✓ Engine initialized");
    
    // List available voices
    let voices = engine.list_voices().await?;
    println!("\nAvailable voices ({}):", voices.len());
    for voice in &voices[..std::cmp::min(5, voices.len())] {
        println!("  - {} ({})", voice.name, voice.id);
    }
    if voices.len() > 5 {
        println!("  ... and {} more", voices.len() - 5);
    }
    
    // Synthesize some text
    println!("\nSynthesizing text...");
    let text = "Hello from ColdVox TTS synthesis. This is a test of the text to speech functionality.";
    
    match engine.synthesize(text, None).await? {
        coldvox_tts::SynthesisEvent::AudioData { synthesis_id, data, sample_rate, channels } => {
            println!("✓ Synthesis successful!");
            println!("  Synthesis ID: {}", synthesis_id);
            println!("  Audio data size: {} bytes", data.len());
            println!("  Sample rate: {} Hz", sample_rate);
            println!("  Channels: {}", channels);
            
            // Save to file for testing
            let filename = "tts_output.wav";
            fs::write(filename, &data).await?;
            println!("  Saved to: {}", filename);
            println!("  You can play it with: aplay {} (or your preferred audio player)", filename);
        }
        coldvox_tts::SynthesisEvent::Failed { synthesis_id, error } => {
            eprintln!("✗ Synthesis failed (ID: {}): {}", synthesis_id, error);
        }
        event => {
            println!("Unexpected event: {:?}", event);
        }
    }
    
    // Test with custom options
    println!("\nTesting with custom options...");
    let options = SynthesisOptions {
        speech_rate: Some(120), // Slower speech
        pitch: Some(1.2),       // Higher pitch
        volume: Some(0.9),      // Louder
        ..Default::default()
    };
    
    let custom_text = "This is spoken with custom settings: slower rate, higher pitch, and louder volume.";
    match engine.synthesize(custom_text, Some(options)).await? {
        coldvox_tts::SynthesisEvent::AudioData { data, .. } => {
            let filename = "tts_output_custom.wav";
            fs::write(filename, &data).await?;
            println!("✓ Custom synthesis saved to: {}", filename);
        }
        coldvox_tts::SynthesisEvent::Failed { error, .. } => {
            eprintln!("✗ Custom synthesis failed: {}", error);
        }
        _ => {}
    }
    
    // Shutdown
    engine.shutdown().await?;
    println!("\n✓ Engine shutdown complete");
    
    Ok(())
}