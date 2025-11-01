//! Modular injectors for different text injection methods
//!
//! This module provides a modular organization for different text injection methods.
//! Each injector implements the TextInjector trait and provides specific functionality
//! for different platforms and injection strategies.

pub mod atspi;
pub mod clipboard;
pub mod unified_clipboard;

// Re-export common types for convenience
#[allow(deprecated)]
pub use atspi::Context as AtspiContext;
#[allow(deprecated)]
pub use clipboard::{ClipboardBackup, ClipboardInjector, Context as ClipboardContext};
pub use unified_clipboard::{
    ClipboardBackup as UnifiedClipboardBackup, ClipboardInjectionMode, UnifiedClipboardInjector,
};
