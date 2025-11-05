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
    use serde::Serialize;
    use serde::de::DeserializeOwned;
    use std::fs;
    use std::path::{Path, PathBuf};

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
        fs::write(&received_path, &received_json)
            .expect("Failed to write received artifact");

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
        let approved_json = fs::read_to_string(&approved_path)
            .expect("Failed to read approved artifact");

        // Deserialize and compare using similar-asserts for a nice diff.
        let approved_value: T = serde_json::from_str(&approved_json)
            .expect("Failed to deserialize approved value");

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

#[cfg(test)]
mod tests {
    use super::harness::assert_golden;
    use super::test_utils::MockInjectionSink;
    use coldvox_app::audio::wav_file_loader::WavFileLoader;
    use coldvox_app::runtime::{start, AppRuntimeOptions, ActivationMode};
    use coldvox_audio::DeviceConfig;
    use coldvox_stt::plugin::{PluginSelectionConfig, FailoverConfig, GcPolicy};
    use coldvox_vad::VadEvent;
    use serde::{Serialize, Deserialize};
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
                    duration_ms: Some(duration_ms),
                },
            }
        }
    }

    #[tokio::test]
    async fn test_short_phrase_pipeline() {
        // Enable detailed logging for debugging
        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .with_test_writer()
            .try_init();

        let test_name = "short_phrase";
        let wav_path = "test_data/test_11.wav";

        tracing::info!("Starting golden master test for: {}", test_name);

        // Enable accelerated playback to speed up the test
        std::env::set_var("COLDVOX_PLAYBACK_MODE", "accelerated");
        std::env::set_var("COLDVOX_PLAYBACK_SPEED_MULTIPLIER", "4.0");

        // 1. Set up the mock injection sink.
        let mock_sink = Arc::new(MockInjectionSink::new());

        // 2. Configure the runtime for a black-box test run.
        let mut wav_loader = WavFileLoader::new(wav_path).unwrap();
        tracing::info!(
            "Loaded WAV: {} Hz, {} channels",
            wav_loader.sample_rate(),
            wav_loader.channels()
        );
        let transcription_config = coldvox_stt::TranscriptionConfig {
            model_path: "tiny.en".to_string(),
            ..Default::default()
        };

        let opts = AppRuntimeOptions {
            activation_mode: ActivationMode::Vad,
            stt_selection: Some(PluginSelectionConfig {
                preferred_plugin: Some("whisper".to_string()),
                failover: Some(FailoverConfig::default()),
                gc_policy: Some(GcPolicy { enabled: false, ..Default::default() }),
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

        // 4. Subscribe to the VAD event channel to capture VAD output.
        let mut vad_rx = app.subscribe_vad();
        let vad_events = Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let vad_events_clone = vad_events.clone();

        let vad_collector_handle = tokio::spawn(async move {
            while let Ok(event) = vad_rx.recv().await {
                tracing::info!("VAD event collected: {:?}", event);
                let serializable_event = SerializableVadEvent::from(event);
                vad_events_clone.lock().await.push(serializable_event);
            }
        });

        // Give the pipeline a moment to fully initialize before streaming audio
        tokio::time::sleep(Duration::from_millis(200)).await;

        // 5. Stream the WAV file into the pipeline in a background task.
        tracing::info!("Starting WAV streaming...");
        let audio_producer = app.audio_producer.clone();
        let stream_handle = tokio::spawn(async move {
            tracing::info!("WAV streaming task started");
            let result = wav_loader
                .stream_to_ring_buffer_locked(audio_producer)
                .await;
            tracing::info!("WAV streaming completed: {:?}", result);
            result.unwrap();
        });

        // 6. Wait for the pipeline to signal completion.
        let vad_clone = vad_events.clone();
        let injection_clone = mock_sink.injected_text.clone();
        let wait_result = tokio::time::timeout(Duration::from_secs(60), async move {
            let mut iteration = 0;
            loop {
                iteration += 1;
                let vad_lock = vad_clone.lock().await;
                let vad_event_count = vad_lock.len();
                let has_speech_start = vad_lock.iter().any(|e| e.kind == "SpeechStart");
                let has_speech_end = vad_lock.iter().any(|e| e.kind == "SpeechEnd");
                let vad_summary: Vec<String> = vad_lock.iter().map(|e| e.kind.clone()).collect();
                drop(vad_lock);

                let injection_lock = injection_clone.lock().unwrap();
                let injection_count = injection_lock.len();
                let has_injection = !injection_lock.is_empty();
                let injection_preview = if !injection_lock.is_empty() {
                    format!("{:?}", &injection_lock[0])
                } else {
                    "None".to_string()
                };
                drop(injection_lock);

                if iteration % 4 == 0 || has_speech_start || has_injection {
                    tracing::info!(
                        "Iter {}: VAD events: {} {:?}, has_start={}, has_end={}, Injections: {}, preview: {}",
                        iteration,
                        vad_event_count,
                        vad_summary,
                        has_speech_start,
                        has_speech_end,
                        injection_count,
                        injection_preview
                    );
                }

                if has_speech_end && has_injection {
                    tracing::info!("Pipeline completion detected!");
                    break;
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        })
        .await;

        // Wait for the streaming task to complete
        let _ = stream_handle.await;

        if let Err(_) = wait_result {
            let vad_lock = vad_events.lock().await;
            let injection_lock = mock_sink.injected_text.lock().unwrap();
            panic!(
                "Test timed out! VAD events: {:?}, Injections: {:?}",
                vad_lock.iter().map(|e| &e.kind).collect::<Vec<_>>(),
                *injection_lock
            );
        }

        // 7. Shut down the pipeline gracefully.
        app.shutdown().await;
        vad_collector_handle.abort();

        // 8. Collect the captured results.
        let final_vad_events = vad_events.lock().await.clone();
        let final_injected_text = mock_sink.injected_text.lock().unwrap().clone();

        // 9. Assert against the golden masters.
        assert_golden(test_name, "vad", &final_vad_events);
        assert_golden(test_name, "injection", &final_injected_text);
    }
}

#[cfg(test)]
pub mod test_utils {
    use coldvox_text_injection::{InjectionContext, InjectionResult, TextInjector};
    use std::sync::{Arc, Mutex};
    use async_trait::async_trait;

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
            if !text.trim().is_empty() {
                let mut guard = self.injected_text.lock().unwrap();
                guard.push(text.to_string());
            }
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
