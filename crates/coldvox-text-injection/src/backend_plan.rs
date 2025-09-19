//! # Backend Planning and Selection
//!
//! This module centralizes the logic for selecting and ordering text injection backends.
//! It provides a pure function, `plan_backends`, that deterministically returns an
//! ordered list of `InjectionMethod`s based on system capabilities and user configuration.
//!
//! The goal is to have a single, testable source of truth for the backend fallback chain,
//! preserving the documented policy:
//! **AT-SPI → Clipboard+Paste (combo) → Clipboard → Input Simulation (YDotool/Enigo)**.

use crate::backend::{Backend, BackendDetector};
use crate::types::{InjectionConfig, InjectionMethod};
use std::collections::HashSet;

/// Determines the prioritized list of injection methods based on available backends and configuration.
///
/// This function encapsulates the complex decision-making process for choosing which injectors
/// to try and in what order. It ensures the documented fallback policy is respected while
/// allowing for user-configured overrides (e.g., enabling `ydotool` or `enigo`).
///
/// # Arguments
///
/// * `config` - The user-provided `InjectionConfig`.
/// * `backend_detector` - A detector that has probed the system for available backends.
///
/// # Returns
///
/// A `Vec<InjectionMethod>` containing the ordered list of methods to attempt. The list
/// is deduplicated, and `NoOp` is always included as a final fallback.
pub fn plan_backends(
    config: &InjectionConfig,
    backend_detector: &dyn BackendDetector,
) -> Vec<InjectionMethod> {
    let available_backends = backend_detector.detect_available_backends();
    let mut base_order: Vec<InjectionMethod> = Vec::new();

    // The core fallback strategy is based on the presence of desktop environment backends.
    let has_desktop_backend = available_backends.iter().any(|b| {
        matches!(
            b,
            Backend::WaylandXdgDesktopPortal
                | Backend::WaylandVirtualKeyboard
                | Backend::X11Xdotool
                | Backend::X11Native
                | Backend::MacCgEvent
                | Backend::WindowsSendInput
        )
    });

    if has_desktop_backend {
        // Standard Linux Desktop Fallback Chain
        base_order.push(InjectionMethod::AtspiInsert);
        base_order.push(InjectionMethod::ClipboardAndPaste);
        base_order.push(InjectionMethod::Clipboard);
    }

    // Add optional, opt-in fallbacks. These are typically less reliable or require
    // special permissions, so they come after the primary methods.
    if config.allow_kdotool {
        base_order.push(InjectionMethod::KdoToolAssist);
    }
    if config.allow_enigo {
        base_order.push(InjectionMethod::EnigoText);
    }
    if config.allow_ydotool {
        base_order.push(InjectionMethod::YdoToolPaste);
    }

    // Deduplicate the list while preserving the insertion order.
    let mut seen = HashSet::new();
    base_order.retain(|m| seen.insert(*m));

    // Always include NoOp at the end as a guaranteed fallback. This ensures that the
    // injection process can always "succeed" gracefully even if no real backends work.
    if !base_order.contains(&InjectionMethod::NoOp) {
        base_order.push(InjectionMethod::NoOp);
    }

    base_order
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::Backend;
    use crate::types::InjectionConfig;

    /// A mock detector for testing `plan_backends` in different scenarios.
    struct MockDetector {
        backends: Vec<Backend>,
    }

    impl MockDetector {
        fn new(backends: Vec<Backend>) -> Self {
            Self { backends }
        }
    }

    impl BackendDetector for MockDetector {
        fn detect_available_backends(&self) -> Vec<Backend> {
            self.backends.clone()
        }
        // Other trait methods are not needed for this test.
        fn get_preferred_backend(&self) -> Option<Backend> {
            self.backends.first().cloned()
        }
        fn check_audio_setup(&self) -> Result<(), crate::types::InjectionError> {
            Ok(())
        }
    }

    #[test]
    fn test_plan_with_wayland_backend() {
        let config = InjectionConfig::default();
        let detector = MockDetector::new(vec![Backend::WaylandXdgDesktopPortal]);
        let plan = plan_backends(&config, &detector);

        let expected = vec![
            InjectionMethod::AtspiInsert,
            InjectionMethod::ClipboardAndPaste,
            InjectionMethod::Clipboard,
            InjectionMethod::NoOp,
        ];
        assert_eq!(plan, expected);
    }

    #[test]
    fn test_plan_with_x11_backend() {
        let config = InjectionConfig::default();
        let detector = MockDetector::new(vec![Backend::X11Native]);
        let plan = plan_backends(&config, &detector);

        let expected = vec![
            InjectionMethod::AtspiInsert,
            InjectionMethod::ClipboardAndPaste,
            InjectionMethod::Clipboard,
            InjectionMethod::NoOp,
        ];
        assert_eq!(plan, expected);
    }

    #[test]
    fn test_plan_with_optional_backends_enabled() {
        let config = InjectionConfig {
            allow_ydotool: true,
            allow_enigo: true,
            allow_kdotool: true,
            ..Default::default()
        };
        let detector = MockDetector::new(vec![Backend::WaylandVirtualKeyboard]);
        let plan = plan_backends(&config, &detector);

        let expected = vec![
            InjectionMethod::AtspiInsert,
            InjectionMethod::ClipboardAndPaste,
            InjectionMethod::Clipboard,
            InjectionMethod::KdoToolAssist,
            InjectionMethod::EnigoText,
            InjectionMethod::YdoToolPaste,
            InjectionMethod::NoOp,
        ];
        assert_eq!(plan, expected);
    }

    #[test]
    fn test_plan_with_no_desktop_backend() {
        let config = InjectionConfig {
            allow_ydotool: true,
            ..Default::default()
        };
        let detector = MockDetector::new(vec![]); // No backends detected
        let plan = plan_backends(&config, &detector);

        // Without a core desktop backend, the plan should only include the opt-in methods.
        let expected = vec![InjectionMethod::YdoToolPaste, InjectionMethod::NoOp];
        assert_eq!(plan, expected);
    }

    #[test]
    fn test_plan_deduplication() {
        let config = InjectionConfig::default();
        // Simulate a scenario where detection might hypothetically return duplicates
        let detector =
            MockDetector::new(vec![Backend::WaylandXdgDesktopPortal, Backend::X11Native]);
        let plan = plan_backends(&config, &detector);

        // The core desktop methods should only appear once.
        let expected = vec![
            InjectionMethod::AtspiInsert,
            InjectionMethod::ClipboardAndPaste,
            InjectionMethod::Clipboard,
            InjectionMethod::NoOp,
        ];
        assert_eq!(plan, expected);
    }
}
