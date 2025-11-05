use std::sync::Once;
use tracing_subscriber::{fmt, EnvFilter};

/// Initialize tracing for tests with debug level
fn init_test_tracing() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug"));

        fmt().with_env_filter(filter).with_test_writer().init();
    });
}

/// Initialize test infrastructure (tracing, sleep observer)
pub fn init_test_infrastructure() {
    init_test_tracing();
    crate::sleep_instrumentation::init_sleep_observer();
}

/// Word Error Rate (WER) calculation utilities for STT testing.
pub mod wer {
    /// Centralized implementation to avoid code duplication between test files.
    pub fn calculate_wer(reference: &str, hypothesis: &str) -> f64 {
        let ref_words: Vec<&str> = reference.split_whitespace().collect();
        let hyp_words: Vec<&str> = hypothesis.split_whitespace().collect();

        if ref_words.is_empty() {
            return if hyp_words.is_empty() { 0.0 } else { 1.0 };
        }

        let ref_len = ref_words.len();
        let hyp_len = hyp_words.len();

        // Dynamic programming table for Levenshtein distance
        let mut dp = vec![vec![0; hyp_len + 1]; ref_len + 1];

        // Initialize base cases
        for (i, row) in dp.iter_mut().enumerate().take(ref_len + 1) {
            row[0] = i; // deletions
        }

        for j in 0..=hyp_len {
            dp[0][j] = j; // insertions
        }

        // Fill the DP table
        for i in 1..=ref_len {
            for j in 1..=hyp_len {
                let substitution_cost = if ref_words[i - 1] == hyp_words[j - 1] {
                    0
                } else {
                    1
                };

                dp[i][j] = std::cmp::min(
                    std::cmp::min(
                        dp[i - 1][j] + 1, // deletion
                        dp[i][j - 1] + 1, // insertion
                    ),
                    dp[i - 1][j - 1] + substitution_cost, // substitution (or match)
                );
            }
        }

        dp[ref_len][hyp_len] as f64 / ref_len as f64
    }

    /// Format WER as a percentage string.
    pub fn format_wer_percentage(wer: f64) -> String {
        format!("{:.1}%", wer * 100.0)
    }
}

#[cfg(feature = "headless-test")]
pub mod mock_injection {
    use async_trait::async_trait;
    use coldvox_text_injection::{InjectionContext, InjectionResult, TextInjector};
    use tokio::sync::mpsc;

    /// A mock text injection sink for testing purposes.
    ///
    /// This injector does not perform any real text injection. Instead, it sends the
    /// text it receives to a `tokio::sync::mpsc::Sender`, allowing test code to
    /// capture and assert on the text that would have been injected.
    #[derive(Debug)]
    pub struct MockInjectionSink {
        sender: mpsc::Sender<String>,
    }

    impl MockInjectionSink {
        /// Creates a new `MockInjectionSink` that will send injected text to the
        /// provided `mpsc::Sender`.
        pub fn new(sender: mpsc::Sender<String>) -> Self {
            Self { sender }
        }
    }

    #[async_trait]
    impl TextInjector for MockInjectionSink {
        /// "Injects" text by sending it to the internal `mpsc::Sender`.
        /// This method will return an error if the receiver has been dropped.
        async fn inject_text(
            &self,
            text: &str,
            _context: Option<&InjectionContext>,
        ) -> InjectionResult<()> {
            self.sender
                .send(text.to_string())
                .await
                .map_err(|e| coldvox_foundation::error::InjectionError::Other(e.to_string()))?;
            Ok(())
        }

        /// This mock injector is always available.
        async fn is_available(&self) -> bool {
            true
        }

        /// Returns the name of this mock injector.
        fn backend_name(&self) -> &'static str {
            "mock_injection_sink"
        }

        /// Returns an empty vector as this mock injector has no specific configuration.
        fn backend_info(&self) -> Vec<(&'static str, String)> {
            vec![]
        }
    }
}
