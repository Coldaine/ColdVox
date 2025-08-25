use std::process::Command;

fn main() {
    println!("Testing VAD with real audio files...\n");
    
    let test_files = vec![
        "test_audio_16k.wav",
        "captured_audio.wav",
        "recording_16khz_10s_1756081007.wav",
    ];
    
    for file in &test_files {
        println!("\n=== Testing with {} ===", file);
        
        // Test with Silero
        println!("\nSilero VAD (threshold 0.3):");
        let output = Command::new("cargo")
            .args(&["run", "--bin", "vad_demo", "--", "silero", "0.3"])
            .env("VAD_TEST_FILE", file)
            .output()
            .expect("Failed to run Silero test");
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let speech_count = stdout.matches("Speech START").count();
        let total_speech: f32 = stdout
            .lines()
            .filter(|l| l.contains("Event handler stopped"))
            .next()
            .and_then(|l| {
                l.split("s of speech")
                    .next()
                    .and_then(|s| s.rsplit(',').next())
                    .and_then(|s| s.trim().parse().ok())
            })
            .unwrap_or(0.0);
        
        println!("  Speech segments: {}", speech_count);
        println!("  Total speech duration: {:.2}s", total_speech);
        
        // Test with Level3
        println!("\nLevel3 VAD:");
        let output = Command::new("cargo")
            .args(&["run", "--bin", "vad_demo", "--", "level3"])
            .env("VAD_TEST_FILE", file)
            .output()
            .expect("Failed to run Level3 test");
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let speech_count = stdout.matches("Speech START").count();
        let total_speech: f32 = stdout
            .lines()
            .filter(|l| l.contains("Event handler stopped"))
            .next()
            .and_then(|l| {
                l.split("s of speech")
                    .next()
                    .and_then(|s| s.rsplit(',').next())
                    .and_then(|s| s.trim().parse().ok())
            })
            .unwrap_or(0.0);
        
        println!("  Speech segments: {}", speech_count);
        println!("  Total speech duration: {:.2}s", total_speech);
    }
}