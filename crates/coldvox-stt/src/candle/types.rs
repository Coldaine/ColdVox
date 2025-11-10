//! Candle-specific Whisper domain types.

/// Represents a contiguous transcription segment with timing and confidence information.
#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
    pub start: f32,
    pub end: f32,
    pub text: String,
    pub confidence: f32,
    pub word_count: usize,
    pub words: Option<Vec<WordTiming>>,
}

impl Segment {
    /// Create a new segment with basic timing information.
    pub fn new(start: f32, end: f32, text: String) -> Self {
        Self {
            start,
            end,
            text: text.trim().to_string(),
            confidence: 0.0,
            word_count: text.split_whitespace().count(),
            words: None,
        }
    }

    /// Create a new segment with confidence score.
    pub fn with_confidence(start: f32, end: f32, text: String, confidence: f32) -> Self {
        let mut segment = Self::new(start, end, text);
        segment.confidence = confidence.clamp(0.0, 1.0);
        segment
    }

    /// Create a new segment with word-level timing information.
    pub fn with_words(start: f32, end: f32, text: String, words: Vec<WordTiming>) -> Self {
        let mut segment = Self::new(start, end, text);
        segment.words = Some(words.clone());
        segment.word_count = words.len();
        
        // Calculate average confidence from words if available
        if !words.is_empty() {
            let total_confidence: f32 = words.iter().map(|w| w.confidence).sum();
            segment.confidence = total_confidence / words.len() as f32;
        }
        
        segment
    }

    /// Get the duration of this segment in seconds.
    pub fn duration(&self) -> f32 {
        (self.end - self.start).max(0.0)
    }

    /// Check if this segment has valid timing information.
    pub fn has_timing(&self) -> bool {
        self.start >= 0.0 && self.end >= self.start && self.duration() > 0.0
    }

    /// Get a summary of this segment for logging/debugging.
    pub fn summary(&self) -> String {
        format!(
            "{:.1}s-{:.1}s ({:.1}s): \"{}\" [{:.1}% confidence, {} words]",
            self.start,
            self.end,
            self.duration(),
            self.text,
            self.confidence * 100.0,
            self.word_count
        )
    }
}

/// Word-level timing information for precise timestamp alignment.
#[derive(Debug, Clone, PartialEq)]
pub struct WordTiming {
    pub text: String,
    pub start: f32,
    pub end: f32,
    pub confidence: f32,
}

impl WordTiming {
    /// Create new word timing information.
    pub fn new(text: String, start: f32, end: f32, confidence: f32) -> Self {
        Self {
            text: text.trim().to_string(),
            start: start.max(0.0),
            end: end.max(start),
            confidence: confidence.clamp(0.0, 1.0),
        }
    }

    /// Get the duration of this word in seconds.
    pub fn duration(&self) -> f32 {
        (self.end - self.start).max(0.0)
    }

    /// Check if this word has valid timing.
    pub fn has_valid_timing(&self) -> bool {
        self.end >= self.start && self.duration() >= 0.0
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
