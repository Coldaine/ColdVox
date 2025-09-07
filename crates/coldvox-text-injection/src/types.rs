//! # Core Data Types for Text Injection
//!
//! This module defines fundamental data structures used across the crate,
//! primarily for configuration.

use serde::{Deserialize, Serialize};
use std::time::Duration;

// NOTE: The main error and outcome types have been moved to `error.rs` and
// `outcome.rs` for better organization. This file now focuses on configuration
// and method identification.

/// Enumeration of all available text injection methods.
/// This is used internally by the `StrategyManager` to decide which injector to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InjectionMethod {
    /// Insert text directly using AT-SPI2 EditableText interface.
    AtspiInsert,
    /// Set the clipboard with text and then trigger a paste action.
    Clipboard,
    /// A combination of setting the clipboard and then using a separate tool to paste.
    ClipboardAndPaste,
    /// Use ydotool to simulate Ctrl+V paste (opt-in).
    YdoToolPaste,
    /// Use kdotool for window activation/focus assistance (opt-in).
    KdoToolAssist,
    /// Use enigo library for synthetic text/paste (opt-in).
    EnigoText,
    /// No-op fallback injector (always succeeds, does nothing).
    NoOp,
}

/// Configuration for the text injection system.
/// This struct is typically deserialized from a configuration file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InjectionConfig {
    /// Whether to allow ydotool usage (requires external binary and uinput permissions).
    #[serde(default = "default_false")]
    pub allow_ydotool: bool,
    /// Whether to allow kdotool usage (external CLI for KDE window activation).
    #[serde(default = "default_false")]
    pub allow_kdotool: bool,
    /// Whether to allow enigo library usage.
    #[serde(default = "default_false")]
    pub allow_enigo: bool,

    /// Whether to restore the clipboard content after a clipboard-based injection.
    #[serde(default = "default_true")]
    pub restore_clipboard: bool,
    /// Environment variable to enforce clipboard restoration, even if it fails.
    /// If "1", a restore failure will be a hard error. Otherwise, it's a warning.
    #[serde(default = "default_false")]
    pub enforce_clipboard_restore: bool,

    /// Whether to allow injection when the focus state is unknown.
    #[serde(default = "default_true")]
    pub inject_on_unknown_focus: bool,

    /// Whether to require an editable UI element to have focus before injecting.
    #[serde(default = "default_true")]
    pub require_focus: bool,

    /// Whether to redact text content in logs for privacy.
    #[serde(default = "default_true")]
    pub redact_logs: bool,

    /// Allowlist of application patterns (regex) for injection.
    #[serde(default)]
    pub allowlist: Vec<String>,

    /// Blocklist of application patterns (regex) to prevent injection.
    #[serde(default)]
    pub blocklist: Vec<String>,
}

fn default_false() -> bool {
    false
}

fn default_true() -> bool {
    true
}

impl Default for InjectionConfig {
    fn default() -> Self {
        Self {
            allow_ydotool: default_false(),
            allow_kdotool: default_false(),
            allow_enigo: default_false(),
            restore_clipboard: default_true(),
            enforce_clipboard_restore: default_false(),
            inject_on_unknown_focus: default_true(),
            require_focus: default_true(),
            redact_logs: default_true(),
            allowlist: Vec::new(),
            blocklist: Vec::new(),
        }
    }
}
