use std::sync::Once;

static TRACING_INIT: Once = Once::new();

/// Sets up the test environment by initializing tracing and setting the VOSK_MODEL_PATH.
pub fn setup_test_env() {
    // Ensure tracing is initialized for logging purposes in tests, but only once.
    TRACING_INIT.call_once(|| {
        tracing_subscriber::fmt::try_init().ok();
    });

    // Set VOSK_MODEL_PATH for the test, canonicalizing the path.
    let model_path = std::path::Path::new("../../models/vosk-model-small-en-us-0.15")
        .canonicalize()
        .unwrap_or_else(|_| std::path::PathBuf::from("../../models/vosk-model-small-en-us-0.15"))
        .to_string_lossy()
        .to_string();
    std::env::set_var("VOSK_MODEL_PATH", &model_path);
}
