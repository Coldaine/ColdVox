#[cfg(feature = "qt-ui")]
mod run_ui {
    use cxx_qt_lib::QGuiApplication;

    // Bring the bridge module (generated via cxx-qt) into scope when needed.
    #[allow(unused_imports)]
    use crate::bridge::ffi;

    pub fn main() {
        // Create a minimal Qt GUI application; no UI loaded yet.
        // This verifies Qt linking works when the feature is enabled.
        let _app = QGuiApplication::new();
        // Intentionally do not load QML or create a window yet.
        // This binary will start and exit immediately when run.
        println!("ColdVox GUI groundwork: Qt + CXX-Qt linked (qt-ui feature).");
    }
}

#[cfg(not(feature = "qt-ui"))]
fn main() {
    println!("ColdVox GUI groundwork ready.");
    println!("Enable with: cargo run -p coldvox-gui --features qt-ui");
}

#[cfg(feature = "qt-ui")]
fn main() {
    run_ui::main();
}

#[cfg(feature = "qt-ui")]
mod bridge;
