use std::io::{stdout, Write};
use crossterm::{cursor::MoveTo, style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor}, terminal, QueueableCommand};

/// Simple terminal indicator shown while recording is active.
/// Draws a small bar centered horizontally about one-third from the bottom
/// of the terminal. Designed to be unobtrusive and easily ignored.
pub struct RecordingIndicator {
    displayed: bool,
}

impl RecordingIndicator {
    pub fn new() -> Self {
        Self { displayed: false }
    }

    /// Show the indicator if it is not already visible.
    pub fn show(&mut self) {
        if self.displayed {
            return;
        }
        if let Ok((cols, rows)) = terminal::size() {
            let msg = " Recording ";
            let x = cols.saturating_sub(msg.len() as u16) / 2;
            let y = rows.saturating_mul(2) / 3; // one-third from bottom
            let mut out = stdout();
            let _ = out
                .queue(MoveTo(x, y))
                .and_then(|o| o.queue(SetForegroundColor(Color::White)))
                .and_then(|o| o.queue(SetBackgroundColor(Color::DarkGrey)))
                .and_then(|o| o.queue(Print(msg)))
                .and_then(|o| o.queue(ResetColor))
                .and_then(|o| o.flush());
        }
        self.displayed = true;
    }

    /// Hide the indicator if visible.
    pub fn hide(&mut self) {
        if !self.displayed {
            return;
        }
        if let Ok((cols, rows)) = terminal::size() {
            let msg = " Recording ";
            let x = cols.saturating_sub(msg.len() as u16) / 2;
            let y = rows.saturating_mul(2) / 3;
            let mut out = stdout();
            let blank = " ".repeat(msg.len());
            let _ = out
                .queue(MoveTo(x, y))
                .and_then(|o| o.queue(Print(blank)))
                .and_then(|o| o.flush());
        }
        self.displayed = false;
    }
}

impl Default for RecordingIndicator {
    fn default() -> Self {
        Self::new()
    }
}
