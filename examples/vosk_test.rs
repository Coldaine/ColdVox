use std::path::Path;

use coldvox_stt::{Transcriber, TranscriptionConfig, TranscriptionEvent};
#[cfg(feature = "vosk")]
use coldvox_stt_vosk::VoskTranscriber;

#[cfg(feature = "vosk")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Test with a small Vosk model (download required)
    let model_path = "models/vosk-model-small-en-us-0.15";

    if !Path::new(model_path).exists() {
        eprintln!("Vosk model not found at: {}", model_path);
        eprintln!("Download a model from https://alphacephei.com/vosk/models");
        eprintln!("Extract to: {}", model_path);
        eprintln!("\nFor example:");
        eprintln!("  wget https://alphacephei.com/vosk/models/vosk-model-small-en-us-0.15.zip");
        eprintln!("  unzip vosk-model-small-en-us-0.15.zip");
        eprintln!("  mv vosk-model-small-en-us-0.15 models/");
        return Ok(());
    }

    println!("Loading Vosk model from: {}", model_path);

    // Create configuration
    let config = TranscriptionConfig {
        enabled: true,
        model_path: model_path.to_string(),
        partial_results: true,
        max_alternatives: 3,
        include_words: true,
        buffer_size_ms: 512,
    };

    // Create transcriber with configuration
    let mut transcriber = VoskTranscriber::new(config.clone(), 16000.0)?;

    println!("Vosk configuration:");
    println!("  Partial results: {}", config.partial_results);
    println!("  Max alternatives: {}", config.max_alternatives);
    println!("  Include words: {}", config.include_words);

    // Generate test audio: sine wave representing speech-like patterns
    let sample_rate = 16000;
    let duration_ms = 1000; // 1 second
    let samples_count = (sample_rate * duration_ms) / 1000;

    let mut test_audio = Vec::with_capacity(samples_count);
    for i in 0..samples_count {
        let t = i as f32 / sample_rate as f32;
        // Mix of frequencies to simulate speech
        let sample = (0.3 * (2.0 * std::f32::consts::PI * 440.0 * t).sin()
            + 0.2 * (2.0 * std::f32::consts::PI * 880.0 * t).sin()
            + 0.1 * (2.0 * std::f32::consts::PI * 1320.0 * t).sin())
            * 16384.0; // Scale to i16 range

        test_audio.push(sample as i16);
    }

    println!(
        "\nProcessing {} samples of synthetic audio...",
        test_audio.len()
    );

    // Process audio in chunks (512 samples = 32ms at 16kHz)
    let chunk_size = 512;
    let mut partial_count = 0;
    let mut result_count = 0;
    let mut error_count = 0;

    for (chunk_idx, chunk) in test_audio.chunks(chunk_size).enumerate() {
        // Use EventBasedTranscriber interface directly
        match coldvox_stt::EventBasedTranscriber::accept_frame(&mut transcriber, chunk)? {
            Some(TranscriptionEvent::Partial {
                utterance_id,
                text,
                t0,
                t1,
            }) => {
                partial_count += 1;
                println!(
                    "Chunk {}: Partial result (utterance {}): \"{}\"",
                    chunk_idx, utterance_id, text
                );
                if t0.is_some() || t1.is_some() {
                    println!("  Timing: {:?} - {:?}", t0, t1);
                }
            }
            Some(TranscriptionEvent::Final {
                utterance_id,
                text,
                words,
            }) => {
                result_count += 1;
                println!(
                    "Chunk {}: Final result (utterance {}): \"{}\"",
                    chunk_idx, utterance_id, text
                );
                if let Some(words) = words {
                    println!("  Words ({}): ", words.len());
                    for word in words.iter().take(5) {
                        println!(
                            "    \"{}\" @ {:.2}s-{:.2}s (conf: {:.2})",
                            word.text, word.start, word.end, word.conf
                        );
                    }
                    if words.len() > 5 {
                        println!("    ... and {} more", words.len() - 5);
                    }
                }
            }
            Some(TranscriptionEvent::Error { code, message }) => {
                error_count += 1;
                eprintln!("Chunk {}: Error [{}]: {}", chunk_idx, code, message);
            }
            None => {
                // No transcription for this chunk
            }
        }
    }

    // Get final result
    println!("\nFinalizing utterance...");
    match coldvox_stt::EventBasedTranscriber::finalize_utterance(&mut transcriber)? {
        Some(TranscriptionEvent::Final {
            utterance_id,
            text,
            words,
        }) => {
            println!(
                "Final transcription (utterance {}): \"{}\"",
                utterance_id, text
            );
            if let Some(words) = words {
                println!("Total words: {}", words.len());
            }
        }
        Some(TranscriptionEvent::Partial { text, .. }) => {
            println!("Unexpected partial result: \"{}\"", text);
        }
        Some(TranscriptionEvent::Error { code, message }) => {
            eprintln!("Finalization error [{}]: {}", code, message);
        }
        None => {
            println!("No final transcription (synthetic audio not recognized as speech)");
        }
    }

    println!("\nTest completed:");
    println!("  Partial results: {}", partial_count);
    println!("  Final results: {}", result_count);
    println!("  Errors: {}", error_count);
    println!("\nNote: Synthetic audio may not produce meaningful transcriptions.");
    println!("For real testing, use actual speech audio or WAV files.");

    // Test backward compatibility with Transcriber trait
    println!("\n--- Testing backward compatibility ---");
    let mut simple_transcriber = VoskTranscriber::new_with_default(model_path, 16000.0)?;

    // Test with smaller chunk
    let test_chunk = &test_audio[0..512];
    match simple_transcriber.accept_pcm16(test_chunk)? {
        Some(text) => println!("Transcriber trait result: \"{}\"", text),
        None => println!("Transcriber trait: No result"),
    }

    match simple_transcriber.finalize()? {
        Some(text) => println!("Transcriber trait final: \"{}\"", text),
        None => println!("Transcriber trait: No final result"),
    }

    Ok(())
}

#[cfg(not(feature = "vosk"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("Vosk feature is not enabled!");
    eprintln!("Run with: cargo run --example vosk_test --features vosk");
    eprintln!("\nThis demonstrates feature gating - the example only compiles and runs when the vosk feature is enabled.");
    Ok(())
}
