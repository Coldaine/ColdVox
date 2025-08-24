use clap::Parser;
use std::time::{Duration, Instant};
use coldvox_app::foundation::*;
use coldvox_app::audio::*;

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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    tracing_subscriber::fmt()
        .with_env_filter("debug")
        .init();
    
    // List available devices
    let device_manager = DeviceManager::new()?;
    let devices = device_manager.enumerate_devices();
    
    println!("Available audio devices:");
    for device in &devices {
        println!("  {} {}", 
            if device.is_default { "[DEFAULT]" } else { "         " },
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
    let start = Instant::now();
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