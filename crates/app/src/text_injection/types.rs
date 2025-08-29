use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Enumeration of all available text injection methods
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InjectionMethod {
    /// Insert text directly using AT-SPI2 EditableText interface
    AtspiInsert,
    /// Set the Wayland clipboard with text
    Clipboard,
    /// Set clipboard then trigger paste via AT-SPI2 Action interface
    ClipboardAndPaste,
    /// Use ydotool to simulate Ctrl+V paste (opt-in)
    YdoToolPaste,
    /// Use kdotool for window activation/focus assistance (opt-in)
    KdoToolAssist,
    /// Use enigo library for synthetic text/paste (opt-in)
    EnigoText,
    /// Use mouse-keyboard-input for synthetic key events (opt-in, last resort)
    UinputKeys,
}

/// Configuration for text injection system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InjectionConfig {
    /// Whether to allow ydotool usage (requires external binary and uinput permissions)
    #[serde(default = "default_false")]
    pub allow_ydotool: bool,
    /// Whether to allow kdotool usage (external CLI for KDE window activation)
    #[serde(default = "default_false")]
    pub allow_kdotool: bool,
    /// Whether to allow enigo library usage (Wayland/libei paths)
    #[serde(default = "default_false")]
    pub allow_enigo: bool,
    /// Whether to allow mouse-keyboard-input usage (uinput)
    #[serde(default = "default_false")]
    pub allow_mki: bool,
    /// Whether to restore the clipboard content after injection
    #[serde(default = "default_false")]
    pub restore_clipboard: bool,
    /// Whether to allow injection when focus state is unknown
    #[serde(default = "default_false")]
    pub inject_on_unknown_focus: bool,

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
}

fn default_false() -> bool {
    false
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

impl Default for InjectionConfig {
    fn default() -> Self {
        Self {
            allow_ydotool: default_false(),
            allow_kdotool: default_false(),
            allow_enigo: default_false(),
            allow_mki: default_false(),
            restore_clipboard: default_false(),
            inject_on_unknown_focus: default_false(),
            max_total_latency_ms: default_max_total_latency_ms(),
            per_method_timeout_ms: default_per_method_timeout_ms(),
            paste_action_timeout_ms: default_paste_action_timeout_ms(),
            cooldown_initial_ms: default_cooldown_initial_ms(),
            cooldown_backoff_factor: default_cooldown_backoff_factor(),
            cooldown_max_ms: default_cooldown_max_ms(),
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
    
    #[cfg(feature = "text-injection-clipboard")]
    #[error("Clipboard error: {0}")]
    Clipboard(#[from] wl_clipboard_rs::copy::Error),
    
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
}