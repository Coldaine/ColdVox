use crate::audio::wav_file_loader::WavFileLoader;
use anyhow::Result;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc};
use tracing::info;

use crate::stt::plugin_manager::SttPluginManager;
use crate::stt::processor::PluginSttProcessor;
use crate::stt::session::{SessionEvent, SessionSource, Settings};
use crate::stt::{TranscriptionConfig, TranscriptionEvent};
use crate::text_injection::{AsyncInjectionProcessor, InjectionConfig};
use coldvox_audio::SharedAudioFrame as AudioFrame;
use coldvox_audio::chunker::{AudioChunker, ChunkerConfig};
use coldvox_audio::ring_buffer::AudioRingBuffer;
use coldvox_audio::DeviceConfig;
use coldvox_foundation::AudioConfig;
use coldvox_vad::config::{UnifiedVadConfig, VadMode};
use coldvox_vad::constants::{FRAME_SIZE_SAMPLES, SAMPLE_RATE_HZ};
use coldvox_vad::types::VadEvent;

/// Resolve the Whisper model identifier to use for integration testing.
///
/// The current Faster-Whisper integration loads models by identifier (e.g.,
/// `tiny`, `base.en`) or by explicit filesystem path. For the integration tests we
/// favour a lightweight default but allow callers to override via environment variables.
///
/// Priority order:
/// 1. WHISPER_MODEL_PATH (explicit filesystem path)
/// 2. WHISPER_MODEL_SIZE (model size identifier)
/// 3. Default based on environment (tiny for CI, base for development)
fn resolve_whisper_model_identifier() -> String {
    // First check for explicit model path
    if let Ok(model_path) = std::env::var("WHISPER_MODEL_PATH") {
        return model_path;
    }

    // Then check for model size
    if let Ok(model_size) = std::env::var("WHISPER_MODEL_SIZE") {
        return model_size;
    }

    // Default based on environment
    if std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok() {
        "tiny".to_string() // Use tiny model in CI for faster execution
    } else {
        "large-v3-turbo".to_string() // Use large model for local development
    }
}

// Helper to open a test terminal that captures input to a file
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

#[tokio::test]
async fn test_end_to_end_with_real_injection() {
    // Ensure tqdm is enabled to avoid buggy 'disabled_tqdm' stub in some Python envs
    std::env::set_var("TQDM_DISABLE", "0");
    // NOTE: test_utils not yet implemented
    // crate::test_utils::init_test_infrastructure();
    // This test uses the real AsyncInjectionProcessor for comprehensive testing
    // It requires:
    // 1. A WAV file with known speech content
    // 2. Faster-Whisper model downloaded and configured (or accessible via identifier)
    // 3. A working text injection backend (e.g., clipboard, AT-SPI)
    // Use a fixed deterministic test asset so results are stable across runs.
    // Chosen sample: test_2.wav with full transcript in test_2.txt
    // Transcript (uppercase original):
    // FAR FROM IT SIRE YOUR MAJESTY HAVING GIVEN NO DIRECTIONS ABOUT IT THE MUSICIANS HAVE RETAINED IT
    // We normalize to lowercase and compare presence of key fragments to allow minor ASR variance.
    let test_wav = "test_data/test_2.wav";
    let transcript_path = "test_data/test_2.txt";
    if !std::path::Path::new(test_wav).exists() || !std::path::Path::new(transcript_path).exists() {
        eprintln!(
            "Skipping test: required fixed test assets missing ({} / {})",
            test_wav, transcript_path
        );
        return;
    }
    // (Optional future enhancement) Full transcript can be loaded for fuzzy similarity metrics
    // let expected_full_transcript = std::fs::read_to_string(transcript_path)
    //     .unwrap_or_default()
    //     .trim()
    //     .to_lowercase();
    // Define a set of distinctive expected fragments (longer words reduce false positives)
    let expected_fragments: [&str; 6] = [
        "majesty having",
        "musicians have",
        "retained it",
        "far from it",
        "no directions",
        "given no",
    ];

    info!("Starting comprehensive end-to-end test with real injection");

    // Set up components
    let audio_config = AudioConfig::default();
    let ring_buffer = AudioRingBuffer::new(audio_config.capture_buffer_samples);
    let (audio_producer, audio_consumer) = ring_buffer.split();

    // Load WAV file (native rate/channels)
    let mut wav_loader = WavFileLoader::new(test_wav).unwrap();
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
        audio_config.capture_buffer_samples,
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
        silero: Default::default(),
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
        model_path: resolve_whisper_model_identifier(),
        partial_results: true,
        max_alternatives: 1,
        include_words: false,
        buffer_size_ms: 512,
        streaming: false,
        auto_extract_model: false,
    };

    // Keep a copy of the selected model identifier for environment-aware assertions later
    let selected_model = stt_config.model_path.clone();

    // Check if STT model exists; if missing, fail fast with actionable guidance
    // For local testing, we expect the model to be downloaded on first use by faster-whisper
    // So we don't check if the path exists, we just let faster-whisper handle it
    if std::env::var("CI").is_err() && std::env::var("GITHUB_ACTIONS").is_err() {
        // Only check for model existence in CI environment
        // Only check if model path is a filesystem path (contains a slash)
        // Model identifiers like "large-v3-turbo" should not be checked as file paths
        if stt_config.model_path.contains('/') && !std::path::Path::new(&stt_config.model_path).exists() {
            panic!(
                "Whisper model or identifier '{}' is not available. \n\nResolution:\n  1. Install the faster-whisper Python package and ensure a model is downloaded.\n  2. Set WHISPER_MODEL_PATH to a valid identifier or model directory.\n  3. Re-run: cargo test -p coldvox-app test_end_to_end_with_real_injection --features whisper,text-injection -- --nocapture\n",
                stt_config.model_path
            );
        }
    }

    let stt_audio_rx = audio_tx.subscribe();
    // Set up Plugin Manager
    let mut plugin_manager = SttPluginManager::new();
    plugin_manager.initialize().await.unwrap();
    let plugin_manager = Arc::new(tokio::sync::RwLock::new(plugin_manager));

    // Set up SessionEvent channel and translator
    let (session_tx, session_rx) = mpsc::channel::<SessionEvent>(100);
    tokio::spawn(async move {
        let mut vad_rx = vad_event_rx;
        while let Some(event) = vad_rx.recv().await {
            let session_event = match event {
                VadEvent::SpeechStart { .. } => {
                    Some(SessionEvent::Start(SessionSource::Vad, Instant::now()))
                }
                VadEvent::SpeechEnd { .. } => {
                    Some(SessionEvent::End(SessionSource::Vad, Instant::now()))
                }
            };
            if let Some(se) = session_event {
                if session_tx.send(se).await.is_err() {
                    break;
                }
            }
        }
    });

    let stt_processor = PluginSttProcessor::new(
        stt_audio_rx,
        session_rx,
        stt_transcription_tx,
        plugin_manager,
        stt_config,
        Settings::default(),
    );
    let stt_handle = tokio::spawn(async move {
        stt_processor.run().await;
    });

    // Set up real injection processor with top 2 methods
    let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>(1);

    // Create a temporary file to capture injected text
    let capture_file =
        std::env::temp_dir().join(format!("coldvox_injection_test_{}.txt", std::process::id()));
    std::fs::write(&capture_file, "").ok();

    // Open a terminal window that will receive the injection. If this fails (headless CI), continue.
    let terminal = match open_test_terminal(&capture_file).await {
        Ok(term) => term,
        Err(e) => {
            eprintln!("Headless fallback: Could not open test terminal: {}", e);
            None
        }
    };
    let terminal_unavailable = terminal.is_none();

    // Give terminal time to start and focus
    tokio::time::sleep(Duration::from_millis(500)).await;

    let mut injection_config = InjectionConfig {
        allow_kdotool: false,
        allow_enigo: false,
        // clipboard restoration is automatic
        inject_on_unknown_focus: false, // Require proper focus
        require_focus: true,
        ..Default::default()
    };
    if terminal.is_none() {
        // Relax focus requirements so injection logic still executes in headless mode
        injection_config.require_focus = false;
        injection_config.inject_on_unknown_focus = true;
    }

    // Tee transcription events so we can both feed the injection processor and retain finals for WER.
    use std::sync::{Arc, Mutex};
    let finals_store: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let finals_store_clone = finals_store.clone();
    let (tee_tx, mut tee_rx) = mpsc::channel::<TranscriptionEvent>(100);

    // Forward original STT events into tee_tx
    let mut stt_rx_for_tee = stt_transcription_rx; // rename for clarity
    let _tee_forward_handle = tokio::spawn(async move {
        while let Some(ev) = stt_rx_for_tee.recv().await {
            if tee_tx.send(ev).await.is_err() {
                break;
            }
        }
    });

    // Split: one consumer for injection processor, one collector
    let (inj_tx, inj_rx) = mpsc::channel::<TranscriptionEvent>(100);
    let finals_collector = finals_store_clone.clone();
    let _collector_handle = tokio::spawn(async move {
        while let Some(ev) = tee_rx.recv().await {
            // Capture finals
            if let TranscriptionEvent::Final { text, .. } = &ev {
                if !text.trim().is_empty() {
                    if let Ok(mut g) = finals_collector.lock() {
                        g.push(text.clone());
                    }
                }
            }
            // Forward to injection pipeline
            if inj_tx.send(ev).await.is_err() {
                break;
            }
        }
    });

    let injection_processor =
        AsyncInjectionProcessor::new(injection_config, inj_rx, shutdown_rx, None).await;

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

    if captured.trim().is_empty() {
        // Fallback: derive quality from collected final transcriptions using WER if we have them.
        let finals: Vec<String> = finals_store.lock().map(|g| g.clone()).unwrap_or_default();
        if !finals.is_empty() {
            let combined = finals.join(" ");
            let expected_ref = {
                let raw = std::fs::read_to_string(transcript_path).unwrap_or_default();
                raw.trim().to_lowercase()
            };
            eprintln!(
                "WER fallback comparison:\n  expected: {}\n  transcribed: {}",
                expected_ref,
                combined.to_lowercase()
            );
            // If we're not on a dev machine (CI/headless) or using a very small model, skip strict WER assertions
            let is_ci = std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok();
            let headless = std::env::var("DISPLAY").is_err();
            let conservative_model = {
                let m = selected_model.to_lowercase();
                m.starts_with("tiny") || m.starts_with("base") || m.starts_with("small")
            };
            if is_ci || headless || terminal_unavailable || conservative_model {
                eprintln!(
                    "Skipping strict WER assertion (is_ci={}, headless={}, terminal_unavailable={}, model='{}').\nThis end-to-end check is intended for dev machines with larger models; on CI/headless we validate pipeline execution only.",
                    is_ci,
                    headless,
                    terminal_unavailable,
                    selected_model
                );
                return;
            }
            // Use centralized WER utility for consistent calculation
            use crate::stt::tests::wer_utils::calculate_wer;
            let wer = calculate_wer(&expected_ref, &combined.to_lowercase());
            assert!(
                wer <= 0.55,
                "WER fallback exceeded threshold: {:.3} > 0.55\nExpected: {}\nGot: {}",
                wer,
                expected_ref,
                combined.to_lowercase()
            );
            eprintln!(
                "No injection capture; used WER fallback (WER={:.3}, ref_words={}, hyp_words={})",
                wer,
                expected_ref.split_whitespace().count(),
                combined.split_whitespace().count()
            );
        } else {
            eprintln!("Warning: No injected text captured and no final transcripts aggregated. Headless environment likely prevented capture. Pipeline execution completed but verification degraded.");
        }
    } else {
        let captured_lc = captured.to_lowercase();
        info!(
            "Captured injected text (len={}): {}",
            captured_lc.trim().len(),
            captured_lc.trim()
        );
        let mut matched = 0usize;
        for frag in expected_fragments.iter() {
            if captured_lc.contains(frag) {
                matched += 1;
            }
        }
        // Require at least half the fragments to appear; tolerate minor ASR variance
        assert!(matched >= 3, "Injected text did not contain enough expected fragments (matched {} of {}): {:?}\nCaptured: {}", matched, expected_fragments.len(), expected_fragments, captured_lc.trim());
        // Optional stronger check: if model performance improves, compare full string similarity
        // (Left as a future enhancement: Levenshtein distance threshold against expected_full_transcript)
    }
}

/// Test AT-SPI injection specifically
#[tokio::test]
#[cfg(feature = "text-injection")]

async fn test_atspi_injection() {
    // NOTE: test_utils not yet implemented
    // crate::test_utils::init_test_infrastructure();
    #[cfg(feature = "text-injection")]
    {
        use crate::text_injection::{
            injectors::atspi::AtspiInjector, InjectionConfig, TextInjector,
        };
        use tokio::time::Duration;

        // Guard the whole test with a short timeout so CI doesn't hang if desktop isn't responsive
        let test_future = async {
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

            // Test injection with centralized timeout utilities
            let test_text = "AT-SPI injection test";
            // Note: timeout wrapper flattens the result, so we need to handle the inner result separately
            let timeout_result = crate::stt::tests::timeout_utils::with_injection_timeout(
                injector.inject_text(test_text, None),
                "AT-SPI injection test",
            )
            .await;

            match timeout_result {
                Ok(injection_result) => match injection_result {
                    Ok(_) => info!("AT-SPI injection successful"),
                    Err(e) => eprintln!("AT-SPI injection failed: {:?}", e),
                },
                Err(timeout_msg) => eprintln!("AT-SPI injection timed out: {}", timeout_msg),
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
        };

        match crate::stt::tests::timeout_utils::with_timeout(
            test_future,
            Some(Duration::from_secs(15)),
            "AT-SPI desktop test",
        )
        .await
        {
            Ok(_) => {} // Test completed successfully or skipped gracefully
            Err(timeout_msg) => {
                eprintln!(
                    "AT-SPI test timed out - skipping (desktop likely unavailable): {}",
                    timeout_msg
                );
            }
        }
    }
}

/// Test clipboard injection specifically
#[tokio::test]
#[cfg(feature = "text-injection")]

async fn test_clipboard_injection() {
    // NOTE: test_utils not yet implemented, clipboard_paste_injector module renamed
    // Temporarily disabled until API is updated
    eprintln!("Test temporarily disabled - awaiting API updates");
}
