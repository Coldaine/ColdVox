use std::fs;
use std::process::Command;
use rand::seq::SliceRandom;

fn main() {
    println!("Testing VAD with real audio files from the test library...");

    // 1. Discover test files from `crates/app/test_data`
    let test_data_dir = "crates/app/test_data";
    let entries = match fs::read_dir(test_data_dir) {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!("Error: Failed to read test data directory at '{}': {}", test_data_dir, e);
            eprintln!("Please ensure the test data directory exists and has the correct permissions.");
            return;
        }
    };

    let wav_files: Vec<String> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("wav") {
                // 2. Ensure a corresponding .txt transcript file exists
                let txt_path = path.with_extension("txt");
                if txt_path.exists() {
                    Some(path.to_string_lossy().to_string())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    if wav_files.is_empty() {
        eprintln!("Warning: No WAV files with corresponding .txt transcripts found in '{}'.", test_data_dir);
        eprintln!("VAD testing will be skipped.");
        return;
    }

    // 3. Randomly select 3-5 files for testing
    let mut rng = rand::thread_rng();
    let sample_size = wav_files.len().min(5).max(3);
    let selected_files: Vec<String> = wav_files
        .choose_multiple(&mut rng, sample_size)
        .cloned()
        .collect();

    println!("Found {} eligible test files. Randomly selected {} for this run.
", wav_files.len(), selected_files.len());

    for file_path in &selected_files {
        println!("
=== Testing with {} ===", file_path);

        // 4. Use the existing `test_silero_wav` example
        println!("Running Silero VAD example...");
        let output = Command::new("cargo")
            .args(&["run", "--features", "examples", "--example", "test_silero_wav"])
            .env("TEST_WAV", file_path)
            .output();

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                if !output.status.success() {
                    eprintln!("  Error: The test example failed to run for {}.", file_path);
                    eprintln!("  Stderr: {}", stderr);
                    continue;
                }

                // 5. Maintain similar output format
                let speech_count = stdout.matches("SpeechStart").count();
                let total_speech: f32 = stdout
                    .lines()
                    .find(|l| l.contains("Total speech duration"))
                    .and_then(|l| {
                        l.split(':')
                            .nth(1)
                            .and_then(|s| s.trim().trim_end_matches('s').parse().ok())
                    })
                    .unwrap_or(0.0);

                println!("  Speech segments detected: {}", speech_count);
                println!("  Total speech duration: {:.2}s", total_speech);

                if speech_count == 0 && total_speech < 0.01 {
                    println!("  Warning: No speech detected. This might be expected for silent audio, or it could indicate a VAD issue.");
                }
            }
            Err(e) => {
                // 6. Handle errors gracefully
                eprintln!("  Error: Failed to execute the 'cargo run' command for the test example.");
                eprintln!("  Reason: {}", e);
                eprintln!("  Please ensure 'cargo' is in your PATH and the example 'test_silero_wav' exists.");
            }
        }
    }
}
