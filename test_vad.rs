use std::process::Command;

fn main() {
    println!("Testing VAD with real audio files...\n");

    let test_files = vec![
        "test_audio_16k.wav",
        "captured_audio.wav",
        "recording_16khz_10s_1756081007.wav",
    ];

    for file in &test_files {
        // Determine absolute path under crates/app
        let mut env_path = std::env::current_dir().unwrap();
        env_path.push("crates");
        env_path.push("app");
        env_path.push(file);
        let wav_env = env_path.to_string_lossy().to_string();

        println!("\n=== Testing with {} ===", file);

        // Test with Silero (maintained VAD example)
        println!("\nSilero VAD:");
        let output = Command::new("cargo")
            .args(&["run", "--features", "examples", "--example", "test_silero_wav"])
            .env("TEST_WAV", &wav_env)
            .output()
            .expect("Failed to run Silero test");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let speech_count = stdout.matches("SpeechStart").count();
        let total_speech: f32 = stdout
            .lines()
            .filter(|l| l.contains("Total speech duration"))
            .next()
            .and_then(|l| {
                l.split("Total speech duration:")
                    .nth(1)
                    .and_then(|s| s.trim().split('s').next())
                    .and_then(|s| s.trim().parse().ok())
            })
            .unwrap_or(0.0);

        println!("  Speech segments: {}", speech_count);
        println!("  Total speech duration: {:.2}s", total_speech);

        // Note: Level3 VAD testing removed; use test_silero_wav for VAD testing
    }
}
