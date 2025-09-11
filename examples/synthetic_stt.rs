//! Synthetic STT test harness for headless validation
//!
//! This example feeds pre-recorded PCM data into the STT plugin manager
//! to validate the entire pipeline without requiring a microphone.

use coldvox_stt::{
    plugin::{PluginSelectionConfig, SttPlugin},
    plugin_adapter::PluginAdapter,
    types::{TranscriptionConfig, TranscriptionEvent},
    plugins::{MockPlugin, NoOpPlugin}
};

#[cfg(feature = "whisper")]
use coldvox_stt::plugins::WhisperPlugin;

#[cfg(feature = "vosk")]
use coldvox_stt::plugins::VoskPlugin;

use clap::Parser;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::time::sleep;

#[derive(Parser)]
#[command(name = "synthetic_stt")]
#[command(about = "Test STT plugins with synthetic audio data")]
struct Args {
    /// STT backend to test
    #[arg(long, default_value = "mock")]
    backend: String,

    /// Path to model file (for whisper/vosk)
    #[arg(long)]
    model_path: Option<PathBuf>,

    /// Duration of synthetic audio in seconds
    #[arg(long, default_value = "5")]
    duration: u64,

    /// Sample rate for synthetic audio
    #[arg(long, default_value = "16000")]
    sample_rate: u32,

    /// Test all available backends
    #[arg(long)]
    test_all: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    if args.test_all {
        test_all_backends(&args).await?;
    } else {
        test_single_backend(&args.backend, &args).await?;
    }

    Ok(())
}

async fn test_all_backends(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing all available STT backends...\n");

    let backends = vec![
        ("mock", true),
        ("noop", true),
        #[cfg(feature = "vosk")]
        ("vosk", args.model_path.is_some()),
        #[cfg(feature = "whisper")]
        ("whisper", args.model_path.is_some()),
    ];

    for (backend, should_test) in backends {
        if should_test {
            println!("=== Testing {} backend ===", backend);
            match test_single_backend(backend, args).await {
                Ok(_) => println!("✓ {} backend test passed\n", backend),
                Err(e) => println!("✗ {} backend test failed: {}\n", backend, e),
            }
        } else {
            println!("⚠ Skipping {} backend (no model configured)\n", backend);
        }
    }

    Ok(())
}

async fn test_single_backend(backend: &str, args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating {} plugin...", backend);

    // Create plugin based on backend type
    let plugin: Box<dyn SttPlugin> = match backend {
        "mock" => Box::new(MockPlugin::new()),
        "noop" => Box::new(NoOpPlugin::new()),
        #[cfg(feature = "vosk")]
        "vosk" => {
            let mut plugin = VoskPlugin::new();
            if let Some(model_path) = &args.model_path {
                plugin.load_model(Some(model_path)).await?;
            }
            Box::new(plugin)
        },
        #[cfg(feature = "whisper")]
        "whisper" => {
            let mut plugin = WhisperPlugin::new();
            if let Some(model_path) = &args.model_path {
                plugin.load_model(Some(model_path)).await?;
            }
            Box::new(plugin)
        },
        _ => return Err(format!("Unknown backend: {}", backend).into()),
    };

    // Check availability
    if !plugin.is_available().await? {
        return Err(format!("Backend {} is not available", backend).into());
    }

    println!("Plugin info: {:?}", plugin.info());
    println!("Plugin capabilities: {:?}", plugin.capabilities());

    // Create adapter and initialize
    let mut adapter = PluginAdapter::new(plugin);
    let config = TranscriptionConfig {
        enabled: true,
        model_path: args.model_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "test-model".to_string()),
        partial_results: true,
        max_alternatives: 1,
        include_words: false,
        buffer_size_ms: 512,
        streaming: false,
    };

    adapter.initialize(config).await
        .map_err(|e| format!("Failed to initialize adapter: {}", e))?;

    println!("Initialized {} backend successfully", backend);

    // Generate synthetic audio data
    let synthetic_audio = generate_synthetic_audio(args.duration, args.sample_rate);
    println!("Generated {} samples of synthetic audio", synthetic_audio.len());

    // Simulate speech processing
    let start_time = Instant::now();
    
    // Reset for new utterance
    adapter.reset().await;
    println!("Started processing audio...");

    // Process audio in chunks (simulate real-time processing)
    let chunk_size = 1600; // ~100ms at 16kHz
    let mut events = Vec::new();
    
    for chunk in synthetic_audio.chunks(chunk_size) {
        if let Some(event) = adapter.on_speech_frame(chunk).await {
            events.push(event);
        }
        
        // Add small delay to simulate real-time processing
        sleep(Duration::from_millis(10)).await;
    }

    // Finalize
    if let Some(final_event) = adapter.on_speech_end().await {
        events.push(final_event);
    }

    let processing_time = start_time.elapsed();
    let audio_duration = Duration::from_secs_f64(synthetic_audio.len() as f64 / args.sample_rate as f64);
    let rt_factor = processing_time.as_secs_f64() / audio_duration.as_secs_f64();

    println!("Processing completed in {:.2}s", processing_time.as_secs_f64());
    println!("Audio duration: {:.2}s", audio_duration.as_secs_f64());
    println!("Real-time factor: {:.3}", rt_factor);
    println!("Events received: {}", events.len());

    // Print transcription results
    for (i, event) in events.iter().enumerate() {
        match event {
            TranscriptionEvent::Partial { text, .. } => {
                println!("  [{}] Partial: {}", i, text);
            }
            TranscriptionEvent::Final { text, words, .. } => {
                println!("  [{}] Final: {}", i, text);
                if let Some(words) = words {
                    println!("    Words: {} with avg confidence {:.2}",
                           words.len(),
                           words.iter().map(|w| w.conf).sum::<f32>() / words.len() as f32);
                }
            }
            TranscriptionEvent::Error { code, message } => {
                println!("  [{}] Error [{}]: {}", i, code, message);
            }
        }
    }

    // Verify we got at least some output (except for NoOp)
    if backend != "noop" && events.is_empty() {
        return Err("No transcription events received".into());
    }

    println!("✓ {} backend test completed successfully", backend);
    Ok(())
}

/// Generate synthetic PCM audio data for testing
/// Creates a simple sine wave pattern that should be recognizable by STT engines
fn generate_synthetic_audio(duration_secs: u64, sample_rate: u32) -> Vec<i16> {
    let total_samples = duration_secs as usize * sample_rate as usize;
    let mut samples = Vec::with_capacity(total_samples);

    for i in 0..total_samples {
        let t = i as f32 / sample_rate as f32;
        
        // Create a complex waveform that simulates speech-like patterns
        // Mix multiple frequencies to create speech-like harmonics
        let fundamental = 200.0; // Base frequency (like voice)
        let signal = 
            0.3 * (2.0 * std::f32::consts::PI * fundamental * t).sin() +
            0.2 * (2.0 * std::f32::consts::PI * fundamental * 2.0 * t).sin() +
            0.15 * (2.0 * std::f32::consts::PI * fundamental * 3.0 * t).sin() +
            0.1 * (2.0 * std::f32::consts::PI * fundamental * 4.0 * t).sin();
        
        // Add envelope to simulate speech patterns (pauses, emphasis)
        let envelope = if (t * 2.0) % 2.0 < 1.5 { 
            1.0 
        } else { 
            0.3 // Quieter sections simulate pauses
        };
        
        // Add slight amplitude modulation for realism
        let modulation = 1.0 + 0.1 * (2.0 * std::f32::consts::PI * 8.0 * t).sin();
        
        let amplitude = signal * envelope * modulation;
        let sample = (amplitude * 16384.0) as i16; // Scale to i16 range
        samples.push(sample);
    }

    samples
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_synthetic_audio_generation() {
        let audio = generate_synthetic_audio(1, 16000);
        assert_eq!(audio.len(), 16000);
        
        // Check that we have non-zero audio
        assert!(audio.iter().any(|&sample| sample.abs() > 1000));
    }

    #[tokio::test]
    async fn test_mock_backend() {
        let args = Args {
            backend: "mock".to_string(),
            model_path: None,
            duration: 1,
            sample_rate: 16000,
            test_all: false,
        };

        assert!(test_single_backend("mock", &args).await.is_ok());
    }

    #[tokio::test]
    async fn test_noop_backend() {
        let args = Args {
            backend: "noop".to_string(),
            model_path: None,
            duration: 1,
            sample_rate: 16000,
            test_all: false,
        };

        assert!(test_single_backend("noop", &args).await.is_ok());
    }
}