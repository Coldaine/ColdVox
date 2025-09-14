fn main() {
    // Generate and build the C++ for #[cxx_qt::bridge] in src/bridge.rs
    // and link the required Qt6 modules.
    cxx_qt_build::CxxQtBuilder::new()
        .file("src/bridge.rs")
        .qt_module("Core")
        .qt_module("Gui")
        .qt_module("Qml")
        .qt_module("Quick")
        .build();
}
