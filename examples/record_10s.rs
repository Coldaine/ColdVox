use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::{WavSpec, WavWriter};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎤 Starting 10-second recording at 16kHz...");
    
    // Set up audio host and device
    let host = cpal::default_host();
    let device = host.default_input_device()
        .ok_or("No input device available")?;
    
    println!("Using device: {}", device.name()?);
    
    // Configure for 16kHz mono
    let config = cpal::StreamConfig {
        channels: 1,
        sample_rate: cpal::SampleRate(16000),
        buffer_size: cpal::BufferSize::Default,
    };
    
    // Prepare WAV writer
    let spec = WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    
    // Generate timestamp for unique filename
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs();
    let output_path = format!("recording_16khz_10s_{}.wav", timestamp);
    let writer = WavWriter::create(&output_path, spec)?;
    let writer = Arc::new(Mutex::new(Some(writer)));
    let writer_clone = Arc::clone(&writer);
    
    // Track recording duration
    let start_time = Instant::now();
    let duration = Duration::from_secs(10);
    let recording_done = Arc::new(Mutex::new(false));
    let recording_done_clone = Arc::clone(&recording_done);
    
    // Create stream
    let stream = device.build_input_stream(
        &config,
        move |data: &[i16], _: &cpal::InputCallbackInfo| {
            // Check if we should stop recording
            if start_time.elapsed() >= duration {
                *recording_done_clone.lock().unwrap() = true;
                return;
            }
            
            // Write samples to WAV file
            if let Ok(mut guard) = writer_clone.lock() {
                if let Some(ref mut writer) = *guard {
                    for &sample in data {
                        let _ = writer.write_sample(sample);
                    }
                }
            }
        },
        move |err| {
            eprintln!("Error in stream: {}", err);
        },
        None,
    )?;
    
    // Start recording
    stream.play()?;
    println!("Recording for 10 seconds...");
    
    // Show progress
    let mut last_second = 0;
    while !*recording_done.lock().unwrap() {
        let elapsed = start_time.elapsed().as_secs();
        if elapsed != last_second {
            print!("\r{}/10 seconds", elapsed);
            use std::io::{self, Write};
            io::stdout().flush().unwrap();
            last_second = elapsed;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    println!("\r10/10 seconds - Done!");
    
    // Stop and finalize
    drop(stream);
    
    // Finalize WAV file
    if let Ok(mut guard) = writer.lock() {
        if let Some(writer) = guard.take() {
            writer.finalize()?;
        }
    }
    
    println!("✅ Recording saved to: {}", output_path);
    
    Ok(())
}