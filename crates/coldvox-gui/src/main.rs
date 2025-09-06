#[cfg(feature = "qt-ui")]
mod run_ui {
    use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QString, QUrl};

    // Bring the bridge module (generated via cxx-qt) into scope when needed.
    #[allow(unused_imports)]
    use crate::bridge::ffi;

    pub fn main() {
        // Initialize the Qt application
        // QGuiApplication manages the GUI application's control flow and main settings
        // The 'mut' is required because we need a Pin<&mut> reference for the exec() call
        let mut app = QGuiApplication::new();

        // Create the QML application engine
        // This engine loads and instantiates QML components and manages the QML context
        let mut engine = QQmlApplicationEngine::new();

        // Construct the path to our QML file
        // Using CARGO_MANIFEST_DIR ensures we find the QML file during development
        // TODO: For production, consider using Qt Resource System (qrc) for embedded resources
        let qml_path = format!("{}/qml/Main.qml", env!("CARGO_MANIFEST_DIR"));
        let url = QUrl::from_local_file(&QString::from(&qml_path));

        // Load the QML file into the engine
        // The as_mut() returns Option<Pin<&mut T>> which we need to unwrap
        // This Pin API is required by CXX-Qt for safe C++ interop
        if let Some(engine_pin) = engine.as_mut() {
            engine_pin.load(&url);
            // TODO: Add error handling - check if engine.root_objects().is_empty()
        }

        // Start the Qt event loop
        // This blocks until the application exits (window closed or quit() called)
        // The exec() method requires a pinned mutable reference for thread safety
        if let Some(app_pin) = app.as_mut() {
            let _ = app_pin.exec();
        }
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