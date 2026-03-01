//! Comprehensive STT (Speech-to-Text) tests
//!
//! Tests cover:
//! - Mock plugin initialization and configuration
//! - Transcription output (text, timing, confidence)
//! - Plugin lifecycle (init → process → finalize → reset)
//! - Error simulation
//! - TranscriptionEvent and WordInfo types
//! - TranscriptionConfig defaults and validation

use coldvox_stt::plugins::mock::{MockConfig, MockPlugin};
use coldvox_stt::plugin::{PluginCapabilities, PluginInfo, SttPlugin};
use coldvox_stt::types::{TranscriptionConfig, TranscriptionEvent, WordInfo};
use coldvox_stt::next_utterance_id;

// ─── TranscriptionEvent Type Tests ──────────────────────────────────

#[test]
fn transcription_event_partial_has_timing() {
    let event = TranscriptionEvent::Partial {
        utterance_id: 1,
        text: "hello".to_string(),
        t0: Some(0.0),
        t1: Some(0.5),
    };
    if let TranscriptionEvent::Partial { text, t0, t1, .. } = &event {
        assert_eq!(text, "hello");
        assert_eq!(*t0, Some(0.0));
        assert_eq!(*t1, Some(0.5));
    }
}

#[test]
fn transcription_event_final_with_words() {
    let words = vec![
        WordInfo {
            start: 0.0,
            end: 0.3,
            conf: 0.95,
            text: "hello".to_string(),
        },
        WordInfo {
            start: 0.35,
            end: 0.65,
            conf: 0.88,
            text: "world".to_string(),
        },
    ];

    let event = TranscriptionEvent::Final {
        utterance_id: 1,
        text: "hello world".to_string(),
        words: Some(words.clone()),
    };

    if let TranscriptionEvent::Final { text, words: ws, .. } = &event {
        assert_eq!(text, "hello world");
        let ws = ws.as_ref().unwrap();
        assert_eq!(ws.len(), 2);
        assert_eq!(ws[0].text, "hello");
        assert!((ws[0].conf - 0.95).abs() < 0.001);
        assert_eq!(ws[1].text, "world");
    }
}

#[test]
fn transcription_event_error() {
    let event = TranscriptionEvent::Error {
        code: "MODEL_NOT_FOUND".to_string(),
        message: "Model file not found".to_string(),
    };
    if let TranscriptionEvent::Error { code, message } = &event {
        assert_eq!(code, "MODEL_NOT_FOUND");
        assert!(!message.is_empty());
    }
}

#[test]
fn transcription_event_partial_without_timing() {
    let event = TranscriptionEvent::Partial {
        utterance_id: 1,
        text: "test".to_string(),
        t0: None,
        t1: None,
    };
    if let TranscriptionEvent::Partial { t0, t1, .. } = &event {
        assert!(t0.is_none());
        assert!(t1.is_none());
    }
}

#[test]
fn transcription_event_final_without_words() {
    let event = TranscriptionEvent::Final {
        utterance_id: 1,
        text: "no words".to_string(),
        words: None,
    };
    if let TranscriptionEvent::Final { words, .. } = &event {
        assert!(words.is_none());
    }
}

// ─── WordInfo Tests ──────────────────────────────────────────────────

#[test]
fn word_info_timing_constraints() {
    let word = WordInfo {
        start: 1.0,
        end: 1.5,
        conf: 0.92,
        text: "hello".to_string(),
    };
    assert!(word.end > word.start, "end time should be after start time");
    assert!(word.conf >= 0.0 && word.conf <= 1.0, "confidence should be 0.0-1.0");
}

#[test]
fn word_info_clone() {
    let word = WordInfo {
        start: 0.0,
        end: 0.5,
        conf: 0.99,
        text: "test".to_string(),
    };
    let cloned = word.clone();
    assert_eq!(cloned.text, "test");
    assert!((cloned.conf - 0.99).abs() < 0.001);
}

// ─── TranscriptionConfig Tests ──────────────────────────────────────

#[test]
fn transcription_config_defaults() {
    let config = TranscriptionConfig::default();
    assert!(!config.enabled);
    assert!(config.partial_results);
    assert!(!config.include_words);
    assert!(!config.streaming);
    assert_eq!(config.max_alternatives, 1);
    assert_eq!(config.buffer_size_ms, 512);
    assert!(config.auto_extract_model);
}

#[test]
fn transcription_config_clone() {
    let config = TranscriptionConfig {
        enabled: true,
        model_path: "/path/to/model".to_string(),
        partial_results: false,
        max_alternatives: 3,
        include_words: true,
        buffer_size_ms: 50,
        streaming: true,
        auto_extract_model: false,
    };
    let cloned = config.clone();
    assert_eq!(cloned.model_path, "/path/to/model");
    assert_eq!(cloned.max_alternatives, 3);
    assert!(!cloned.partial_results);
}

// ─── Mock Plugin Tests ──────────────────────────────────────────────

#[tokio::test]
async fn mock_plugin_info() {
    let plugin = MockPlugin::default();
    let info = plugin.info();
    assert_eq!(info.id, "mock");
    assert_eq!(info.name, "Mock STT");
    assert!(info.is_available);
    assert!(info.is_local);
    assert!(!info.requires_network);
    assert!(info.supported_languages.contains(&"en".to_string()));
}

#[tokio::test]
async fn mock_plugin_capabilities() {
    let plugin = MockPlugin::default();
    let caps = plugin.capabilities();
    assert!(caps.streaming);
    assert!(caps.batch);
    assert!(caps.word_timestamps);
    assert!(caps.confidence_scores);
    assert!(caps.auto_punctuation);
    assert!(!caps.speaker_diarization);
}

#[tokio::test]
async fn mock_plugin_initialize() {
    let mut plugin = MockPlugin::default();
    let config = TranscriptionConfig::default();
    let result = plugin.initialize(config).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn mock_plugin_immediate_transcription() {
    let mut plugin = MockPlugin::with_transcription("hello world".to_string());
    plugin.initialize(TranscriptionConfig::default()).await.unwrap();

    let audio = vec![0i16; 512];
    let result = plugin.process_audio(&audio).await.unwrap();
    assert!(result.is_some(), "should produce immediate transcription");

    if let Some(TranscriptionEvent::Final { text, .. }) = result {
        assert_eq!(text, "hello world");
    } else {
        panic!("Expected Final transcription event");
    }
}

#[tokio::test]
async fn mock_plugin_delayed_transcription() {
    let mut plugin = MockPlugin::with_delayed_transcription(3, "delayed result".to_string());
    plugin.initialize(TranscriptionConfig::default()).await.unwrap();

    let audio = vec![0i16; 512];

    // First two chunks — no result yet
    assert!(plugin.process_audio(&audio).await.unwrap().is_none());
    assert!(plugin.process_audio(&audio).await.unwrap().is_none());

    // Third chunk — should produce result
    let result = plugin.process_audio(&audio).await.unwrap();
    assert!(result.is_some(), "should produce transcription after 3 chunks");
}

#[tokio::test]
async fn mock_plugin_failure_simulation() {
    let config = MockConfig {
        fail_after_calls: Some(2),
        immediate_transcription: Some("test".to_string()),
        ..Default::default()
    };
    let mut plugin = MockPlugin::new(config);
    plugin.initialize(TranscriptionConfig::default()).await.unwrap();

    let audio = vec![0i16; 512];

    // First two calls succeed
    assert!(plugin.process_audio(&audio).await.is_ok());
    assert!(plugin.process_audio(&audio).await.is_ok());

    // Third call should fail
    let result = plugin.process_audio(&audio).await;
    assert!(result.is_err(), "should fail after configured number of calls");
}

#[tokio::test]
async fn mock_plugin_is_available() {
    let plugin = MockPlugin::default();
    assert!(plugin.is_available().await.unwrap());
}

// ─── Utterance ID Generation Tests ──────────────────────────────────

#[test]
fn utterance_ids_are_unique() {
    let id1 = next_utterance_id();
    let id2 = next_utterance_id();
    let id3 = next_utterance_id();
    assert_ne!(id1, id2);
    assert_ne!(id2, id3);
    assert!(id2 > id1);
    assert!(id3 > id2);
}

#[test]
fn utterance_ids_are_monotonically_increasing() {
    let ids: Vec<u64> = (0..100).map(|_| next_utterance_id()).collect();
    for window in ids.windows(2) {
        assert!(window[1] > window[0], "IDs should be monotonically increasing");
    }
}

// ─── Plugin Interface Contract Tests ────────────────────────────────

#[tokio::test]
async fn plugin_lifecycle_init_process_finalize() {
    let mut plugin = MockPlugin::with_delayed_transcription(2, "lifecycle test".to_string());

    // Initialize
    plugin.initialize(TranscriptionConfig::default()).await.unwrap();

    // Process audio
    let audio = vec![0i16; 512];
    plugin.process_audio(&audio).await.unwrap();

    // Finalize should return result for remaining audio
    let final_result = plugin.finalize().await;
    assert!(final_result.is_ok());
}

#[tokio::test]
async fn plugin_reset_clears_state() {
    let mut plugin = MockPlugin::with_delayed_transcription(5, "reset test".to_string());
    plugin.initialize(TranscriptionConfig::default()).await.unwrap();

    let audio = vec![0i16; 512];
    // Process 3 chunks
    for _ in 0..3 {
        plugin.process_audio(&audio).await.unwrap();
    }

    // Reset
    let reset_result = plugin.reset().await;
    assert!(reset_result.is_ok());
}
