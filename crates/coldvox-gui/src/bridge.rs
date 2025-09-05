// Minimal QObject bridge scaffold for future UI/backend wiring.
// Only compiled and generated when the `qt-ui` feature is enabled.

#![cfg(feature = "qt-ui")]

use core::pin::Pin;

#[cxx_qt::bridge]
mod ffi {
    // Expose Qt types used by the object. (cxx-qt-lib provides wrappers)
    unsafe extern "C++" {}

    /// GUI bridge object (stub). Extend later with signals/slots.
    #[qobject]
    pub struct GuiBridge {
        #[qproperty]
        pub expanded: bool,
        #[qproperty]
        pub level: i32,
        #[qproperty]
        pub state: i32,
        #[qproperty]
        pub transcript: QString,
    }

    impl Default for GuiBridge {
        fn default() -> Self {
            Self {
                expanded: false,
                level: 0,
                state: 0,
                transcript: QString::from(&""),
            }
        }
    }

    impl qobject::GuiBridge {
        /// Start recording (stub)
        #[qinvokable]
        pub fn cmd_start(self: Pin<&mut qobject::GuiBridge>) {
            let mut this = self;
            this.set_state(1);
        }

        /// Toggle pause/resume (stub)
        #[qinvokable]
        pub fn cmd_toggle_pause(self: Pin<&mut qobject::GuiBridge>) {
            let mut this = self;
            let s = this.state();
            this.set_state(if s == 1 { 0 } else { 1 });
        }

        /// Stop (stub)
        #[qinvokable]
        pub fn cmd_stop(self: Pin<&mut qobject::GuiBridge>) {
            let mut this = self;
            this.set_state(2);
        }

        /// Clear transcript (stub)
        #[qinvokable]
        pub fn cmd_clear(self: Pin<&mut qobject::GuiBridge>) {
            let mut this = self;
            this.set_transcript(QString::from(&""));
        }
    }
}
