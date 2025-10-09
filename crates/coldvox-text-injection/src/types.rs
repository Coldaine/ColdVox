use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Behavior when all injection methods fail. Used for debugging/CI to cause
/// immediate termination or panic when injection cannot succeed.
fn default_fail_fast() -> bool {
    false
}

/// Injection mode override (paste vs keystroke decision)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InjectionMode {
    /// Use paste-based injection
    Paste,
    /// Use keystroke-based injection
    Keystroke,
}

/// Unified injection context passed to all injectors
/// Contains pre-warmed data, focus info, and mode overrides
#[derive(Debug, Clone, Default)]
pub struct InjectionContext {
    /// Target application identifier
    pub target_app: Option<String>,
    /// Window identifier
    pub window_id: Option<String>,
    /// Pre-warmed AT-SPI focused node path (stored as path string for portability)
    pub atspi_focused_node_path: Option<String>,
    /// Pre-warmed clipboard backup data
    pub clipboard_backup: Option<String>,
    /// Injection mode override (paste vs keystroke)
    /// When Some, this overrides the config-based decision
    pub mode_override: Option<InjectionMode>,
}

/// Enumeration of all available text injection methods
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InjectionMethod {
    /// Insert text directly using AT-SPI2 EditableText interface
    AtspiInsert,
    /// Set clipboard then trigger paste; requires paste success.
    /// Implementation tries AT-SPI paste first, then ydotool fallback.
    ClipboardPasteFallback,
    /// Use kdotool for window activation/focus assistance (opt-in)
    KdoToolAssist,
    /// Use enigo library for synthetic text/paste (opt-in)
    EnigoText,

    /// No-op fallback injector (always succeeds, does nothing)
    NoOp,
}

/// Configuration for text injection system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InjectionConfig {
    /// Whether to allow kdotool usage (external CLI for KDE window activation)
    #[serde(default = "default_false")]
    pub allow_kdotool: bool,
    /// Whether to allow enigo library usage (Wayland/libei paths)
    #[serde(default = "default_false")]
    pub allow_enigo: bool,

    /// Whether to restore the clipboard content after injection
    // Clipboard restoration is unconditional now; removal of runtime toggle.
    /// Whether to allow injection when focus state is unknown
    #[serde(default = "default_inject_on_unknown_focus")]
    pub inject_on_unknown_focus: bool,

    /// Whether to require editable focus for injection
    #[serde(default = "default_require_focus")]
    pub require_focus: bool,

    /// Hotkey to pause/resume injection (e.g., "Ctrl+Alt+P")
    #[serde(default = "default_pause_hotkey")]
    pub pause_hotkey: Option<String>,

    /// Whether to redact text content in logs
    #[serde(default = "default_redact_logs")]
    pub redact_logs: bool,

    /// Overall latency budget for a single injection call, across all fallbacks.
    #[serde(default = "default_max_total_latency_ms")]
    pub max_total_latency_ms: u64,

    /// Timeout for individual injection method attempts (e.g., AT-SPI call, clipboard set).
    #[serde(default = "default_per_method_timeout_ms")]
    pub per_method_timeout_ms: u64,
    /// Timeout specifically for a paste action (e.g., waiting for AT-SPI paste to complete).
    #[serde(default = "default_paste_action_timeout_ms")]
    pub paste_action_timeout_ms: u64,

    /// Initial cooldown period after a method fails for a specific application.
    #[serde(default = "default_cooldown_initial_ms")]
    pub cooldown_initial_ms: u64,
    /// Backoff factor to apply to the cooldown after consecutive failures.
    #[serde(default = "default_cooldown_backoff_factor")]
    pub cooldown_backoff_factor: f32,
    /// Maximum cooldown period to prevent excessively long waits.
    #[serde(default = "default_cooldown_max_ms")]
    pub cooldown_max_ms: u64,

    /// Mode for text injection: "keystroke", "paste", or "auto"
    #[serde(default = "default_injection_mode")]
    pub injection_mode: String,
    /// Keystroke rate in characters per second (cps)
    #[serde(default = "default_keystroke_rate_cps")]
    pub keystroke_rate_cps: u32,
    /// Maximum number of characters to send in a single burst
    #[serde(default = "default_max_burst_chars")]
    pub max_burst_chars: u32,
    /// Number of characters to chunk paste operations into
    #[serde(default = "default_paste_chunk_chars")]
    pub paste_chunk_chars: u32,
    /// Delay between paste chunks in milliseconds
    #[serde(default = "default_chunk_delay_ms")]
    pub chunk_delay_ms: u64,

    /// Cache duration for focus status (ms)
    #[serde(default = "default_focus_cache_duration_ms")]
    pub focus_cache_duration_ms: u64,

    /// Minimum success rate before trying fallback methods
    #[serde(default = "default_min_success_rate")]
    pub min_success_rate: f64,

    /// Number of samples before trusting success rate
    #[serde(default = "default_min_sample_size")]
    pub min_sample_size: u32,

    /// Enable window manager integration
    #[serde(default = "default_true")]
    pub enable_window_detection: bool,

    /// Delay before restoring clipboard (ms)
    #[serde(default = "default_clipboard_restore_delay_ms")]
    pub clipboard_restore_delay_ms: Option<u64>,

    /// Timeout for window discovery operations (ms)
    #[serde(default = "default_discovery_timeout_ms")]
    pub discovery_timeout_ms: u64,

    /// Allowlist of application patterns (regex) for injection
    #[serde(default)]
    pub allowlist: Vec<String>,

    /// Blocklist of application patterns (regex) to block injection
    #[serde(default)]
    pub blocklist: Vec<String>,

    /// If true, exit the process immediately if all injection methods fail.
    #[serde(default = "default_fail_fast")]
    pub fail_fast: bool,
}

fn default_false() -> bool {
    false
}

fn default_inject_on_unknown_focus() -> bool {
    true // Default to true to avoid blocking on Wayland without AT-SPI
}

fn default_require_focus() -> bool {
    false
}

fn default_pause_hotkey() -> Option<String> {
    None
}

fn default_redact_logs() -> bool {
    true // Privacy-first by default
}

fn default_allowlist() -> Vec<String> {
    vec![]
}

fn default_blocklist() -> Vec<String> {
    vec![]
}

fn default_injection_mode() -> String {
    "auto".to_string()
}

fn default_keystroke_rate_cps() -> u32 {
    20 // 20 characters per second (human typing speed)
}

fn default_max_burst_chars() -> u32 {
    50 // Maximum 50 characters in a single burst
}

fn default_paste_chunk_chars() -> u32 {
    500 // Chunk paste operations into 500 character chunks
}

fn default_chunk_delay_ms() -> u64 {
    30
}

fn default_focus_cache_duration_ms() -> u64 {
    200 // Cache focus status for 200ms
}

fn default_min_success_rate() -> f64 {
    0.3 // 30% minimum success rate before considering fallback
}

fn default_min_sample_size() -> u32 {
    5 // Need at least 5 samples before trusting success rate
}

fn default_true() -> bool {
    true
}

fn default_clipboard_restore_delay_ms() -> Option<u64> {
    Some(500) // Wait 500ms before restoring clipboard
}

fn default_max_total_latency_ms() -> u64 {
    800
}

fn default_per_method_timeout_ms() -> u64 {
    250
}

fn default_paste_action_timeout_ms() -> u64 {
    200
}

fn default_cooldown_initial_ms() -> u64 {
    10000 // 10 seconds
}

fn default_cooldown_backoff_factor() -> f32 {
    2.0
}

fn default_cooldown_max_ms() -> u64 {
    300_000 // 5 minutes
}

fn default_discovery_timeout_ms() -> u64 {
    1000 // 1 second
}

impl Default for InjectionConfig {
    fn default() -> Self {
        Self {
            allow_kdotool: default_false(),
            allow_enigo: default_false(),

            // restore_clipboard removed - restoration is always performed by clipboard injectors
            inject_on_unknown_focus: default_inject_on_unknown_focus(),
            require_focus: default_require_focus(),
            pause_hotkey: default_pause_hotkey(),
            redact_logs: default_redact_logs(),
            max_total_latency_ms: default_max_total_latency_ms(),
            per_method_timeout_ms: default_per_method_timeout_ms(),
            paste_action_timeout_ms: default_paste_action_timeout_ms(),
            cooldown_initial_ms: default_cooldown_initial_ms(),
            cooldown_backoff_factor: default_cooldown_backoff_factor(),
            cooldown_max_ms: default_cooldown_max_ms(),
            injection_mode: default_injection_mode(),
            keystroke_rate_cps: default_keystroke_rate_cps(),
            max_burst_chars: default_max_burst_chars(),
            paste_chunk_chars: default_paste_chunk_chars(),
            chunk_delay_ms: default_chunk_delay_ms(),
            focus_cache_duration_ms: default_focus_cache_duration_ms(),
            min_success_rate: default_min_success_rate(),
            min_sample_size: default_min_sample_size(),
            enable_window_detection: default_true(),
            clipboard_restore_delay_ms: default_clipboard_restore_delay_ms(),
            discovery_timeout_ms: default_discovery_timeout_ms(),
            allowlist: default_allowlist(),
            blocklist: default_blocklist(),
            fail_fast: default_fail_fast(),
        }
    }
}

impl InjectionConfig {
    pub fn max_total_latency(&self) -> Duration {
        Duration::from_millis(self.max_total_latency_ms)
    }

    pub fn per_method_timeout(&self) -> Duration {
        Duration::from_millis(self.per_method_timeout_ms)
    }

    pub fn paste_action_timeout(&self) -> Duration {
        Duration::from_millis(self.paste_action_timeout_ms)
    }
}

/// Result type for injection operations
pub type InjectionResult<T> = Result<T, InjectionError>;

/// Errors that can occur during text injection
#[derive(Debug, thiserror::Error)]
pub enum InjectionError {
    #[error("No editable focus found")]
    NoEditableFocus,

    #[error("Method not available: {0}")]
    MethodNotAvailable(String),

    #[error("Timeout after {0}ms")]
    Timeout(u64),

    #[error("All methods failed: {0}")]
    AllMethodsFailed(String),

    #[error("Method unavailable: {0}")]
    MethodUnavailable(String),

    #[error("Method failed: {0}")]
    MethodFailed(String),

    #[error("Budget exhausted")]
    BudgetExhausted,

    #[error("Clipboard error: {0}")]
    Clipboard(String),

    #[error("Process error: {0}")]
    Process(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Other error: {0}")]
    Other(String),
}

/// Metrics and telemetry data for injection attempts
#[derive(Debug, Default, Clone)]
pub struct InjectionMetrics {
    /// Total number of injection attempts
    pub attempts: u64,
    /// Number of successful injections
    pub successes: u64,
    /// Number of failed injections
    pub failures: u64,
    /// Total time spent in injection attempts
    pub total_duration_ms: u64,
    /// Average duration of injection attempts
    pub avg_duration_ms: f64,
    /// Method-specific metrics
    pub method_metrics: std::collections::HashMap<InjectionMethod, MethodMetrics>,
    /// Number of characters buffered
    pub chars_buffered: u64,
    /// Number of characters injected
    pub chars_injected: u64,
    /// Number of flushes
    pub flushes: u64,
    /// Number of paste operations
    pub paste_uses: u64,
    /// Number of keystroke operations
    pub keystroke_uses: u64,
    /// Number of backend denials
    pub backend_denied: u64,
    /// Number of focus missing errors
    pub focus_missing: u64,
    /// Number of rate limited events
    pub rate_limited: u64,
    /// Histogram of latency from final transcription to injection
    pub latency_from_final_ms: Vec<u64>,
    /// Histogram of flush sizes
    pub flush_size_chars: Vec<u64>,
    /// Timestamp of last injection
    pub last_injection: Option<std::time::Instant>,
    /// Age of stuck buffer (if any)
    pub stuck_buffer_age_ms: u64,
    /// Count of compiled allowlist regex patterns (feature `regex`)
    pub allowlist_regex_count: u64,
    /// Count of compiled blocklist regex patterns (feature `regex`)
    pub blocklist_regex_count: u64,
}

/// Metrics for a specific injection method
#[derive(Debug, Default, Clone)]
pub struct MethodMetrics {
    /// Number of attempts using this method
    pub attempts: u64,
    /// Number of successful attempts
    pub successes: u64,
    /// Number of failures
    pub failures: u64,
    /// Total duration of attempts
    pub total_duration_ms: u64,
    /// Last success timestamp
    pub last_success: Option<std::time::Instant>,
    /// Last failure timestamp and error message
    pub last_failure: Option<(std::time::Instant, String)>,
}

impl InjectionMetrics {
    /// Record a new injection attempt
    pub fn record_attempt(&mut self, method: InjectionMethod, duration_ms: u64) {
        self.attempts += 1;
        self.total_duration_ms += duration_ms;

        // Update method-specific metrics
        let method_metrics = self.method_metrics.entry(method).or_default();
        method_metrics.attempts += 1;
        method_metrics.total_duration_ms += duration_ms;
    }

    /// Record characters that have been buffered
    pub fn record_buffered_chars(&mut self, count: u64) {
        self.chars_buffered += count;
    }

    /// Record characters that have been successfully injected
    pub fn record_injected_chars(&mut self, count: u64) {
        self.chars_injected += count;
    }

    /// Record a flush event
    pub fn record_flush(&mut self, size: u64) {
        self.flushes += 1;
        self.flush_size_chars.push(size);
    }

    /// Record a paste operation
    pub fn record_paste(&mut self) {
        self.paste_uses += 1;
    }

    /// Record a keystroke operation
    pub fn record_keystroke(&mut self) {
        self.keystroke_uses += 1;
    }

    /// Record a backend denial
    pub fn record_backend_denied(&mut self) {
        self.backend_denied += 1;
    }

    /// Record a focus missing error
    pub fn record_focus_missing(&mut self) {
        self.focus_missing += 1;
    }

    /// Record a rate limited event
    pub fn record_rate_limited(&mut self) {
        self.rate_limited += 1;
    }

    /// Record latency from final transcription to injection
    pub fn record_latency_from_final(&mut self, latency_ms: u64) {
        self.latency_from_final_ms.push(latency_ms);
    }

    /// Update the last injection timestamp
    pub fn update_last_injection(&mut self) {
        self.last_injection = Some(std::time::Instant::now());
    }

    /// Update the stuck buffer age
    pub fn update_stuck_buffer_age(&mut self, age_ms: u64) {
        self.stuck_buffer_age_ms = age_ms;
    }

    /// Record a successful injection
    pub fn record_success(&mut self, method: InjectionMethod, duration_ms: u64) {
        self.successes += 1;
        self.record_attempt(method, duration_ms);

        // Update method-specific success
        if let Some(metrics) = self.method_metrics.get_mut(&method) {
            metrics.successes += 1;
            metrics.last_success = Some(std::time::Instant::now());
        }
    }

    /// Record a failed injection
    pub fn record_failure(&mut self, method: InjectionMethod, duration_ms: u64, error: String) {
        self.failures += 1;
        self.record_attempt(method, duration_ms);

        // Update method-specific failure
        if let Some(metrics) = self.method_metrics.get_mut(&method) {
            metrics.failures += 1;
            metrics.last_failure = Some((std::time::Instant::now(), error));
        }
    }

    /// Calculate average duration
    pub fn calculate_avg_duration(&mut self) {
        self.avg_duration_ms = if self.attempts > 0 {
            self.total_duration_ms as f64 / self.attempts as f64
        } else {
            0.0
        };
    }

    /// Set the number of compiled allowlist regex patterns
    pub fn set_allowlist_regex_count(&mut self, count: usize) {
        self.allowlist_regex_count = count as u64;
    }

    /// Set the number of compiled blocklist regex patterns
    pub fn set_blocklist_regex_count(&mut self, count: usize) {
        self.blocklist_regex_count = count as u64;
    }
}
// Note: The TextInjector trait has been moved to lib.rs to avoid conflicts.
// Use crate::TextInjector for the canonical async trait definition.
