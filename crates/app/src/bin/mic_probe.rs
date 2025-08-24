use clap::Parser;
use coldvox_app::audio::*;
use coldvox_app::foundation::*;
use hound::{WavSpec, WavWriter};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "120")]
    duration: u64,

    #[arg(long)]
    device: Option<String>,

    #[arg(long)]
    expect_disconnect: bool,

    #[arg(long)]
    save_audio: bool,

    #[arg(long, default_value = "100")]
    silence_threshold: i16,

    #[arg(long)]
    test_capture: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Special test capture mode
    if args.test_capture {
        return run_audio_test().await;
    }

    tracing_subscriber::fmt().with_env_filter("debug").init();

    // List available devices
    let device_manager = DeviceManager::new()?;
    let devices = device_manager.enumerate_devices();

    println!("Available audio devices:");
    for device in &devices {
        println!(
            "  {} {}",
            if device.is_default {
                "[DEFAULT]"
            } else {
                "         "
            },
            device.name
        );
    }

    // Create capture
    let config = AudioConfig {
        silence_threshold: args.silence_threshold,
    };

    let mut capture = AudioCapture::new(config)?;
    capture.start(args.device.as_deref()).await?;

    // Install Ctrl+C shutdown guard for clean exit
    let shutdown = ShutdownHandler::new().install().await;

    // Get receiver to consume frames
    let frame_rx = capture.get_receiver();

    // Monitor loop
    let _start = Instant::now();
    let mut last_stats = Instant::now();

    // Spawn task to consume audio frames
    tokio::spawn(async move {
        while let Ok(_frame) = frame_rx.recv() {
            // Just consume frames to prevent buffer overflow
            // In a real application, this is where you'd process the audio
        }
    });

    // Main probe loop: duration or Ctrl+C, whichever first
    let deadline = tokio::time::sleep(Duration::from_secs(args.duration));
    tokio::pin!(deadline);
    loop {
        tokio::select! {
            _ = &mut deadline => { break; }
            _ = shutdown.wait() => { println!("Shutdown requested"); break; }
            _ = tokio::time::sleep(Duration::from_secs(1)) => {
                // Print stats every 5 seconds
                if last_stats.elapsed() > Duration::from_secs(5) {
                    let stats = capture.get_stats();
                    println!(
                        "Stats: {} frames, {} active, {} silent, {} dropped, {} disconnects, {} reconnects",
                        stats.frames_captured,
                        stats.active_frames,
                        stats.silent_frames,
                        stats.frames_dropped,
                        stats.disconnections,
                        stats.reconnections
                    );
                    if let Some(age) = stats.last_frame_age {
                        if age > Duration::from_secs(2) {
                            println!("WARNING: No frames for {:?}", age);
                        }
                    }
                    last_stats = Instant::now();
                }

                // Test disconnect recovery
                if args.expect_disconnect {
                    println!("Unplug and replug your microphone to test recovery...");
                    // Wait for watchdog to trigger
                    if capture.get_watchdog().is_triggered() {
                        println!("Device disconnected, attempting recovery...");
                        match capture.recover().await {
                            Ok(_) => println!("Recovery successful!"),
                            Err(e) => println!("Recovery failed: {}", e),
                        }
                    }
                }
            }
        }
    }

    // Clean shutdown of capture and watchdog
    capture.stop();
    println!("Test completed successfully");
    Ok(())
}

async fn run_audio_test() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎤 ColdVox Audio Capture Test");
    println!("Capturing 10 seconds of microphone audio...");
    println!();

    tracing_subscriber::fmt().with_env_filter("info").init();

    // Create capture with default config
    let config = AudioConfig::default();
    let mut capture = AudioCapture::new(config)?;

    // List and show selected device
    let device_manager = DeviceManager::new()?;
    let devices = device_manager.enumerate_devices();

    println!("Available devices:");
    for device in &devices {
        println!("  {}", device.name);
    }
    println!();

    capture.start(None).await?;

    // Get the receiver for audio frames
    let frame_rx = capture.get_receiver();

    // Setup WAV file writing
    let spec = WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let output_file = "captured_audio.wav";
    let mut writer = WavWriter::create(output_file, spec)?;

    // Volume analysis variables
    let mut volume_samples = VecDeque::new();
    let mut max_volume = 0.0f32;
    let mut total_samples = 0u64;
    let mut total_energy = 0.0f64;

    let start_time = Instant::now();
    let mut last_update = Instant::now();

    println!("Recording... (speak into your microphone!)");
    println!("Volume: [          ] 0%");

    // Capture loop
    while start_time.elapsed() < Duration::from_secs(10) {
        if let Ok(frame) = frame_rx.try_recv() {
            // Write samples to WAV file
            for &sample in &frame.samples {
                writer.write_sample(sample)?;
            }

            // Calculate RMS volume for this frame
            let sum_squares: f64 = frame
                .samples
                .iter()
                .map(|&s| (s as f64 / 32768.0).powi(2))
                .sum();
            let rms = (sum_squares / frame.samples.len() as f64).sqrt();
            let volume_percent = (rms * 100.0) as f32;

            // Track statistics
            max_volume = max_volume.max(volume_percent);
            total_samples += frame.samples.len() as u64;
            total_energy += sum_squares;

            // Keep rolling window of recent volumes
            volume_samples.push_back(volume_percent);
            if volume_samples.len() > 10 {
                volume_samples.pop_front();
            }

            // Update display every 100ms
            if last_update.elapsed() > Duration::from_millis(100) {
                let current_avg = volume_samples.iter().sum::<f32>() / volume_samples.len() as f32;
                let elapsed = start_time.elapsed().as_secs();

                // Create volume bar
                let bar_length = 10;
                let filled =
                    ((current_avg / 20.0) * bar_length as f32).min(bar_length as f32) as usize;
                let bar: String = "█".repeat(filled) + &"░".repeat(bar_length - filled);

                print!(
                    "\r🎤 {}s Volume: [{}] {:.1}%  Peak: {:.1}%",
                    elapsed, bar, current_avg, max_volume
                );
                std::io::Write::flush(&mut std::io::stdout())?;

                last_update = Instant::now();
            }
        } else {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    println!("\n");

    // Finalize WAV file
    writer.finalize()?;
    capture.stop();

    // Calculate overall statistics
    let overall_rms = if total_samples > 0 {
        (total_energy / total_samples as f64).sqrt()
    } else {
        0.0
    };
    let overall_volume = (overall_rms * 100.0) as f32;

    let stats = capture.get_stats();

    // Results summary
    println!("✅ Recording Complete!");
    println!();
    println!("📊 Audio Statistics:");
    println!("   • File: {}", output_file);
    println!("   • Duration: 10 seconds");
    println!("   • Sample Rate: 16,000 Hz");
    println!("   • Format: 16-bit mono WAV");
    println!("   • Frames Captured: {}", stats.frames_captured);
    println!("   • Frames Dropped: {}", stats.frames_dropped);
    println!();
    println!("🔊 Volume Analysis:");
    println!("   • Average Volume: {:.1}%", overall_volume);
    println!("   • Peak Volume: {:.1}%", max_volume);
    println!("   • Total Samples: {}", total_samples);
    println!("   • Active Frames: {}", stats.active_frames);
    println!("   • Silent Frames: {}", stats.silent_frames);

    if max_volume < 1.0 {
        println!();
        println!("⚠️  Very low volume detected. Check your microphone!");
    } else if max_volume > 80.0 {
        println!();
        println!("⚠️  Very high volume detected. Audio may be clipped!");
    } else if max_volume > 5.0 {
        println!();
        println!("✅ Good audio levels detected!");
    }

    println!();
    println!("🎵 You can play the captured audio with:");
    println!("   aplay {}", output_file);
    println!("   or");
    println!("   mpv {}", output_file);

    Ok(())
}
