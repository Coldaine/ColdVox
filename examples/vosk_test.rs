use coldvox_app::stt::{Transcriber, VoskTranscriber};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Test with a small Vosk model (download required)
    let model_path = "models/vosk-model-small-en-us-0.15";
    
    if !Path::new(model_path).exists() {
        eprintln!("Vosk model not found at: {}", model_path);
        eprintln!("Download a model from https://alphacephei.com/vosk/models");
        eprintln!("Extract to: {}", model_path);
        return Ok(());
    }
    
    println!("Loading Vosk model from: {}", model_path);
    let mut transcriber = VoskTranscriber::new(model_path, 16000.0)?;
    
    // Generate test audio: sine wave representing speech-like patterns
    let sample_rate = 16000;
    let duration_ms = 1000; // 1 second
    let samples_count = (sample_rate * duration_ms) / 1000;
    
    let mut test_audio = Vec::with_capacity(samples_count);
    for i in 0..samples_count {
        let t = i as f32 / sample_rate as f32;
        // Mix of frequencies to simulate speech
        let sample = (
            0.3 * (2.0 * std::f32::consts::PI * 440.0 * t).sin() +
            0.2 * (2.0 * std::f32::consts::PI * 880.0 * t).sin() +
            0.1 * (2.0 * std::f32::consts::PI * 1320.0 * t).sin()
        ) * 16384.0; // Scale to i16 range
        
        test_audio.push(sample as i16);
    }
    
    println!("Processing {} samples of synthetic audio...", test_audio.len());
    
    // Process audio in chunks (512 samples = 32ms at 16kHz)
    let chunk_size = 512;
    let mut partial_count = 0;
    let mut result_count = 0;
    
    for chunk in test_audio.chunks(chunk_size) {
        match transcriber.accept_pcm16(chunk)? {
            Some(text) if text.starts_with("[partial]") => {
                partial_count += 1;
                println!("Partial result {}: {}", partial_count, text);
            }
            Some(text) => {
                result_count += 1;
                println!("Final result {}: {}", result_count, text);
            }
            None => {
                // No transcription for this chunk
            }
        }
    }
    
    // Get final result
    if let Some(final_text) = transcriber.finalize()? {
        println!("Final transcription: {}", final_text);
    } else {
        println!("No final transcription (synthetic audio not recognized as speech)");
    }
    
    println!("Test completed. Partial results: {}, Final results: {}", partial_count, result_count);
    println!("Note: Synthetic audio may not produce meaningful transcriptions");
    
    Ok(())
}