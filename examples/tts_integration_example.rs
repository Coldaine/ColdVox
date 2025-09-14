//! TTS integration example for ColdVox
//!
//! This example demonstrates how TTS synthesis can work with transcription events

use coldvox_stt::TranscriptionEvent;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use tracing_subscriber;

#[cfg(feature = "tts-espeak")]
use coldvox_app::tts::{TtsProcessor, TtsIntegrationConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("ColdVox TTS Integration Example");
    println!("===============================");
    
    #[cfg(not(feature = "tts-espeak"))]
    {
        println!("TTS features not enabled. Run with --features tts-espeak");
        return Ok(());
    }
    
    #[cfg(feature = "tts-espeak")]
    {
        // Create a channel for transcription events
        let (tx, rx) = mpsc::channel::<TranscriptionEvent>(100);
        
        // Configure TTS
        let mut integration_config = TtsIntegrationConfig::default();
        integration_config.announce_final_transcriptions = true;
        integration_config.save_audio_files = true;
        
        // Create TTS processor
        let tts_processor = TtsProcessor::new_with_espeak(
            integration_config.tts_config.clone(),
            rx,
        ).await?;
        
        println!("✓ TTS processor created");
        
        // Start TTS processor in background
        let tts_handle = tokio::spawn(async move {
            tts_processor.run().await;
        });
        
        println!("✓ TTS processor started");
        
        // Simulate some transcription events
        let transcriptions = vec![
            "Hello from ColdVox TTS integration",
            "This is a test of the text to speech synthesis system",
            "The quick brown fox jumps over the lazy dog",
            "ColdVox now supports both speech to text and text to speech",
        ];
        
        for (i, text) in transcriptions.iter().enumerate() {
            let utterance_id = (i + 1) as u64;
            
            println!("Sending transcription event {}: {}", utterance_id, text);
            
            let event = TranscriptionEvent::Final {
                utterance_id,
                text: text.to_string(),
                words: None,
            };
            
            if tx.send(event).await.is_err() {
                eprintln!("Failed to send transcription event");
                break;
            }
            
            // Wait a bit between events
            sleep(Duration::from_secs(3)).await;
        }
        
        // Send an error event to test error handling
        let error_event = TranscriptionEvent::Error {
            code: "TEST_ERROR".to_string(),
            message: "This is a test error message".to_string(),
        };
        
        println!("Sending error event");
        let _ = tx.send(error_event).await;
        
        // Close the channel and wait for TTS processor to finish
        drop(tx);
        
        println!("Waiting for TTS processor to finish...");
        if let Err(e) = tts_handle.await {
            eprintln!("TTS processor error: {}", e);
        }
        
        println!("✓ TTS integration example completed");
        println!("Check /tmp/ for generated TTS audio files (*.wav)");
    }
    
    Ok(())
}