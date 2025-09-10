// CXX-Qt bridge for Rust-QML interoperability
// This module defines the bridge between Rust backend and Qt/QML frontend
// Only compiled when the `qt-ui` feature is enabled to keep non-GUI builds clean

// Gated at the module site in main.rs via `#[cfg(feature = "qt-ui")] mod bridge;`

use cxx_qt_lib::QVariant;

// The state of the core application logic, exposed to the GUI.
// This is a Q_ENUM, so it can be used directly in QML.
#[cxx_qt::qenum]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppState {
    Idle,
    Activating,
    Active,
    Paused,
    Stopping,
    Error,
}

impl Default for AppState {
    fn default() -> Self {
        Self::Idle
    }
}

// The CXX-Qt bridge macro generates C++ binding code and Rust trait implementations
// This enables seamless communication between Rust and Qt's object system
#[cxx_qt::bridge]
mod ffi {
    // The "RustQt" extern block is CXX-Qt 0.7's required pattern for defining
    // QObjects that are implemented in Rust but exposed to Qt/QML
    // The 'unsafe' is required because we're crossing the FFI boundary between
    // Rust and C++ where Rust's safety guarantees cannot be automatically enforced
    unsafe extern "RustQt" {
        // Re-export the AppState enum to be visible to Qt
        #[qenum]
        type AppState = super::AppState;

        // Define a QObject that will be accessible from QML
        // The #[qobject] attribute tells CXX-Qt to generate Qt Meta-Object Compiler (MOC) data
        #[qobject]
        // Properties are declared on the type definition, not as struct fields
        // This generates getter/setter methods and a '...Changed' signal automatically
        #[qproperty(bool, expanded)]
        #[qproperty(AppState, state)]
        #[qproperty(QString, last_error)]
        // Map the Qt-visible type to our Rust implementation struct
        // This separation allows us to keep Rust logic separate from Qt bindings
        type GuiBridge = super::GuiBridgeRust;

        // These methods are invokable from QML. They form the command interface
        // for the user to control the application.

        /// Starts the STT engine. Transitions from Idle -> Active.
        #[qinvokable]
        fn cmd_start(self: Pin<&mut GuiBridge>);

        /// Stops the STT engine. Transitions from Active/Paused -> Idle.
        #[qinvokable]
        fn cmd_stop(self: Pin<&mut GuiBridge>);

        /// Pauses the STT engine. Transitions from Active -> Paused.
        #[qinvokable]
        fn cmd_pause(self: Pin<&mut GuiBridge>);

        /// Resumes the STT engine. Transitions from Paused -> Active.
        #[qinvokable]
        fn cmd_resume(self: Pin<&mut GuiBridge>);

        /// Clears any error state. Transitions from Error -> Idle.
        #[qinvokable]
        fn cmd_clear_error(self: Pin<&mut GuiBridge>);
    }
}

// The actual Rust struct that backs the QObject
// This must have fields matching the properties declared above
#[derive(Default)]
pub struct GuiBridgeRust {
    expanded: bool,
    state: AppState,
    last_error: String,
}

impl GuiBridge {
    /// Starts the STT engine.
    /// Valid transitions:
    /// - Idle -> Active
    pub fn cmd_start(self: Pin<&mut Self>) {
        let current_state = *self.as_ref().state();
        if current_state == AppState::Idle {
            self.set_state(AppState::Active);
        } else {
            // TODO: Log a warning about invalid state transition
        }
    }

    /// Stops the STT engine.
    /// Valid transitions:
    /// - Active -> Idle
    /// - Paused -> Idle
    pub fn cmd_stop(self: Pin<&mut Self>) {
        let current_state = *self.as_ref().state();
        if matches!(current_state, AppState::Active | AppState::Paused) {
            self.set_state(AppState::Idle);
        } else {
            // TODO: Log a warning
        }
    }

    /// Pauses the STT engine.
    /// Valid transitions:
    /// - Active -> Paused
    pub fn cmd_pause(self: Pin<&mut Self>) {
        if *self.as_ref().state() == AppState::Active {
            self.set_state(AppState::Paused);
        } else {
            // TODO: Log a warning
        }
    }

    /// Resumes the STT engine.
    /// Valid transitions:
    /// - Paused -> Active
    pub fn cmd_resume(self: Pin<&mut Self>) {
        if *self.as_ref().state() == AppState::Paused {
            self.set_state(AppState::Active);
        } else {
            // TODO: Log a warning
        }
    }

    /// Clears any error state and returns to Idle.
    pub fn cmd_clear_error(mut self: Pin<&mut Self>) {
        if *self.as_ref().state() == AppState::Error {
            self.as_mut().set_state(AppState::Idle);
            self.as_mut().set_last_error("".to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cxx_qt::CxxQtThread;
    use std::pin::Pin;

    /// Helper to create a GuiBridge instance on the CXX-Qt thread
    fn create_bridge() -> Pin<Box<GuiBridge>> {
        GuiBridge::new()
    }

    #[test]
    fn test_initial_state() {
        let bridge = create_bridge();
        assert_eq!(*bridge.as_ref().state(), AppState::Idle);
        assert_eq!(*bridge.as_ref().expanded(), false);
        assert_eq!(*bridge.as_ref().last_error(), "");
    }

    #[test]
    fn test_transition_idle_to_active() {
        let mut bridge = create_bridge();
        bridge.as_mut().cmd_start();
        assert_eq!(*bridge.as_ref().state(), AppState::Active);
    }

    #[test]
    fn test_transition_active_to_idle() {
        let mut bridge = create_bridge();
        bridge.as_mut().set_state(AppState::Active);
        bridge.as_mut().cmd_stop();
        assert_eq!(*bridge.as_ref().state(), AppState::Idle);
    }

    #[test]
    fn test_transition_active_to_paused() {
        let mut bridge = create_bridge();
        bridge.as_mut().set_state(AppState::Active);
        bridge.as_mut().cmd_pause();
        assert_eq!(*bridge.as_ref().state(), AppState::Paused);
    }

    #[test]
    fn test_transition_paused_to_active() {
        let mut bridge = create_bridge();
        bridge.as_mut().set_state(AppState::Paused);
        bridge.as_mut().cmd_resume();
        assert_eq!(*bridge.as_ref().state(), AppState::Active);
    }

    #[test]
    fn test_transition_paused_to_idle() {
        let mut bridge = create_bridge();
        bridge.as_mut().set_state(AppState::Paused);
        bridge.as_mut().cmd_stop();
        assert_eq!(*bridge.as_ref().state(), AppState::Idle);
    }

    #[test]
    fn test_transition_error_to_idle() {
        let mut bridge = create_bridge();
        bridge.as_mut().set_state(AppState::Error);
        bridge.as_mut().set_last_error("Something went wrong".to_string());
        bridge.as_mut().cmd_clear_error();
        assert_eq!(*bridge.as_ref().state(), AppState::Idle);
        assert_eq!(*bridge.as_ref().last_error(), "");
    }

    #[test]
    fn test_invalid_transition_start_from_active() {
        let mut bridge = create_bridge();
        bridge.as_mut().set_state(AppState::Active);
        bridge.as_mut().cmd_start();
        // State should not change
        assert_eq!(*bridge.as_ref().state(), AppState::Active);
    }

    #[test]
    fn test_invalid_transition_pause_from_idle() {
        let mut bridge = create_bridge();
        // ensure it's idle
        bridge.as_mut().set_state(AppState::Idle);
        bridge.as_mut().cmd_pause();
        // State should not change
        assert_eq!(*bridge.as_ref().state(), AppState::Idle);
    }

    #[test]
    fn test_invalid_transition_resume_from_active() {
        let mut bridge = create_bridge();
        bridge.as_mut().set_state(AppState::Active);
        bridge.as_mut().cmd_resume();
        // State should not change
        assert_eq!(*bridge.as_ref().state(), AppState::Active);
    }
}
