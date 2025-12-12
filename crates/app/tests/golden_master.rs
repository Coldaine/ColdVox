//! Golden Master testing harness for the ColdVox pipeline.
//!
//! This testing methodology is based on the concept of "approval testing" or
//! "golden master testing". It works by capturing the output of a system and
//! comparing it to a previously approved "golden master" file.
//!
//! ## Workflow
//!
//! 1. **Run the test:** A test is executed that runs the full application pipeline
//!    with a deterministic input (e.g., a specific WAV file).
//!
//! 2. **Capture output:** The test captures key outputs at specific "anchor points"
//!    in the pipeline (e.g., VAD events, final transcribed text).
//!
//! 3. **Serialize and compare:** The captured output is serialized to a
//!    `.received.json` file. The test harness then compares this file to a
//!    corresponding `.approved.json` file.
//!
//! 4. **Assert:**
//!    - If the files match, the test passes.
//!    - If the files do not match, the test fails and prints a rich diff of the
//!      changes.
//!
//! 5. **Approval:**
//!    - If a change is intentional (e.g., due to a feature change or a bug fix),
//!      the developer can "approve" the new output by copying the `.received.json`
//!      file over the `.approved.json` file and committing the change to Git.
//!      This establishes a new baseline for future test runs.
//!    - If the change is unintentional, it indicates a regression that must be fixed.
//!
//! This approach is powerful for testing complex systems with intricate outputs,
//! as it separates the act of running the test from the act of verifying the output.

pub mod harness {
    use serde::de::DeserializeOwned;
    use serde::Serialize;
    use std::fs;
    use std::path::PathBuf;

    /// Returns the path to the directory where test artifacts are stored.
    ///
    /// This will be `crates/app/tests/golden_master_artifacts/`.
    fn artifacts_dir() -> PathBuf {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests");
        path.push("golden_master_artifacts");
        path
    }

    /// Constructs the file path for a given test artifact.
    ///
    /// - `test_name`: A unique identifier for the test case.
    /// - `anchor`: The specific anchor point in the pipeline (e.g., "vad", "injection").
    /// - `kind`: Either "approved" or "received".
    fn get_artifact_path(test_name: &str, anchor: &str, kind: &str) -> PathBuf {
        let dir = artifacts_dir();
        let filename = format!("{}.{}.{}.json", test_name, anchor, kind);
        dir.join(filename)
    }

    /// Asserts that the received value matches the approved golden master.
    ///
    /// - `test_name`: A unique identifier for the test case (e.g., "short_phrase").
    /// - `anchor`: The name of the pipeline anchor point being tested (e.g., "vad").
    /// - `received_value`: The value captured from the latest test run, which must
    ///   be serializable to JSON.
    pub fn assert_golden<T>(test_name: &str, anchor: &str, received_value: &T)
    where
        T: Serialize + DeserializeOwned + std::fmt::Debug + PartialEq,
    {
        let received_path = get_artifact_path(test_name, anchor, "received");
        let approved_path = get_artifact_path(test_name, anchor, "approved");

        // Ensure the artifacts directory exists.
        fs::create_dir_all(artifacts_dir()).unwrap();

        // Serialize the received value to its file.
        let received_json = serde_json::to_string_pretty(received_value)
            .expect("Failed to serialize received value");
        fs::write(&received_path, &received_json).expect("Failed to write received artifact");

        // Check if the approved file exists.
        if !approved_path.exists() {
            panic!(
                "Golden master file not found for test '{}', anchor '{}'.\n\
                Approve the received output by running:\n\
                cp {} {}",
                test_name,
                anchor,
                received_path.display(),
                approved_path.display()
            );
        }

        // Read the approved file.
        let approved_json =
            fs::read_to_string(&approved_path).expect("Failed to read approved artifact");

        // Deserialize and compare using similar-asserts for a nice diff.
        let approved_value: T =
            serde_json::from_str(&approved_json).expect("Failed to deserialize approved value");

        // For VAD events we occasionally see frame boundary jitter causing
        // minor duration differences. If both approved and received are arrays
        // of SerializableVadEvent, apply a tolerance for SpeechEnd duration.
        let mismatch = if anchor == "vad" {
            // Custom tolerant comparison.
            use serde_json::Value;
            let received_val: Value = serde_json::to_value(received_value).unwrap();
            let approved_val: Value = serde_json::to_value(&approved_value).unwrap();
            let tolerant = match (approved_val, received_val) {
                (Value::Array(a), Value::Array(b)) if a.len() == b.len() => {
                    // Iterate and compare per element; tolerate SpeechEnd duration diff <= 128ms
                    let mut all_ok = true;
                    for (av, bv) in a.iter().zip(b.iter()) {
                        match (av, bv) {
                            (Value::Object(ao), Value::Object(bo)) => {
                                let kind_a = ao.get("kind").and_then(|v| v.as_str()).unwrap_or("");
                                let kind_b = bo.get("kind").and_then(|v| v.as_str()).unwrap_or("");
                                if kind_a != kind_b {
                                    all_ok = false;
                                    break;
                                }
                                if kind_a == "SpeechEnd" {
                                    let da =
                                        ao.get("duration_ms").and_then(|v| v.as_u64()).unwrap_or(0);
                                    let db =
                                        bo.get("duration_ms").and_then(|v| v.as_u64()).unwrap_or(0);
                                    let diff = da.abs_diff(db);
                                    if diff > 128 {
                                        all_ok = false;
                                        break;
                                    }
                                } else if kind_a == "SpeechStart" {
                                    // SpeechStart has no duration, ignore
                                } else {
                                    // Unknown kind fallback to strict equality
                                    if av != bv {
                                        all_ok = false;
                                        break;
                                    }
                                }
                            }
                            _ => {
                                if av != bv {
                                    all_ok = false;
                                    break;
                                }
                            }
                        }
                    }
                    all_ok
                }
                _ => false,
            };
            !tolerant
        } else {
            approved_value != *received_value
        };

        if mismatch {
            similar_asserts::assert_eq!(
                approved_value,
                *received_value,
                "Golden master mismatch for test '{}', anchor '{}'.\n\
                If the change is intentional, approve it with:\n\
                cp {} {}",
                test_name,
                anchor,
                received_path.display(),
                approved_path.display()
            );
        }
    }
}

mod common;

#[cfg(test)]
mod tests {
    use super::harness::assert_golden;
    use super::test_utils::MockInjectionSink;
    use crate::common::logging::init_test_logging;
    use coldvox_app::audio::wav_file_loader::WavFileLoader;
    use coldvox_app::runtime::{start, ActivationMode, AppRuntimeOptions};
    use coldvox_audio::DeviceConfig;
    use coldvox_stt::plugin::{FailoverConfig, GcPolicy, PluginSelectionConfig};
    use coldvox_vad::VadEvent;
    use serde::{Deserialize, Serialize};
    use std::sync::Arc;
    use std::time::Duration;

    // A serializable representation of VadEvent for stable golden master files.
    #[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
    struct SerializableVadEvent {
        kind: String,
        duration_ms: Option<u64>,
    }

    impl From<VadEvent> for SerializableVadEvent {
        fn from(event: VadEvent) -> Self {
            match event {
                VadEvent::SpeechStart { .. } => Self {
                    kind: "SpeechStart".to_string(),
                    duration_ms: None,
                },
                VadEvent::SpeechEnd { duration_ms, .. } => Self {
                    kind: "SpeechEnd".to_string(),
                    // Normalize to reduce flakiness from scheduling/frame timing jitter.
                    // Round to the nearest 64ms bucket (half-up).
                    duration_ms: Some({
                        const BUCKET: u64 = 64;
                        ((duration_ms + BUCKET / 2) / BUCKET) * BUCKET
                    }),
                },
            }
        }
    }

    #[tokio::test]
    async fn test_short_phrase_pipeline() {
        // Initialize comprehensive file-based logging
        let _guard = init_test_logging("golden_master_short_phrase");

        let test_name = "short_phrase";
        let wav_path = "test_data/test_11.wav";

        tracing::info!("Starting golden master test for: {}", test_name);

        // 1. Set up the mock injection sink.
        let mock_sink = Arc::new(MockInjectionSink::new());

        // 2. Configure the runtime for a black-box test run.
        let mut wav_loader = WavFileLoader::new(wav_path)
            .unwrap_or_else(|e| panic!("Failed to load WAV file '{}': {}", wav_path, e));

        tracing::info!(
            "Loaded WAV file: {} Hz, {} channels",
            wav_loader.sample_rate(),
            wav_loader.channels()
        );

        // If the whisper feature isn't enabled, fall back to mock plugin.
        #[cfg(feature = "whisper")]
        let transcription_config = coldvox_stt::TranscriptionConfig {
            enabled: true,
            model_path: "tiny.en".to_string(),
            ..Default::default()
        };

        #[cfg(not(feature = "whisper"))]
        let transcription_config = coldvox_stt::TranscriptionConfig {
            enabled: true,
            // Use a placeholder path; mock plugin ignores it.
            model_path: "mock".to_string(),
            ..Default::default()
        };

        // Configure VAD with higher threshold for test WAV files
        // Default threshold (0.3) is too sensitive and detects noise as speech
        let vad_config = coldvox_vad::config::UnifiedVadConfig {
            mode: coldvox_vad::config::VadMode::Silero,
            silero: coldvox_vad::config::SileroConfig {
                threshold: 0.5,              // Higher threshold = less sensitive, better for test WAVs
                min_speech_duration_ms: 100, // Reduced from default 250ms
                min_silence_duration_ms: 300, // Increased from default 100ms for cleaner end detection
                window_size_samples: 512,
            },
            frame_size_samples: 512,
            sample_rate_hz: 16000,
        };

        // Choose preferred plugin depending on feature set
        #[cfg(feature = "whisper")]
        let preferred_plugin = "whisper".to_string();
        #[cfg(not(feature = "whisper"))]
        let preferred_plugin = "mock".to_string();

        let opts = AppRuntimeOptions {
            activation_mode: ActivationMode::Vad,
            vad_config: Some(vad_config),
            stt_selection: Some(PluginSelectionConfig {
                preferred_plugin: Some(preferred_plugin),
                failover: Some(FailoverConfig::default()),
                gc_policy: Some(GcPolicy {
                    enabled: false,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            injection: Some(Default::default()),
            test_capture_to_dummy: true,
            test_device_config: Some(DeviceConfig {
                sample_rate: wav_loader.sample_rate(),
                channels: wav_loader.channels(),
            }),
            test_injection_sink: Some(mock_sink.clone()),
            transcription_config: Some(transcription_config),
            ..Default::default()
        };

        // 3. Start the application runtime.
        tracing::info!("Starting application runtime...");
        let app = start(opts).await.expect("Failed to start app runtime");
        let app = Arc::new(app);
        tracing::info!("Application runtime started successfully");

        // 4. Subscribe to VAD events directly and collect until SpeechEnd.
        let mut vad_rx = app.subscribe_vad();
        let vad_events = Arc::new(tokio::sync::Mutex::new(Vec::new()));

        // 5. Stream the WAV file into the pipeline in a background task.
        let audio_producer = app.audio_producer.clone();
        let stream_handle = tokio::spawn(async move {
            tracing::info!("Starting to stream WAV file...");
            match wav_loader
                .stream_to_ring_buffer_locked(audio_producer)
                .await
            {
                Ok(_) => tracing::info!("WAV streaming completed successfully"),
                Err(e) => tracing::error!("WAV streaming failed: {}", e),
            }
        });

        // 6. Collect VAD events until SpeechEnd (and injection if whisper enabled)
        let start_wait = std::time::Instant::now();

        // Wrap the entire wait loop in a 30-second timeout to prevent hangs
        let vad_collection_result = tokio::time::timeout(Duration::from_secs(30), async {
            let mut no_events_warning_printed = false;
            loop {
                tokio::select! {
                    evt = vad_rx.recv() => {
                        match evt {
                            Ok(e) => {
                                let ser = SerializableVadEvent::from(e);
                                tracing::info!("VAD event captured: {:?}", ser);
                                vad_events.lock().await.push(ser);
                                no_events_warning_printed = false; // Reset warning flag on new event
                            }
                            Err(e) => {
                                tracing::warn!("VAD channel closed or error: {}", e);
                                break;
                            }
                        }
                    }
                    _ = tokio::time::sleep(Duration::from_millis(250)) => {
                        // Periodic completion check
                    }
                }

                let events = vad_events.lock().await;
                let has_speech_end = events.iter().any(|e| e.kind == "SpeechEnd");
                #[cfg(feature = "whisper")]
                let has_injection = !mock_sink.injected_text.lock().unwrap().is_empty();
                #[cfg(not(feature = "whisper"))]
                let has_injection = true; // Ignore injection for mock-only runs
                drop(events);

                if has_speech_end && has_injection {
                    tracing::info!("Pipeline completion detected!");
                    break;
                }

                // Early warning if no events after 5 seconds
                if start_wait.elapsed() > Duration::from_secs(5) && !no_events_warning_printed {
                    let vad_lock = vad_events.lock().await;
                    tracing::warn!(
                        "No VAD events after 5 seconds. VAD events so far: {:?}",
                        *vad_lock
                    );
                    no_events_warning_printed = true;
                }

                // Fail fast with detailed output after 10 seconds
                if start_wait.elapsed() > Duration::from_secs(10) {
                    let vad_lock = vad_events.lock().await;
                    let injection_lock = mock_sink.injected_text.lock().unwrap();
                    panic!(
                        "Test timed out waiting for completion after 10 seconds.\nVAD events: {:?}\nInjections: {:?}",
                        *vad_lock, *injection_lock
                    );
                }
            }
        }).await;

        // Handle timeout case
        if vad_collection_result.is_err() {
            let vad_lock = vad_events.lock().await;
            let injection_lock = mock_sink.injected_text.lock().unwrap();
            panic!(
                "Test hung and timed out after 30 seconds waiting for completion.\nVAD events: {:?}\nInjections: {:?}",
                *vad_lock, *injection_lock
            );
        }

        // Wait a bit longer to ensure all events are processed
        tokio::time::sleep(Duration::from_millis(500)).await;

        // 7. Shut down the pipeline gracefully.
        tracing::info!("Shutting down pipeline...");
        app.shutdown().await;
        stream_handle.abort();
        tracing::info!("Pipeline shutdown complete");

        // 8. Collect the captured results.
        let final_vad_events = vad_events.lock().await.clone();
        let final_injected_text = mock_sink.injected_text.lock().unwrap().clone();

        tracing::info!("Final VAD events: {:?}", final_vad_events);
        tracing::info!("Final injected text: {:?}", final_injected_text);

        // 9. Assert against the golden masters.
        assert_golden(test_name, "vad", &final_vad_events);
        #[cfg(feature = "whisper")]
        assert_golden(test_name, "injection", &final_injected_text);
    }
}

#[cfg(test)]
pub mod test_utils {
    use async_trait::async_trait;
    use coldvox_text_injection::{InjectionContext, InjectionResult, TextInjector};
    use std::sync::{Arc, Mutex};

    /// A mock text injection sink that captures injected text for assertions.
    #[derive(Clone, Default)]
    pub struct MockInjectionSink {
        /// A shared, mutable vector to store the text that has been "injected".
        pub injected_text: Arc<Mutex<Vec<String>>>,
    }

    impl MockInjectionSink {
        pub fn new() -> Self {
            Self::default()
        }
    }

    #[async_trait]
    impl TextInjector for MockInjectionSink {
        async fn inject_text(
            &self,
            text: &str,
            _context: Option<&InjectionContext>,
        ) -> InjectionResult<()> {
            tracing::info!("Mock injection sink received text: {}", text);
            self.injected_text.lock().unwrap().push(text.to_string());
            Ok(())
        }

        async fn is_available(&self) -> bool {
            true
        }

        fn backend_name(&self) -> &'static str {
            "mock"
        }

        fn backend_info(&self) -> Vec<(&'static str, String)> {
            vec![]
        }
    }
}
