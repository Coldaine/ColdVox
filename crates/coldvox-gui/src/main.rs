#[cfg(feature = "qt-ui")]
mod run_ui {
    use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QString, QUrl};

    // Bring the bridge module (generated via cxx-qt) into scope when needed.
    #[allow(unused_imports)]
    use crate::bridge::ffi;

    pub fn main() {
        // Create a Qt GUI app and load our QML UI.
        let _app = QGuiApplication::new();

        let engine = QQmlApplicationEngine::new();

        // Load the main QML from the crate's qml directory.
        let qml_path = format!("{}/qml/Main.qml", env!("CARGO_MANIFEST_DIR"));
        let url = QUrl::from_local_file(&QString::from(&qml_path));
        engine.load_url(&url);

        // Enter Qt event loop.
        let _ = QGuiApplication::exec();
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
