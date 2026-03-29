use coldvox_audio::{AudioCaptureThread, AudioRingBuffer};
use coldvox_foundation::AudioConfig;
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rb = AudioRingBuffer::new(16000 * 5); // 5 seconds capacity
    let (producer, mut consumer) = rb.split();

    let config = AudioConfig {
        silence_threshold: 500, // i16 threshold
        ..Default::default()
    };

    println!("Starting live audio capture...");

    let (thread, device_cfg, mut _config_rx, mut _event_rx) = AudioCaptureThread::spawn(
        config,
        Arc::new(Mutex::new(producer)),
        None, // Use default device
        true, // Enable monitor
    )?;

    println!("Capture started. Device Config: {:?}", device_cfg);
    println!("Listening for 10 seconds...");

    let start = std::time::Instant::now();
    let mut buffer = vec![0i16; 4000]; // 250ms chunks (at 16kHz)

    while start.elapsed() < Duration::from_secs(10) {
        let read = consumer.read(&mut buffer);
        if read > 0 {
            let max_amplitude = buffer[..read].iter().map(|&x| x.abs()).max().unwrap_or(0);
            let mut bar = String::new();
            let scaled = (max_amplitude as f32 / 32768.0 * 50.0) as usize;
            for _ in 0..scaled {
                bar.push('*');
            }
            println!(
                "Read {:<5} samples. Amplitude: {:<5} |{}",
                read, max_amplitude, bar
            );
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    println!("Stopping capture...");
    thread.stop();
    println!("Exiting cleanly.");

    Ok(())
}
