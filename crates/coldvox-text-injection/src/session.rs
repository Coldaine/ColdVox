use crate::types::InjectionMetrics;
use std::time::{Duration, Instant};
use tracing::{debug, warn};

/// Session state machine for buffered text injection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SessionState {
    /// No active session, waiting for first transcription
    #[default]
    Idle,
    /// Actively receiving transcriptions, buffering them
    Buffering,
    /// No new transcriptions received, waiting for silence timeout
    WaitingForSilence,
    /// Silence timeout reached, ready to inject buffered text
    ReadyToInject,
}

impl std::fmt::Display for SessionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionState::Idle => write!(f, "IDLE"),
            SessionState::Buffering => write!(f, "BUFFERING"),
            SessionState::WaitingForSilence => write!(f, "WAITING_FOR_SILENCE"),
            SessionState::ReadyToInject => write!(f, "READY_TO_INJECT"),
        }
    }
}

/// Configuration for session management
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Silence timeout before triggering injection (default: 1500ms)
    pub silence_timeout_ms: u64,
    /// Maximum buffer size in characters (default: 5000)
    pub max_buffer_size: usize,
    /// Separator to join buffered transcriptions (default: " ")
    pub join_separator: String,
    /// Time to wait before transitioning from Buffering to WaitingForSilence (default: 500ms)
    pub buffer_pause_timeout_ms: u64,
    /// Whether to flush on punctuation marks
    pub flush_on_punctuation: bool,
    /// Punctuation marks that trigger flushing
    pub punctuation_marks: Vec<char>,
    /// Whether to normalize whitespace
    pub normalize_whitespace: bool,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            silence_timeout_ms: 0, // Immediate injection after STT completes
            max_buffer_size: 5000,
            join_separator: " ".to_string(),
            buffer_pause_timeout_ms: 0, // No pause needed since STT buffers audio
            flush_on_punctuation: true,
            punctuation_marks: vec!['.', '!', '?', ';'],
            normalize_whitespace: true,
        }
    }
}

/// Manages a single dictation session with buffering and silence detection
#[derive(Debug)]
pub struct InjectionSession {
    /// Current state in the session state machine
    state: SessionState,
    /// Buffered transcriptions waiting to be injected
    buffer: Vec<String>,
    /// Timestamp of the last received transcription
    last_transcription: Option<Instant>,
    /// Timestamp when we transitioned to Buffering state
    buffering_start: Option<Instant>,
    /// Configurable silence timeout duration
    silence_timeout: Duration,
    /// Time to wait before transitioning from Buffering to WaitingForSilence
    buffer_pause_timeout: Duration,
    /// Maximum buffer size in characters
    max_buffer_size: usize,
    /// Separator for joining buffered text
    join_separator: String,
    /// Whether to flush on punctuation marks
    flush_on_punctuation: bool,
    /// Punctuation marks that trigger flushing
    punctuation_marks: Vec<char>,
    /// Whether to normalize whitespace
    normalize_whitespace: bool,
    /// Reference to injection metrics for telemetry
    metrics: std::sync::Arc<std::sync::Mutex<InjectionMetrics>>,
    /// Throttled timestamp for diagnostic logging to avoid log spam
    last_diagnostic_log: Option<Instant>,
}

impl InjectionSession {
    /// Create a new session with the given configuration
    pub fn new(
        config: SessionConfig,
        metrics: std::sync::Arc<std::sync::Mutex<InjectionMetrics>>,
    ) -> Self {
        Self {
            state: SessionState::Idle,
            buffer: Vec::new(),
            last_transcription: None,
            buffering_start: None,
            silence_timeout: Duration::from_millis(config.silence_timeout_ms),
            buffer_pause_timeout: Duration::from_millis(config.buffer_pause_timeout_ms),
            max_buffer_size: config.max_buffer_size,
            join_separator: config.join_separator,
            flush_on_punctuation: config.flush_on_punctuation,
            punctuation_marks: config.punctuation_marks,
            normalize_whitespace: config.normalize_whitespace,
            metrics,
            last_diagnostic_log: None,
        }
    }

    /// Add a new transcription to the session buffer
    pub fn add_transcription(&mut self, text: String) {
        // Filter out empty or whitespace-only transcriptions
        let text = text.trim();
        if text.is_empty() {
            return;
        }

        let text = if self.normalize_whitespace {
            // Normalize whitespace (collapse multiple spaces, remove leading/trailing)
            text.split_whitespace().collect::<Vec<&str>>().join(" ")
        } else {
            text.to_string()
        };

        // Reset diagnostic throttling on new input
        self.last_diagnostic_log = None;

        // Record the number of characters being buffered
        self.record_buffered_chars(text.len() as u64);

        // Check if text ends with punctuation that should trigger flushing
        let ends_with_punctuation = self.flush_on_punctuation
            && !text.is_empty()
            && self
                .punctuation_marks
                .contains(&text.chars().last().unwrap());

        // Add to buffer
        self.buffer.push(text);
        self.last_transcription = Some(Instant::now());

        // Update state based on current state
        match self.state {
            SessionState::Idle => {
                self.state = SessionState::Buffering;
                self.buffering_start = Some(Instant::now());
                debug!("Session started - first transcription buffered");
            }
            SessionState::Buffering => {
                debug!(
                    "Additional transcription buffered, {} items in session",
                    self.buffer.len()
                );
            }
            SessionState::WaitingForSilence => {
                // New transcription resets the silence timer and transitions back to Buffering
                self.state = SessionState::Buffering;
                self.buffering_start = Some(Instant::now());
                debug!("Silence timer reset by new transcription");
            }
            SessionState::ReadyToInject => {
                // This shouldn't happen in normal flow, but handle gracefully
                warn!("Received transcription while ready to inject - resetting session");
                self.state = SessionState::Buffering;
                self.buffering_start = Some(Instant::now());
            }
        }

        // Check if buffer is too large and force injection
        if self.total_chars() > self.max_buffer_size {
            self.state = SessionState::ReadyToInject;
            warn!("Buffer size limit reached, forcing injection");
            return;
        }

        // Check if we should flush due to punctuation
        if ends_with_punctuation {
            self.state = SessionState::ReadyToInject;
            debug!("Flushing buffer due to punctuation mark");
        }
    }

    /// Check if the session should transition to WaitingForSilence state
    /// This should be called periodically to detect when transcription has paused
    pub fn check_for_silence_transition(&mut self) {
        if self.state == SessionState::Buffering {
            if let Some(_buffering_start) = self.buffering_start {
                let time_since_last_transcription = self.last_transcription.map(|t| t.elapsed());

                // If we haven't received a transcription for buffer_pause_timeout,
                // transition to WaitingForSilence
                if let Some(time_since_last) = time_since_last_transcription {
                    if time_since_last >= self.buffer_pause_timeout {
                        self.state = SessionState::WaitingForSilence;
                        debug!("Transitioned to WaitingForSilence state");
                    }
                }
            }
        }
    }

    /// Determine if we should emit a diagnostic log message (throttled to avoid noise)
    fn should_log_diagnostic(&mut self) -> bool {
        const DIAGNOSTIC_INTERVAL: Duration = Duration::from_millis(500);
        let now = Instant::now();
        if let Some(last) = self.last_diagnostic_log {
            if now.duration_since(last) < DIAGNOSTIC_INTERVAL {
                return false;
            }
        }
        self.last_diagnostic_log = Some(now);
        true
    }

    /// Check if the session should inject based on silence timeout
    pub fn should_inject(&mut self) -> bool {
        match self.state {
            SessionState::Buffering => {
                // Check if we should transition to WaitingForSilence first
                self.check_for_silence_transition();
                if self.should_log_diagnostic() {
                    let since_last = self
                        .time_since_last_transcription()
                        .map(|d| d.as_millis())
                        .unwrap_or(0);
                    debug!(
                        state = "BUFFERING",
                        buffer_items = self.buffer.len(),
                        buffer_chars = self.total_chars(),
                        time_since_last_ms = since_last,
                        pause_timeout_ms = self.buffer_pause_timeout.as_millis(),
                        "Session buffering - awaiting pause before injection"
                    );
                }
                false // Don't inject while still in Buffering state
            }
            SessionState::WaitingForSilence => {
                if let Some(last_time) = self.last_transcription {
                    if last_time.elapsed() >= self.silence_timeout {
                        // Silence timeout reached, transition to ready to inject
                        self.state = SessionState::ReadyToInject;
                        self.last_diagnostic_log = None;
                        debug!(
                            "Silence timeout reached, ready to inject {} transcriptions",
                            self.buffer.len()
                        );
                        true
                    } else {
                        if self.should_log_diagnostic() {
                            let elapsed = last_time.elapsed();
                            let remaining = self.silence_timeout.saturating_sub(elapsed);
                            debug!(
                                state = "WAITING_FOR_SILENCE",
                                buffer_items = self.buffer.len(),
                                buffer_chars = self.total_chars(),
                                elapsed_ms = elapsed.as_millis(),
                                remaining_ms = remaining.as_millis(),
                                silence_timeout_ms = self.silence_timeout.as_millis(),
                                "Waiting for silence before injection"
                            );
                        }
                        false
                    }
                } else {
                    false
                }
            }
            SessionState::ReadyToInject => {
                // Check if buffer is empty (could happen if cleared)
                if self.buffer.is_empty() {
                    self.state = SessionState::Idle;
                    self.last_diagnostic_log = None;
                    false
                } else {
                    self.last_diagnostic_log = None;
                    true
                }
            }
            SessionState::Idle => false,
        }
    }

    /// Take the buffered text and reset the session to idle
    pub fn take_buffer(&mut self) -> String {
        let text = self.buffer.join(&self.join_separator);
        let size = text.len();
        self.buffer.clear();
        self.last_transcription = None;
        self.buffering_start = None;
        self.state = SessionState::Idle;
        self.last_diagnostic_log = None;
        debug!("Session buffer cleared, {} chars taken", text.len());

        // Record the flush event with the size
        self.record_flush(size as u64);
        text
    }

    /// Get current session state
    pub fn state(&self) -> SessionState {
        self.state
    }

    /// Get number of buffered transcriptions
    pub fn buffer_len(&self) -> usize {
        self.buffer.len()
    }

    /// Get total character count in buffer
    pub fn total_chars(&self) -> usize {
        self.buffer.iter().map(|s| s.len()).sum::<usize>()
            + (self.buffer.len().saturating_sub(1) * self.join_separator.len())
    }

    /// Get time since last transcription (None if no transcriptions)
    pub fn time_since_last_transcription(&self) -> Option<Duration> {
        self.last_transcription.map(|t| t.elapsed())
    }

    /// Check if session has any buffered content
    pub fn has_content(&self) -> bool {
        !self.buffer.is_empty()
    }

    /// Force the session into ready-to-inject state (for manual triggers)
    pub fn force_inject(&mut self) {
        if self.has_content() {
            self.state = SessionState::ReadyToInject;
            self.last_diagnostic_log = None;
            debug!("Session forced to inject state");
        }
    }

    /// Clear the session buffer and reset to idle (for cancellation)
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.last_transcription = None;
        self.buffering_start = None;
        self.state = SessionState::Idle;
        self.last_diagnostic_log = None;
        debug!("Session cleared and reset to idle");
    }

    /// Get buffer preview without taking the buffer (for debugging/UI)
    pub fn buffer_preview(&self) -> String {
        self.buffer.join(&self.join_separator)
    }

    /// Record characters that have been buffered
    pub fn record_buffered_chars(&self, count: u64) {
        if let Ok(mut metrics) = self.metrics.lock() {
            metrics.record_buffered_chars(count);
        }
    }

    /// Record a flush event
    pub fn record_flush(&self, size: u64) {
        if let Ok(mut metrics) = self.metrics.lock() {
            metrics.record_flush(size);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_session_state_transitions() {
        let config = SessionConfig {
            silence_timeout_ms: 100,     // Short timeout for testing
            buffer_pause_timeout_ms: 50, // Short pause timeout for testing
            ..Default::default()
        };
        let metrics = std::sync::Arc::new(std::sync::Mutex::new(InjectionMetrics::default()));
        let mut session = InjectionSession::new(config, metrics);

        // Start with idle state
        assert_eq!(session.state(), SessionState::Idle);
        assert!(!session.has_content());

        // Add first transcription
        session.add_transcription("Hello".to_string());
        assert_eq!(session.state(), SessionState::Buffering);
        assert!(session.has_content());
        assert_eq!(session.buffer_len(), 1);

        // Add second transcription
        session.add_transcription("world".to_string());
        assert_eq!(session.state(), SessionState::Buffering);
        assert_eq!(session.buffer_len(), 2);

        // Wait for buffer pause timeout (should transition to WaitingForSilence)
        thread::sleep(Duration::from_millis(75));
        session.check_for_silence_transition();
        assert_eq!(session.state(), SessionState::WaitingForSilence);

        // Wait for silence timeout (should transition to ReadyToInject)
        thread::sleep(Duration::from_millis(75));
        assert!(session.should_inject());
        assert_eq!(session.state(), SessionState::ReadyToInject);

        // Take buffer
        let text = session.take_buffer();
        assert_eq!(text, "Hello world");
        assert_eq!(session.state(), SessionState::Idle);
        assert!(!session.has_content());
    }

    #[test]
    fn test_buffer_size_limit() {
        let config = SessionConfig {
            max_buffer_size: 10, // Very small limit
            ..Default::default()
        };
        let metrics = std::sync::Arc::new(std::sync::Mutex::new(InjectionMetrics::default()));
        let mut session = InjectionSession::new(config, metrics);

        // Add text that exceeds limit
        session.add_transcription("This is a long sentence".to_string());
        assert_eq!(session.state(), SessionState::ReadyToInject);
    }

    #[test]
    fn test_empty_transcription_filtering() {
        let metrics = std::sync::Arc::new(std::sync::Mutex::new(InjectionMetrics::default()));
        let mut session = InjectionSession::new(SessionConfig::default(), metrics);

        session.add_transcription("".to_string());
        session.add_transcription("   ".to_string());
        session.add_transcription("Hello".to_string());

        assert_eq!(session.buffer_len(), 1);
        assert_eq!(session.take_buffer(), "Hello");
    }

    #[test]
    fn test_silence_detection() {
        let config = SessionConfig {
            silence_timeout_ms: 200,
            buffer_pause_timeout_ms: 50,
            ..Default::default()
        };
        let metrics = std::sync::Arc::new(std::sync::Mutex::new(InjectionMetrics::default()));
        let mut session = InjectionSession::new(config, metrics);

        // Add transcription
        session.add_transcription("Test".to_string());
        assert_eq!(session.state(), SessionState::Buffering);

        // Wait for buffer pause timeout
        thread::sleep(Duration::from_millis(75));
        session.check_for_silence_transition();
        assert_eq!(session.state(), SessionState::WaitingForSilence);

        // Add new transcription - should go back to Buffering
        session.add_transcription("Another".to_string());
        assert_eq!(session.state(), SessionState::Buffering);

        // Wait for buffer pause timeout again
        thread::sleep(Duration::from_millis(75));
        session.check_for_silence_transition();
        assert_eq!(session.state(), SessionState::WaitingForSilence);
    }
}
