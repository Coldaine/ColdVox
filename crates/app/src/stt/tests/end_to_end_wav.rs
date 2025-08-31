#![cfg(feature = "vosk")]
use anyhow::Result;
use hound::WavReader;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc};
use tracing::info;

use coldvox_audio::chunker::{AudioChunker, ChunkerConfig};
use coldvox_audio::ring_buffer::{AudioRingBuffer, AudioProducer};
use coldvox_audio::chunker::AudioFrame;
use crate::stt::{processor::SttProcessor, TranscriptionConfig, TranscriptionEvent};
// use crate::text_injection::{AsyncInjectionProcessor, InjectionProcessorConfig};
use coldvox_vad::config::{UnifiedVadConfig, VadMode};
use coldvox_vad::constants::{FRAME_SIZE_SAMPLES, SAMPLE_RATE_HZ};
use coldvox_vad::types::VadEvent;

/// Mock text injector that captures injection attempts for testing
pub struct MockTextInjector {
    injections: Arc<Mutex<Vec<String>>>,
}

impl MockTextInjector {
    pub fn new() -> Self {
        Self {
            injections: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn inject(&self, text: &str) -> Result<()> {
        info!("Mock injection: {}", text);
        self.injections.lock().unwrap().push(text.to_string());
        Ok(())
    }

    pub fn get_injections(&self) -> Vec<String> {
        self.injections.lock().unwrap().clone()
    }
}

/// WAV file loader that feeds audio data through the pipeline
pub struct WavFileLoader {
    samples: Vec<i16>,
    sample_rate: u32,
    current_pos: usize,
    frame_size: usize,
}

impl WavFileLoader {
    /// Load WAV file and prepare for streaming
    pub fn new<P: AsRef<Path>>(wav_path: P, target_sample_rate: u32) -> Result<Self> {
        let mut reader = WavReader::open(wav_path)?;
        let spec = reader.spec();
        
        info!("Loading WAV: {} Hz, {} channels, {} bits", 
              spec.sample_rate, spec.channels, spec.bits_per_sample);

        // Read all samples
        let samples: Vec<i16> = reader.samples::<i16>()
            .collect::<Result<Vec<_>, _>>()?;

        // Convert to mono if stereo
        let mono_samples = if spec.channels == 2 {
            samples.chunks(2)
                .map(|chunk| ((chunk[0] as i32 + chunk[1] as i32) / 2) as i16)
                .collect()
        } else {
            samples
        };

        // Resample if necessary (simple linear interpolation)
        let final_samples = if spec.sample_rate != target_sample_rate {
            let ratio = target_sample_rate as f32 / spec.sample_rate as f32;
            let new_len = (mono_samples.len() as f32 * ratio) as usize;
            let mut resampled = Vec::with_capacity(new_len);
            
            for i in 0..new_len {
                let src_idx = i as f32 / ratio;
                let idx = src_idx as usize;
                if idx < mono_samples.len() {
                    resampled.push(mono_samples[idx]);
                }
            }
            resampled
        } else {
            mono_samples
        };

        info!("WAV loaded: {} samples at {} Hz", final_samples.len(), target_sample_rate);

        Ok(Self {
            samples: final_samples,
            sample_rate: target_sample_rate,
            current_pos: 0,
            frame_size: FRAME_SIZE_SAMPLES,
        })
    }

    /// Stream audio data to ring buffer with realistic timing
    pub async fn stream_to_ring_buffer(&mut self, mut producer: AudioProducer) -> Result<()> {
        let frame_duration = Duration::from_millis((self.frame_size * 1000) as u64 / self.sample_rate as u64);
        
        while self.current_pos < self.samples.len() {
            let end_pos = (self.current_pos + self.frame_size).min(self.samples.len());
            let chunk = &self.samples[self.current_pos..end_pos];
            
            // Try to write chunk to ring buffer
            let mut written = 0;
            while written < chunk.len() {
                match producer.write(&chunk[written..]) {
                    Ok(count) => written += count,
                    Err(_) => {
                        // Ring buffer full, wait a bit
                        tokio::time::sleep(Duration::from_millis(1)).await;
                    }
                }
            }
            
            self.current_pos = end_pos;
            
            // Maintain realistic timing
            tokio::time::sleep(frame_duration).await;
        }
        
        info!("WAV streaming completed");
        Ok(())
    }

    pub fn duration_ms(&self) -> u64 {
        (self.samples.len() * 1000) as u64 / self.sample_rate as u64
    }
}

/// Mock injection processor that uses our mock injector
pub struct MockInjectionProcessor {
    injector: MockTextInjector,
    transcription_rx: mpsc::Receiver<TranscriptionEvent>,
    shutdown_rx: mpsc::Receiver<()>,
}

impl MockInjectionProcessor {
    pub fn new(
        injector: MockTextInjector,
        transcription_rx: mpsc::Receiver<TranscriptionEvent>,
        shutdown_rx: mpsc::Receiver<()>,
    ) -> Self {
        Self {
            injector,
            transcription_rx,
            shutdown_rx,
        }
    }

    pub async fn run(mut self) -> Result<()> {
        let mut buffer = String::new();
        let check_interval = Duration::from_millis(200);
        let mut last_transcription = None;

        loop {
            tokio::select! {
                // Handle transcription events
                Some(event) = self.transcription_rx.recv() => {
                    match event {
                        TranscriptionEvent::Final { text, .. } => {
                            info!("Mock processor received final: {}", text);
                            if !text.trim().is_empty() {
                                buffer.push_str(&text);
                                buffer.push(' ');
                                last_transcription = Some(Instant::now());
                            }
                        }
                        TranscriptionEvent::Partial { text, .. } => {
                            info!("Mock processor received partial: {}", text);
                        }
                        TranscriptionEvent::Error { code, message } => {
                            info!("Mock processor received error [{}]: {}", code, message);
                        }
                    }
                }

                // Check for silence timeout and inject
                _ = tokio::time::sleep(check_interval) => {
                    if let Some(last_time) = last_transcription {
                        if last_time.elapsed() > Duration::from_millis(500) && !buffer.trim().is_empty() {
                            let text_to_inject = buffer.trim().to_string();
                            if !text_to_inject.is_empty() {
                                self.injector.inject(&text_to_inject).await?;
                                buffer.clear();
                                last_transcription = None;
                            }
                        }
                    }
                }

                // Shutdown signal
                _ = self.shutdown_rx.recv() => {
                    info!("Mock injection processor shutting down");
                    // Inject any remaining buffer content
                    if !buffer.trim().is_empty() {
                        self.injector.inject(&buffer.trim()).await?;
                    }
                    break;
                }
            }
        }

        Ok(())
    }
}

/// End-to-end test that processes a WAV file through the entire pipeline
pub async fn test_wav_pipeline<P: AsRef<Path>>(
    wav_path: P,
    expected_text_fragments: Vec<&str>,
) -> Result<Vec<String>> {
    info!("Starting end-to-end WAV pipeline test");

    // Set up components
    let mock_injector = MockTextInjector::new();
    let ring_buffer = AudioRingBuffer::new(16384 * 4);
    let (audio_producer, audio_consumer) = ring_buffer.split();
    
    // Load WAV file
    let mut wav_loader = WavFileLoader::new(wav_path, SAMPLE_RATE_HZ)?;
    let test_duration = Duration::from_millis(wav_loader.duration_ms() + 2000); // Add buffer time
    
    // Set up audio chunker
    let (audio_tx, _) = broadcast::channel::<AudioFrame>(200);
    let frame_reader = coldvox_audio::frame_reader::FrameReader::new(
        audio_consumer,
        SAMPLE_RATE_HZ,
        1, // mono
        16384 * 4,
        None,
    );
    
    let chunker_cfg = ChunkerConfig {
        frame_size_samples: FRAME_SIZE_SAMPLES,
        sample_rate_hz: SAMPLE_RATE_HZ,
        resampler_quality: coldvox_audio::chunker::ResamplerQuality::Balanced,
    };
    
    let chunker = AudioChunker::new(frame_reader, audio_tx.clone(), chunker_cfg);
    let chunker_handle = chunker.spawn();

    // Set up VAD processor
    let vad_cfg = UnifiedVadConfig {
        mode: VadMode::Silero,
        frame_size_samples: FRAME_SIZE_SAMPLES,
        sample_rate_hz: SAMPLE_RATE_HZ,
        ..Default::default()
    };

    let (vad_event_tx, vad_event_rx) = mpsc::channel::<VadEvent>(100);
    let vad_audio_rx = audio_tx.subscribe();
    let vad_handle = match crate::audio::vad_processor::VadProcessor::spawn(
        vad_cfg,
        vad_audio_rx,
        vad_event_tx,
        None,
    ) {
        Ok(handle) => handle,
        Err(e) => anyhow::bail!("Failed to spawn VAD processor: {}", e),
    };

    // Set up STT processor
    let (stt_transcription_tx, stt_transcription_rx) = mpsc::channel::<TranscriptionEvent>(100);
    let stt_config = TranscriptionConfig {
        enabled: true,
        model_path: std::env::var("VOSK_MODEL_PATH")
            .unwrap_or_else(|_| "models/vosk-model-small-en-us-0.15".to_string()),
        partial_results: true,
        max_alternatives: 1,
        include_words: false,
        buffer_size_ms: 512,
    };

    // Check if STT model exists
    if !std::path::Path::new(&stt_config.model_path).exists() {
        anyhow::bail!(
            "Vosk model not found at '{}'. Download a model or set VOSK_MODEL_PATH environment variable.",
            stt_config.model_path
        );
    }

    let stt_audio_rx = audio_tx.subscribe();
    let stt_processor = match SttProcessor::new(stt_audio_rx, vad_event_rx, stt_transcription_tx, stt_config) {
        Ok(processor) => processor,
        Err(e) => anyhow::bail!("Failed to create STT processor: {}", e),
    };
    let stt_handle = tokio::spawn(async move {
        stt_processor.run().await;
    });

    // Set up mock injection processor
    let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>(1);
    let mock_injector_clone = MockTextInjector {
        injections: Arc::clone(&mock_injector.injections),
    };
    
    let injection_processor = MockInjectionProcessor::new(
        mock_injector_clone,
        stt_transcription_rx,
        shutdown_rx,
    );
    let _injection_handle = tokio::spawn(async move {
        injection_processor.run().await
    });

    // Start streaming WAV data
    let streaming_handle = tokio::spawn(async move {
        wav_loader.stream_to_ring_buffer(audio_producer).await
    });

    info!("Pipeline started, running for {:?}", test_duration);

    // Wait for test duration
    tokio::time::sleep(test_duration).await;

    // Shutdown
    let _ = shutdown_tx.send(()).await;
    chunker_handle.abort();
    vad_handle.abort();
    stt_handle.abort();
    streaming_handle.abort();

    // Give a moment for final processing
    tokio::time::sleep(Duration::from_millis(500)).await;

    let injections = mock_injector.get_injections();
    info!("Test completed. Injections captured: {:?}", injections);

    // Verify at least one expected text fragment is present (STT may not be 100% accurate)
    let all_text = injections.join(" ").to_lowercase();
    let mut found_any = false;
    let mut found_fragments = Vec::new();
    
    for expected in &expected_text_fragments {
        if all_text.contains(&expected.to_lowercase()) {
            found_any = true;
            found_fragments.push(expected.clone());
        }
    }
    
    if !found_any && !expected_text_fragments.is_empty() {
        anyhow::bail!(
            "None of the expected text fragments {:?} were found in injections: {:?}",
            expected_text_fragments,
            injections
        );
    }
    
    info!("Found expected fragments: {:?}", found_fragments);

    Ok(injections)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires WAV files and Vosk model
    async fn test_end_to_end_wav_pipeline() {
        use std::fs;
        use rand::seq::SliceRandom;
        
        // This test requires:
        // 1. A WAV file with known speech content
        // 2. Vosk model downloaded and configured
        
        // Look for test WAV files in test_data directory
        let test_data_dir = "test_data";
        
        // If TEST_WAV is set, use that specific file
        let (wav_path, expected_fragments) = if let Ok(specific_wav) = std::env::var("TEST_WAV") {
            if !std::path::Path::new(&specific_wav).exists() {
                eprintln!("Skipping test: WAV file '{}' not found", specific_wav);
                return;
            }
            // For manually specified WAV, use generic expectations
            (specific_wav, vec!["the".to_string()])
        } else {
            // Find all WAV files in test_data that have corresponding transcripts
            let entries = match fs::read_dir(test_data_dir) {
                Ok(entries) => entries,
                Err(_) => {
                    eprintln!("Skipping test: test_data directory not found");
                    eprintln!("Expected test WAV files in: {}", test_data_dir);
                    return;
                }
            };
            
            let mut wav_files = Vec::new();
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("wav") {
                    let txt_path = path.with_extension("txt");
                    if txt_path.exists() {
                        wav_files.push(path.to_string_lossy().to_string());
                    }
                }
            }
            
            if wav_files.is_empty() {
                eprintln!("Skipping test: No WAV files with transcripts found in test_data/");
                return;
            }
            
            // Randomly select a test file
            let mut rng = rand::thread_rng();
            let selected_wav = wav_files.choose(&mut rng).unwrap().clone();
            
            // Load the corresponding transcript
            let txt_path = std::path::Path::new(&selected_wav).with_extension("txt");
            let transcript = fs::read_to_string(&txt_path)
                .unwrap_or_else(|e| panic!("Failed to read transcript {}: {}", txt_path.display(), e));
            
            // Extract key words from transcript (longer words are more distinctive)
            let words: Vec<String> = transcript
                .to_lowercase()
                .split_whitespace()
                .filter(|w| w.len() >= 4) // Focus on words with 4+ characters
                .take(3) // Take up to 3 key words
                .map(|s| s.to_string())
                .collect();
            
            let expected = if words.is_empty() {
                // Fallback to any word if no long words found
                transcript
                    .to_lowercase()
                    .split_whitespace()
                    .take(2)
                    .map(|s| s.to_string())
                    .collect()
            } else {
                words
            };
            
            (selected_wav, expected)
        };
        
        println!("Testing with WAV file: {}", wav_path);
        println!("Expected keywords: {:?}", expected_fragments);
        
        // Convert Vec<String> to Vec<&str> for the test function
        let expected_refs: Vec<&str> = expected_fragments.iter().map(|s| s.as_str()).collect();
        
        match test_wav_pipeline(wav_path, expected_refs).await {
            Ok(injections) => {
                println!("✅ Test passed! Injections: {:?}", injections);
                assert!(!injections.is_empty(), "No text was injected");
            }
            Err(e) => {
                eprintln!("❌ Test failed: {}", e);
                panic!("End-to-end test failed: {}", e);
            }
        }
    }

    #[test]
    fn test_wav_file_loader() {
        // Test WAV file loading with a simple synthetic file
        // This could be expanded to create a simple test WAV file
        
        // For now, just test the struct creation
        let injector = MockTextInjector::new();
        assert_eq!(injector.get_injections().len(), 0);
        
        // Test injection
        tokio_test::block_on(async {
            injector.inject("test").await.unwrap();
            assert_eq!(injector.get_injections(), vec!["test"]);
        });
    }
}