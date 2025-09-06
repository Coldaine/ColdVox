fn main() {
    // Tell Cargo to re-run this build script if these files change
    // This ensures the CXX-Qt bridge is regenerated when modifications are made
    println!("cargo:rerun-if-changed=src/bridge.rs");
    println!("cargo:rerun-if-changed=build.rs");

    // Check if the qt-ui feature is enabled
    // This allows the crate to build without Qt dependencies for non-GUI use cases
    // The environment variable is set by Cargo when --features qt-ui is specified
    let feature_enabled = std::env::var("CARGO_FEATURE_QT_UI").is_ok();
    if !feature_enabled {
        // Exit early if Qt UI is not requested, keeping the build lightweight
        return;
    }

    // Configure CXX-Qt code generation
    // This builder:
    // 1. Parses the Rust bridge definition in src/bridge.rs
    // 2. Generates C++ MOC (Meta-Object Compiler) code for Qt integration
    // 3. Compiles and links the generated C++ code
    // 4. Creates Rust bindings that connect to the C++ Qt objects
    let builder = cxx_qt_build::CxxQtBuilder::new()
        .file("src/bridge.rs")
        // Link required Qt modules:
        // - Gui: Basic GUI functionality and window system integration
        // - Qml: QML engine for declarative UI
        // - Quick: Qt Quick components for modern QML interfaces
        // Note: Core module is implicitly included by CXX-Qt
        .qt_module("Gui")
        .qt_module("Qml")
        .qt_module("Quick");

    // Execute the build process
    // This generates bridge.cxxqt.rs in OUT_DIR which is included by main.rs
    builder.build();
}
