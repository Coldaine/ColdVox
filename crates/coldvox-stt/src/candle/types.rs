//! Candle-specific Whisper domain types.

/// Represents a contiguous transcription segment with optional timing.
#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
    pub start: f32,
    pub end: f32,
    pub text: String,
}

impl Segment {
    pub fn new(start: f32, end: f32, text: String) -> Self {
        Self { start, end, text }
    }
}

/// Result of a Whisper decoding run.
#[derive(Debug, Clone, PartialEq)]
pub struct Transcript {
    pub segments: Vec<Segment>,
    pub language: Option<String>,
}

impl Transcript {
    pub fn from_text(text: String) -> Self {
        Self {
            segments: vec![Segment::new(0.0, 0.0, text)],
            language: None,
        }
    }

    pub fn push_segment(&mut self, segment: Segment) {
        self.segments.push(segment);
    }
}
