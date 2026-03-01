//! Comprehensive text-injection pipeline tests.
//!
//! Tests cover:
//! - InjectionConfig defaults and serialization
//! - InjectionContext building
//! - InjectionMethod enumeration
//! - NoOp injector (always available, does nothing — used as test double)
//! - TextInjector trait contract via NoOp
//! - Session state management
//! - InjectionMode (Paste vs Keystroke)

use coldvox_text_injection::{
    InjectionConfig, InjectionContext, InjectionMethod, InjectionMode,
    InjectionResult, TextInjector,
};
use coldvox_text_injection::session::{SessionConfig, SessionState};

// ─── InjectionConfig Tests ──────────────────────────────────────────

/// Helper to create an InjectionConfig with serde defaults (no Default impl).
fn default_injection_config() -> InjectionConfig {
    serde_json::from_str("{}").expect("empty JSON should produce default InjectionConfig")
}

#[test]
fn injection_config_defaults() {
    let config = default_injection_config();
    // Verify key defaults
    assert!(!config.allow_kdotool, "kdotool should be disabled by default");
    assert!(!config.allow_enigo, "enigo should be disabled by default");
    assert!(config.inject_on_unknown_focus, "should inject on unknown focus by default");
    assert!(!config.require_focus, "require_focus should be false by default");
    assert!(config.redact_logs, "should redact logs by default");
    assert!(!config.fail_fast, "fail_fast should be disabled by default");
    assert_eq!(config.max_total_latency_ms, 800);
    assert_eq!(config.per_method_timeout_ms, 250);
    assert_eq!(config.paste_action_timeout_ms, 200);
    assert_eq!(config.keystroke_rate_cps, 20);
    assert_eq!(config.injection_mode, "auto");
}

#[test]
fn injection_config_serde_roundtrip() {
    let config = default_injection_config();
    let json = serde_json::to_string(&config).expect("should serialize");
    let deserialized: InjectionConfig = serde_json::from_str(&json).expect("should deserialize");
    assert_eq!(config.allow_kdotool, deserialized.allow_kdotool);
    assert_eq!(config.inject_on_unknown_focus, deserialized.inject_on_unknown_focus);
    assert_eq!(config.max_total_latency_ms, deserialized.max_total_latency_ms);
}

#[test]
fn injection_config_custom_values_roundtrip() {
    let json = r#"{
        "allow_kdotool": true,
        "allow_enigo": true,
        "inject_on_unknown_focus": false,
        "require_focus": false,
        "redact_logs": false,
        "max_total_latency_ms": 5000,
        "per_method_timeout_ms": 2000,
        "paste_action_timeout_ms": 1000,
        "injection_mode": "paste",
        "keystroke_rate_cps": 100,
        "fail_fast": true
    }"#;
    let config: InjectionConfig = serde_json::from_str(json).expect("should parse custom config");
    assert!(config.allow_kdotool);
    assert!(config.allow_enigo);
    assert!(!config.inject_on_unknown_focus);
    assert!(!config.require_focus);
    assert_eq!(config.max_total_latency_ms, 5000);
    assert_eq!(config.injection_mode, "paste");
    assert!(config.fail_fast);
}

// ─── InjectionContext Tests ─────────────────────────────────────────

#[test]
fn injection_context_default_is_empty() {
    let ctx = InjectionContext::default();
    assert!(ctx.target_app.is_none());
    assert!(ctx.window_id.is_none());
    assert!(ctx.atspi_focused_node_path.is_none());
    assert!(ctx.clipboard_backup.is_none());
    assert!(ctx.mode_override.is_none());
}

#[test]
fn injection_context_with_target_app() {
    let ctx = InjectionContext {
        target_app: Some("firefox".to_string()),
        window_id: Some("0x12345".to_string()),
        ..Default::default()
    };
    assert_eq!(ctx.target_app.as_deref(), Some("firefox"));
    assert_eq!(ctx.window_id.as_deref(), Some("0x12345"));
}

#[test]
fn injection_context_with_mode_override() {
    let ctx = InjectionContext {
        mode_override: Some(InjectionMode::Paste),
        ..Default::default()
    };
    assert_eq!(ctx.mode_override, Some(InjectionMode::Paste));

    let ctx2 = InjectionContext {
        mode_override: Some(InjectionMode::Keystroke),
        ..Default::default()
    };
    assert_eq!(ctx2.mode_override, Some(InjectionMode::Keystroke));
}

#[test]
fn injection_context_clone() {
    let ctx = InjectionContext {
        target_app: Some("vscode".to_string()),
        clipboard_backup: Some("previous clipboard".to_string()),
        ..Default::default()
    };
    let cloned = ctx.clone();
    assert_eq!(cloned.target_app, ctx.target_app);
    assert_eq!(cloned.clipboard_backup, ctx.clipboard_backup);
}

// ─── InjectionMethod Tests ──────────────────────────────────────────

#[test]
fn injection_method_variants() {
    let methods = vec![
        InjectionMethod::AtspiInsert,
        InjectionMethod::ClipboardPasteFallback,
        InjectionMethod::KdoToolAssist,
        InjectionMethod::EnigoText,
        InjectionMethod::NoOp,
    ];
    assert_eq!(methods.len(), 5);
}

#[test]
fn injection_method_equality() {
    assert_eq!(InjectionMethod::AtspiInsert, InjectionMethod::AtspiInsert);
    assert_ne!(InjectionMethod::AtspiInsert, InjectionMethod::NoOp);
}

#[test]
fn injection_method_serde_roundtrip() {
    let method = InjectionMethod::ClipboardPasteFallback;
    let json = serde_json::to_string(&method).expect("serialize");
    let deserialized: InjectionMethod = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(method, deserialized);
}

#[test]
fn injection_method_hash_works() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(InjectionMethod::AtspiInsert);
    set.insert(InjectionMethod::NoOp);
    set.insert(InjectionMethod::AtspiInsert); // duplicate
    assert_eq!(set.len(), 2);
}

// ─── InjectionMode Tests ────────────────────────────────────────────

#[test]
fn injection_mode_variants() {
    assert_ne!(InjectionMode::Paste, InjectionMode::Keystroke);
    let mode = InjectionMode::Paste;
    let mode2 = mode; // Copy
    assert_eq!(mode, mode2);
}

// ─── Session State Tests ────────────────────────────────────────────

#[test]
fn session_state_default_is_idle() {
    let state = SessionState::default();
    assert_eq!(state, SessionState::Idle);
}

#[test]
fn session_state_display() {
    assert_eq!(format!("{}", SessionState::Idle), "IDLE");
    assert_eq!(format!("{}", SessionState::Buffering), "BUFFERING");
    assert_eq!(format!("{}", SessionState::WaitingForSilence), "WAITING_FOR_SILENCE");
    assert_eq!(format!("{}", SessionState::ReadyToInject), "READY_TO_INJECT");
}

#[test]
fn session_state_equality() {
    assert_eq!(SessionState::Idle, SessionState::Idle);
    assert_ne!(SessionState::Idle, SessionState::Buffering);
}

#[test]
fn session_config_defaults() {
    let config = SessionConfig::default();
    assert!(config.max_buffer_size > 0, "should have positive buffer size");
    assert!(config.flush_on_punctuation, "should flush on punctuation by default");
    assert!(config.normalize_whitespace, "should normalize whitespace by default");
    assert!(!config.punctuation_marks.is_empty(), "should have punctuation marks");
}

// ─── NoOp Injector Integration (via TextInjector trait) ─────────────

// Note: The NoOp injector is the test-friendly injector that's always available.
// This section tests it as a stand-in for the TextInjector trait contract.

use coldvox_text_injection::noop_injector::NoOpInjector;

#[tokio::test]
async fn noop_injector_inject_text_succeeds() {
    let injector = NoOpInjector::new(default_injection_config());
    let result = injector.inject_text("hello world", None).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn noop_injector_inject_empty_text() {
    let injector = NoOpInjector::new(default_injection_config());
    let result = injector.inject_text("", None).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn noop_injector_inject_with_context() {
    let injector = NoOpInjector::new(default_injection_config());
    let ctx = InjectionContext {
        target_app: Some("test-app".to_string()),
        mode_override: Some(InjectionMode::Paste),
        ..Default::default()
    };
    let result = injector.inject_text("test", Some(&ctx)).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn noop_injector_is_always_available() {
    let injector = NoOpInjector::new(default_injection_config());
    assert!(injector.is_available().await);
}

#[tokio::test]
async fn noop_injector_backend_name() {
    let injector = NoOpInjector::new(default_injection_config());
    assert_eq!(injector.backend_name(), "NoOp");
}

#[tokio::test]
async fn noop_injector_backend_info_not_empty() {
    let injector = NoOpInjector::new(default_injection_config());
    let info = injector.backend_info();
    assert!(!info.is_empty());
}

#[tokio::test]
async fn noop_injector_inject_unicode() {
    let injector = NoOpInjector::new(default_injection_config());
    let result = injector.inject_text("こんにちは 🌍 café résumé", None).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn noop_injector_inject_long_text() {
    let injector = NoOpInjector::new(default_injection_config());
    let text = "a".repeat(10_000);
    let result = injector.inject_text(&text, None).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn noop_injector_inject_special_chars() {
    let injector = NoOpInjector::new(default_injection_config());
    let result = injector.inject_text("Hello\nWorld\t!\r\nLine3", None).await;
    assert!(result.is_ok());
}
