use coldvox_stt::plugin::{SttPlugin, SttPluginFactory};
use coldvox_stt::plugins::moonshine::MoonshinePluginFactory;
use coldvox_stt::types::TranscriptionConfig;
use hound;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Setup minimal logging (removed tracing-subscriber dependency)
    println!("🚀 Starting Moonshine Verification...");

    // 2. Create Plugin Factory
    // This will check requirement (Python deps)
    println!("🔍 Checking requirements...");
    let factory = MoonshinePluginFactory::new();
    factory.check_requirements()?;

    // 3. Create Plugin Instance
    println!("📦 Creating plugin instance...");
    let mut plugin = factory.create()?;

    // 4. Initialize (loads model)
    println!("⏳ Initializing model (this uses PyO3 and might take a moment)...");
    let start = Instant::now();
    plugin.initialize(TranscriptionConfig::default()).await?;
    println!("✅ Model loaded in {:.2?}", start.elapsed());

    // 5. Generate Test Audio (1s of 440Hz sine wave @ 16kHz)
    println!("🎵 Generating test audio...");
    let sample_rate = 16000;
    let duration_secs = 2;
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    // We create a buffer of samples
    let mut samples = Vec::new();
    for t in (0..sample_rate * duration_secs).map(|x| x as f32 / sample_rate as f32) {
        let sample = (t * 440.0 * 2.0 * std::f32::consts::PI).sin();
        let amplitude = i16::MAX as f32 * 0.5;
        samples.push((sample * amplitude) as i16);
    }

    // 6. Process Audio
    println!("🗣️ Processing audio ({} samples)...", samples.len());
    let process_start = Instant::now();

    // Feed audio in chunks
    let chunk_size = 4000;
    for chunk in samples.chunks(chunk_size) {
        plugin.process_audio(chunk).await?;
    }

    // 7. Finalize (Trigger transcription)
    println!("📝 Finalizing and transcribing...");
    if let Some(event) = plugin.finalize().await? {
        println!("🎉 Transcription Result: {:?}", event);
    } else {
        println!("⚠️ No transcription result returned.");
    }

    println!("⏱️ Total processing time: {:.2?}", process_start.elapsed());

    Ok(())
}
