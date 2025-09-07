// CXX-Qt bridge for Rust-QML interoperability
// This module defines the bridge between Rust backend and Qt/QML frontend
// Only compiled when the `qt-ui` feature is enabled to keep non-GUI builds clean

// Gated at the module site in main.rs via `#[cfg(feature = "qt-ui")] mod bridge;`

// The CXX-Qt bridge macro generates C++ binding code and Rust trait implementations
// This enables seamless communication between Rust and Qt's object system
#[cxx_qt::bridge]
mod ffi {
    // The "RustQt" extern block is CXX-Qt 0.7's required pattern for defining
    // QObjects that are implemented in Rust but exposed to Qt/QML
    // The 'unsafe' is required because we're crossing the FFI boundary between
    // Rust and C++ where Rust's safety guarantees cannot be automatically enforced
    unsafe extern "RustQt" {
        // Define a QObject that will be accessible from QML
        // The #[qobject] attribute tells CXX-Qt to generate Qt Meta-Object Compiler (MOC) data
        #[qobject]
        // Properties are declared on the type definition, not as struct fields
        // This generates getter/setter methods and a 'expandedChanged' signal automatically
        #[qproperty(bool, expanded)]
        // Map the Qt-visible type to our Rust implementation struct
        // This separation allows us to keep Rust logic separate from Qt bindings
        type GuiBridge = super::GuiBridgeRust;
    }
}

// The actual Rust struct that backs the QObject
// This must have fields matching the properties declared above
// Using #[derive(Default)] provides initialization with expanded = false
#[derive(Default)]
pub struct GuiBridgeRust {
    expanded: bool, // Tracks whether the GUI is in expanded or collapsed state
}
