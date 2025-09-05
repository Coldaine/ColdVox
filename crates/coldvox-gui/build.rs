fn main() {
    // Re-run if the bridge changes or this build script updates
    println!("cargo:rerun-if-changed=src/bridge.rs");
    println!("cargo:rerun-if-changed=build.rs");

    // Only engage CXX-Qt codegen and Qt linkage when the feature is enabled.
    let feature_enabled = std::env::var("CARGO_FEATURE_QT_UI").is_ok();
    if !feature_enabled {
        return;
    }

    // Use cxx-qt-build to generate the C++/Rust glue and link Qt modules.
    let builder = cxx_qt_build::CxxQtBuilder::new()
        .file("src/bridge.rs")
        // Core is implicit; add GUI/QML/Quick for future QML UI work.
        .qt_module("Gui")
        .qt_module("Qml")
        .qt_module("Quick");

    builder.build();
}
