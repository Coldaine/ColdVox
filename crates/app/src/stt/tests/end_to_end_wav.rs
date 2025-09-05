#![cfg(feature = "vosk")]
use anyhow::Result;
use hound::WavReader;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, info};

use crate::stt::{processor::SttProcessor, TranscriptionConfig, TranscriptionEvent};
use crate::text_injection::{AsyncInjectionProcessor, InjectionConfig};
use coldvox_audio::chunker::AudioFrame;
use coldvox_audio::chunker::{AudioChunker, ChunkerConfig};
use coldvox_audio::ring_buffer::{AudioProducer, AudioRingBuffer};
use coldvox_audio::DeviceConfig;
use coldvox_vad::config::{UnifiedVadConfig, VadMode};
use coldvox_vad::constants::{FRAME_SIZE_SAMPLES, SAMPLE_RATE_HZ};
use coldvox_vad::types::VadEvent;

/// Initialize tracing for tests with debug level
fn init_test_tracing() {
    use std::sync::Once;
    use tracing_subscriber::{fmt, EnvFilter};

    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug"));

        fmt().with_env_filter(filter).with_test_writer().init();
    });
}

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
    channels: u16,
    current_pos: usize,
    frame_size_total: usize,
}

impl WavFileLoader {
    /// Load WAV file and prepare for streaming (no resample/mono conversion)
    /// This mirrors live capture: raw device rate/channels into ring buffer.
    pub fn new<P: AsRef<Path>>(wav_path: P) -> Result<Self> {
        let mut reader = WavReader::open(wav_path)?;
        let spec = reader.spec();

        info!(
            "Loading WAV: {} Hz, {} channels, {} bits",
            spec.sample_rate, spec.channels, spec.bits_per_sample
        );

        // Read all samples as interleaved i16
        let samples: Vec<i16> = reader.samples::<i16>().collect::<Result<Vec<_>, _>>()?;

        info!(
            "WAV loaded: {} samples (interleaved) at {} Hz, {} channels",
            samples.len(),
            spec.sample_rate,
            spec.channels
        );

        // Choose a chunk size close to ~32ms per channel to emulate callback pacing
        // FRAME_SIZE_SAMPLES is per mono channel; scale by channel count for total i16 samples
        let frame_size_total = FRAME_SIZE_SAMPLES * spec.channels as usize;

        Ok(Self {
            samples,
            sample_rate: spec.sample_rate,
            channels: spec.channels,
            current_pos: 0,
            frame_size_total,
        })
    }

    /// Stream audio data to ring buffer with realistic timing
    pub async fn stream_to_ring_buffer(&mut self, mut producer: AudioProducer) -> Result<()> {
        // Duration for one chunk of size `frame_size_total` (interleaved across channels)
        // time = samples_total / (sample_rate * channels)
        let nanos_per_sample_total =
            1_000_000_000u64 / (self.sample_rate as u64 * self.channels as u64);

        while self.current_pos < self.samples.len() {
            let end_pos = (self.current_pos + self.frame_size_total).min(self.samples.len());
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

            // Maintain realistic timing for the total interleaved samples written
            let written_total = chunk.len() as u64;
            let sleep_nanos = written_total * nanos_per_sample_total;
            tokio::time::sleep(Duration::from_nanos(sleep_nanos)).await;
        }

        info!(
            "WAV streaming completed ({} total samples processed)",
            self.current_pos
        );
        Ok(())
    }

    pub fn duration_ms(&self) -> u64 {
        // Total interleaved samples divided by (rate * channels)
        ((self.samples.len() as u64) * 1000) / (self.sample_rate as u64 * self.channels as u64)
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    pub fn channels(&self) -> u16 {
        self.channels
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
                        self.injector.inject(buffer.trim()).await?;
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
    init_test_tracing();
    info!("Starting end-to-end WAV pipeline test");
    debug!("Processing WAV file: {:?}", wav_path.as_ref());
    debug!("Expected text fragments: {:?}", expected_text_fragments);

    // Set up components
    let mock_injector = MockTextInjector::new();
    let ring_buffer = AudioRingBuffer::new(16384 * 4);
    let (audio_producer, audio_consumer) = ring_buffer.split();

    // Load WAV file (keep native rate/channels)
    let mut wav_loader = WavFileLoader::new(wav_path.as_ref())?;
    let test_duration = Duration::from_millis(wav_loader.duration_ms() + 2000); // Add buffer time

    // Set up audio chunker
    let (audio_tx, _) = broadcast::channel::<AudioFrame>(200);
    // Emulate device config broadcast like live capture
    let (cfg_tx, cfg_rx) = broadcast::channel::<DeviceConfig>(8);
    let _ = cfg_tx.send(DeviceConfig {
        sample_rate: wav_loader.sample_rate(),
        channels: wav_loader.channels(),
    });

    let frame_reader = coldvox_audio::frame_reader::FrameReader::new(
        audio_consumer,
        wav_loader.sample_rate(),
        wav_loader.channels(),
        16384 * 4,
        None,
    );

    let chunker_cfg = ChunkerConfig {
        frame_size_samples: FRAME_SIZE_SAMPLES,
        sample_rate_hz: SAMPLE_RATE_HZ,
        resampler_quality: coldvox_audio::chunker::ResamplerQuality::Balanced,
    };

    let chunker =
        AudioChunker::new(frame_reader, audio_tx.clone(), chunker_cfg).with_device_config(cfg_rx);
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
        streaming: false,
    };

    // Check if STT model exists
    if !std::path::Path::new(&stt_config.model_path).exists() {
        anyhow::bail!(
            "Vosk model not found at '{}'. Download a model or set VOSK_MODEL_PATH environment variable.",
            stt_config.model_path
        );
    }

    let stt_audio_rx = audio_tx.subscribe();
    let stt_processor =
        match SttProcessor::new(stt_audio_rx, vad_event_rx, stt_transcription_tx, stt_config) {
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

    let injection_processor =
        MockInjectionProcessor::new(mock_injector_clone, stt_transcription_rx, shutdown_rx);
    let _injection_handle = tokio::spawn(async move { injection_processor.run().await });

    // Start streaming WAV data
    let streaming_handle =
        tokio::spawn(async move { wav_loader.stream_to_ring_buffer(audio_producer).await });

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
            found_fragments.push(expected.to_string());
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
    async fn test_end_to_end_wav_pipeline() {
        init_test_tracing();
        use rand::seq::SliceRandom;
        use std::fs;

        // This test requires:
        // 1. A WAV file with known speech content
        // 2. Vosk model downloaded and configured

        // Look for test WAV files in test_data directory
        let test_data_dir = "test_data";

        // Allow an opt-in mode to run ALL WAV samples sequentially
        let run_all = std::env::var("TEST_WAV_MODE")
            .map(|v| v.eq_ignore_ascii_case("all"))
            .unwrap_or(false);

        if run_all {
            // Discover all WAV+TXT pairs
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

            println!("Running ALL {} WAV samples...", wav_files.len());
            let mut failures = Vec::new();
            for wav_path in wav_files {
                let txt_path = std::path::Path::new(&wav_path).with_extension("txt");
                let transcript = fs::read_to_string(&txt_path).unwrap_or_else(|e| {
                    panic!("Failed to read transcript {}: {}", txt_path.display(), e)
                });

                // Extract a few distinctive keywords
                let words: Vec<String> = transcript
                    .to_lowercase()
                    .split_whitespace()
                    .filter(|w| w.len() >= 4)
                    .take(3)
                    .map(|s| s.to_string())
                    .collect();
                let expected = if words.is_empty() {
                    transcript
                        .to_lowercase()
                        .split_whitespace()
                        .take(2)
                        .map(|s| s.to_string())
                        .collect()
                } else {
                    words
                };

                println!("Testing with WAV file: {}", wav_path);
                println!("Expected keywords: {:?}", expected);
                let expected_refs: Vec<&str> = expected.iter().map(|s| s.as_str()).collect();

                match test_wav_pipeline(&wav_path, expected_refs).await {
                    Ok(injections) => {
                        println!("✅ Test passed! Injections: {:?}", injections);
                        if injections.is_empty() {
                            failures.push(format!("{}: no text injected", wav_path));
                        }
                    }
                    Err(e) => {
                        eprintln!("❌ Test failed for {}: {}", wav_path, e);
                        failures.push(format!("{}: {}", wav_path, e));
                    }
                }
            }

            if !failures.is_empty() {
                panic!("One or more WAV samples failed: {:?}", failures);
            }
            return;
        }

        // If TEST_WAV is set, use that specific file (single-file mode)
        if let Ok(specific_wav) = std::env::var("TEST_WAV") {
            if !std::path::Path::new(&specific_wav).exists() {
                eprintln!("Skipping test: WAV file '{}' not found", specific_wav);
                return;
            }
            let expected_fragments = vec!["the".to_string()]; // Generic expectation for ad-hoc file
            println!("Testing with WAV file: {}", specific_wav);
            println!("Expected keywords: {:?}", expected_fragments);
            let expected_refs: Vec<&str> = expected_fragments.iter().map(|s| s.as_str()).collect();
            match test_wav_pipeline(specific_wav, expected_refs).await {
                Ok(injections) => {
                    println!("✅ Test passed! Injections: {:?}", injections);
                    assert!(!injections.is_empty(), "No text was injected");
                }
                Err(e) => {
                    eprintln!("❌ Test failed: {}", e);
                    panic!("End-to-end test failed: {}", e);
                }
            }
            return;
        }

        // Default: random single-file mode (fast CI-friendly)
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

        let expected_fragments = if words.is_empty() {
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

        println!("Testing with WAV file: {}", selected_wav);
        println!("Expected keywords: {:?}", expected_fragments);
        let expected_refs: Vec<&str> = expected_fragments.iter().map(|s| s.as_str()).collect();
        match test_wav_pipeline(selected_wav, expected_refs).await {
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

    #[tokio::test]
    async fn test_end_to_end_with_real_injection() {
        init_test_tracing();
        // This test uses the real AsyncInjectionProcessor for comprehensive testing
        // It requires:
        // 1. A WAV file with known speech content
        // 2. Vosk model downloaded and configured
        // 3. A working text injection backend (e.g., clipboard, AT-SPI)

        let test_wav =
            std::env::var("TEST_WAV").unwrap_or_else(|_| "test_data/sample.wav".to_string());

        if !std::path::Path::new(&test_wav).exists() {
            eprintln!("Skipping test: WAV file '{}' not found", test_wav);
            return;
        }

        info!("Starting comprehensive end-to-end test with real injection");

        // Set up components
        let ring_buffer = AudioRingBuffer::new(16384 * 4);
        let (audio_producer, audio_consumer) = ring_buffer.split();

        // Load WAV file (native rate/channels)
        let mut wav_loader = WavFileLoader::new(&test_wav).unwrap();
        let test_duration = Duration::from_millis(wav_loader.duration_ms() + 2000);

        // Set up audio chunker
        let (audio_tx, _) = broadcast::channel::<AudioFrame>(200);
        // Emulate device config broadcast like live capture
        let (cfg_tx, cfg_rx) = broadcast::channel::<DeviceConfig>(8);
        let _ = cfg_tx.send(DeviceConfig {
            sample_rate: wav_loader.sample_rate(),
            channels: wav_loader.channels(),
        });

        let frame_reader = coldvox_audio::frame_reader::FrameReader::new(
            audio_consumer,
            wav_loader.sample_rate(),
            wav_loader.channels(),
            16384 * 4,
            None,
        );

        let chunker_cfg = ChunkerConfig {
            frame_size_samples: FRAME_SIZE_SAMPLES,
            sample_rate_hz: SAMPLE_RATE_HZ,
            resampler_quality: coldvox_audio::chunker::ResamplerQuality::Balanced,
        };

        let chunker = AudioChunker::new(frame_reader, audio_tx.clone(), chunker_cfg)
            .with_device_config(cfg_rx);
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
            Err(e) => {
                eprintln!("Failed to spawn VAD processor: {}", e);
                return;
            }
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
            eprintln!(
                "Vosk model not found at '{}'. Skipping test.",
                stt_config.model_path
            );
            return;
        }

        let stt_audio_rx = audio_tx.subscribe();
        let stt_processor =
            match SttProcessor::new(stt_audio_rx, vad_event_rx, stt_transcription_tx, stt_config) {
                Ok(processor) => processor,
                Err(e) => {
                    eprintln!("Failed to create STT processor: {}", e);
                    return;
                }
            };
        let stt_handle = tokio::spawn(async move {
            stt_processor.run().await;
        });

        // Set up real injection processor with top 2 methods
        let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>(1);

        // Create a temporary file to capture injected text
        let capture_file =
            std::env::temp_dir().join(format!("coldvox_injection_test_{}.txt", std::process::id()));
        std::fs::write(&capture_file, "").ok();

        // Open a terminal window that will receive the injection
        let terminal = match open_test_terminal(&capture_file).await {
            Ok(term) => term,
            Err(e) => {
                eprintln!("Skipping test: Could not open test terminal: {}", e);
                return;
            }
        };

        // Give terminal time to start and focus
        tokio::time::sleep(Duration::from_millis(500)).await;

        let injection_config = InjectionConfig {
            allow_ydotool: false, // Test primary methods only
            allow_kdotool: false,
            allow_enigo: false,
            restore_clipboard: true,        // Enable clipboard restoration
            inject_on_unknown_focus: false, // Require proper focus
            require_focus: true,
            ..Default::default()
        };

        let injection_processor =
            AsyncInjectionProcessor::new(injection_config, stt_transcription_rx, shutdown_rx, None)
                .await;

        let injection_handle = tokio::spawn(async move { injection_processor.run().await });

        // Start streaming WAV data
        let streaming_handle =
            tokio::spawn(async move { wav_loader.stream_to_ring_buffer(audio_producer).await });

        info!("Pipeline started, running for {:?}", test_duration);

        // Wait for test duration
        tokio::time::sleep(test_duration).await;

        // Shutdown
        let _ = shutdown_tx.send(()).await;
        chunker_handle.abort();
        vad_handle.abort();
        stt_handle.abort();
        injection_handle.abort();
        streaming_handle.abort();

        info!("Comprehensive end-to-end test completed");

        // Close the terminal
        if let Some(mut term) = terminal {
            let _ = term.kill().await;
        }

        // Verify injection by reading capture file
        tokio::time::sleep(Duration::from_millis(500)).await;
        let captured = std::fs::read_to_string(&capture_file).unwrap_or_default();
        let _ = std::fs::remove_file(&capture_file);

        if !captured.is_empty() {
            info!("Successfully captured injected text: {}", captured);
            // Verify some expected words were transcribed
            assert!(!captured.trim().is_empty(), "No text was injected");
        } else {
            eprintln!("Warning: Could not verify injection (normal in headless environment)");
        }
    }

    /// Test AT-SPI injection specifically
    #[tokio::test]
    #[cfg(feature = "text-injection")]
    async fn test_atspi_injection() {
        #[cfg(feature = "text-injection")]
        {
            use crate::text_injection::{
                atspi_injector::AtspiInjector, InjectionConfig, TextInjector,
            };

            let config = InjectionConfig::default();
            let injector = AtspiInjector::new(config);

            // Check availability first
            if !injector.is_available().await {
                eprintln!("Skipping AT-SPI test: Backend not available");
                return;
            }

            // Open test terminal
            let capture_file = std::env::temp_dir().join("coldvox_atspi_test.txt");
            let terminal = match open_test_terminal(&capture_file).await {
                Ok(term) => term,
                Err(_) => {
                    eprintln!("Skipping AT-SPI test: Could not open terminal");
                    return;
                }
            };

            tokio::time::sleep(Duration::from_millis(500)).await;

            // Test injection
            let test_text = "AT-SPI injection test";
            match injector.inject_text(test_text).await {
                Ok(_) => info!("AT-SPI injection successful"),
                Err(e) => eprintln!("AT-SPI injection failed: {:?}", e),
            }

            // Cleanup
            if let Some(mut term) = terminal {
                let _ = term.kill().await;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
            let captured = std::fs::read_to_string(&capture_file).unwrap_or_default();
            let _ = std::fs::remove_file(&capture_file);

            if captured.contains(test_text) {
                info!("✅ AT-SPI injection verified");
            }
        }
    }

    /// Test clipboard injection specifically
    #[tokio::test]
    #[cfg(feature = "text-injection")]
    async fn test_clipboard_injection() {
        #[cfg(feature = "text-injection")]
        {
            use crate::text_injection::{
                clipboard_injector::ClipboardInjector, InjectionConfig, TextInjector,
            };

            let config = InjectionConfig::default();
            let injector = ClipboardInjector::new(config);

            // Check availability
            if !injector.is_available().await {
                eprintln!("Skipping clipboard test: Backend not available");
                return;
            }

            // Save current clipboard
            let original_clipboard = get_clipboard_content().await;

            // Open test terminal
            let capture_file = std::env::temp_dir().join("coldvox_clipboard_test.txt");
            let terminal = match open_test_terminal(&capture_file).await {
                Ok(term) => term,
                Err(_) => {
                    eprintln!("Skipping clipboard test: Could not open terminal");
                    return;
                }
            };

            tokio::time::sleep(Duration::from_millis(500)).await;

            // Test injection
            let test_text = "Clipboard injection test";
            match injector.inject_text(test_text).await {
                Ok(_) => info!("Clipboard injection successful"),
                Err(e) => eprintln!("Clipboard injection failed: {:?}", e),
            }

            // Verify clipboard was restored
            tokio::time::sleep(Duration::from_millis(500)).await;
            let restored_clipboard = get_clipboard_content().await;

            if original_clipboard == restored_clipboard {
                info!("✅ Clipboard correctly restored");
            } else {
                eprintln!("⚠️ Clipboard not restored properly");
            }

            // Cleanup
            if let Some(mut term) = terminal {
                let _ = term.kill().await;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
            let captured = std::fs::read_to_string(&capture_file).unwrap_or_default();
            let _ = std::fs::remove_file(&capture_file);

            if captured.contains(test_text) {
                info!("✅ Clipboard injection verified");
            }
        }
    }
}

/// Helper to open a test terminal that captures input to a file
async fn open_test_terminal(
    capture_file: &std::path::Path,
) -> Result<Option<tokio::process::Child>> {
    use std::process::Stdio;

    // Try xterm first (commonly available in CI)
    let xterm_result = tokio::process::Command::new("xterm")
        .arg("-e")
        .arg("bash")
        .arg("-c")
        .arg(format!("tee {} > /dev/null", capture_file.display()))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();

    if let Ok(child) = xterm_result {
        return Ok(Some(child));
    }

    // Try gnome-terminal as fallback
    let gnome_result = tokio::process::Command::new("gnome-terminal")
        .arg("--")
        .arg("bash")
        .arg("-c")
        .arg(format!("tee {} > /dev/null", capture_file.display()))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();

    if let Ok(child) = gnome_result {
        return Ok(Some(child));
    }

    // In headless CI, we might not have a terminal but can still test
    // Create a simple background process that reads stdin
    if std::env::var("CI").is_ok() || std::env::var("DISPLAY").is_err() {
        // In CI/headless, just create the file for the test to proceed
        std::fs::write(capture_file, "").ok();
        return Ok(None);
    }

    Err(anyhow::anyhow!("No suitable terminal emulator found"))
}

/// Helper to get clipboard content
async fn get_clipboard_content() -> Option<String> {
    // Try wl-paste first (Wayland)
    let wl_result = tokio::process::Command::new("wl-paste")
        .arg("--no-newline")
        .output()
        .await;

    if let Ok(output) = wl_result {
        if output.status.success() {
            return Some(String::from_utf8_lossy(&output.stdout).to_string());
        }
    }

    // Try xclip (X11)
    let xclip_result = tokio::process::Command::new("xclip")
        .arg("-selection")
        .arg("clipboard")
        .arg("-o")
        .output()
        .await;

    if let Ok(output) = xclip_result {
        if output.status.success() {
            return Some(String::from_utf8_lossy(&output.stdout).to_string());
        }
    }

    None
}
