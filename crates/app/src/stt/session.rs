use std::time::Instant;

/// Source of an STT activation session
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionSource {
    Vad,
    Hotkey,
}

/// Events that define the lifecycle of a transcription session.
/// This abstracts away the difference between VAD and Hotkey activation.
pub enum SessionEvent {
    /// A session has started.
    Start(SessionSource, Instant),
    /// A session has ended cleanly.
    End(SessionSource, Instant),
    /// A session was aborted.
    Abort(SessionSource, &'static str),
    // Future: A long-running session has been split by a silence detector.
    // SegmentSplit(SessionSource, Instant),
}

/// Defines the primary activation method for STT.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActivationMode {
    /// Push-to-talk.
    Hotkey,
    /// Ambient voice activity detection.
    Vad,
}

/// Defines how a hotkey press is treated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HotkeyBehavior {
    /// (Default) Process audio incrementally as it arrives.
    /// Partial results are emitted throughout the keypress.
    Incremental,
    // Future: Buffer all audio during keypress and process it all on release.
    // BatchOnRelease,
    // Future: A hybrid approach combining Incremental and BatchOnRelease.
    // Hybrid,
}

/// Policy for emitting partial transcription results.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PartialPolicy {
    /// (Default) Emit all partial results as they are generated.
    Emit,
    // Future: Suppress all partial results, only emitting the final transcription.
    // Suppress,
    // Future: Emit partial results at most once per duration.
    // Throttle(std::time::Duration),
}

/// Configuration for the "long-hold" hotkey behavior (stubbed for future use).
#[derive(Clone, Debug)]
pub struct LongHoldStub {
    pub enabled: bool,
    pub min_hold_secs: u32,
    pub silence_split_secs: u32,
}

impl Default for LongHoldStub {
    fn default() -> Self {
        Self {
            enabled: false,
            min_hold_secs: 10,
            silence_split_secs: 2,
        }
    }
}

/// Unified settings for the STT processor.
#[derive(Clone, Debug)]
pub struct Settings {
    pub activation_mode: ActivationMode,
    pub hotkey_behavior: HotkeyBehavior,
    pub partial_policy: PartialPolicy,
    pub long_hold: LongHoldStub,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            activation_mode: ActivationMode::Hotkey,
            hotkey_behavior: HotkeyBehavior::Incremental,
            partial_policy: PartialPolicy::Emit,
            long_hold: LongHoldStub::default(),
        }
    }
}
