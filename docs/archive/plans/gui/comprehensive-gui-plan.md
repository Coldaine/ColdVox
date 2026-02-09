---
doc_type: plan
subsystem: gui
status: draft
freshness: dead
preservation: delete
last_reviewed: 2025-10-19
owners: Documentation Working Group
version: 1.0.0
---

# Comprehensive GUI Development Plan for ColdVox

## 1.0 Introduction

This document outlines a phased development plan for creating a modern, Qt/QML-based Graphical User Interface (GUI) for the ColdVox application. The target platform is Fedora/Nobara KDE Plasma on Wayland, utilizing the `cxx-qt` library for bridging Rust and C++.

The plan follows an incremental approach, beginning with environment setup and toolchain validation, moving to a minimal viable product, and progressively adding core features and advanced visual effects. Each phase is designed to be a verifiable checkpoint.

---

## 2.0 Phase 1: Environment Setup & Toolchain Validation

**Goal:** To prepare the development environment and project structure, culminating in a successful compilation of a minimal Rust-to-C++ bridge. This phase ensures that all system dependencies, the Rust toolchain, and the `cxx-qt` build process are correctly configured before any UI code is written.

### 2.1 System Dependencies (Fedora/Nobara)

Install the necessary packages for C++ development, Qt6, and graphics/windowing systems.

```bash
# Install core C++ build tools
sudo dnf groupinstall -y "Development Tools"

# Install build system tools and Qt6 development libraries
sudo dnf install -y cmake ninja-build pkgconf-pkg-config \
  qt6-qtbase-devel qt6-qtdeclarative-devel qt6-qtquickcontrols2-devel \
  qt6-qttools-devel qt6-qtshadertools-devel

# Install graphics and windowing system headers for Wayland and X11
sudo dnf install -y mesa-libEGL-devel mesa-libGL-devel mesa-dri-drivers \
  libX11-devel libxcb-devel wayland-devel

# (Optional) Install tools for QML debugging and profiling
sudo dnf install -y qt6-qtdeclarative-devel-tools
```

**Verification:** Ensure Qt6 is correctly installed and accessible.

```bash
/usr/lib64/qt6/bin/qmake6 -query QT_VERSION
```

*Expect an output of `6.6.0` or higher.*

### 2.2 Rust Toolchain

A standard stable Rust toolchain is required.

```bash
# Install rustup if not already present
curl https://sh.rustup.rs -sSf | sh -s -- -y
source "$HOME/.cargo/env"

# Set to stable and add essential components
rustup default stable
rustup component add rustfmt clippy

# Verify installation
cargo --version && rustc --version
```

### 2.3 Project Scaffolding

Create a new crate to house the GUI code, keeping it separate from the core application logic.

```bash
cargo new --lib crates/coldvox-gui
```

*Remember to add `"crates/coldvox-gui"` to the `[workspace.members]` list in the root `Cargo.toml`.*

### 2.4 CXX-Qt Integration

Wire up the `coldvox-gui` crate to use `cxx-qt`. We will use a specific commit from the `cxx-qt` git repository to ensure a stable and reproducible build.

**Important:** Using a `git` dependency is necessary for features not yet on crates.io. Pinning to a specific `rev` is critical to prevent the build from breaking when the `cxx-qt` `main` branch is updated. Pick a recent, stable commit SHA from the [KDAB/cxx-qt repository](https://github.com/KDAB/cxx-qt) and replace `<REV>` with it.

**`crates/coldvox-gui/Cargo.toml`**
```toml
[package]
name = "coldvox-gui"
version = "0.1.0"
edition = "2021"

[dependencies]
cxx = "1.0"
cxx-qt = { git = "https://github.com/KDAB/cxx-qt.git", rev = "2a968056c60c31a1a610e00c511f9b02b6033f13" }
cxx-qt-lib = { git = "https://github.com/KDAB/cxx-qt.git", rev = "2a968056c60c31a1a610e00c511f9b02b6033f13", features = ["full"] }
tokio = "1"
thiserror = "1.0"
tracing = "0.1"

[build-dependencies]
cxx-qt-build = { git = "https://github.com/KDAB/cxx-qt.git", rev = "2a968056c60c31a1a610e00c511f9b02b6033f13" }

# The patch section ensures that all dependencies in the tree use the same
# git version of cxx-qt, preventing version conflicts.
[patch.crates-io]
cxx-qt = { git = "https://github.com/KDAB/cxx-qt.git", rev = "2a968056c60c31a1a610e00c511f9b02b6033f13" }
cxx-qt-lib = { git = "https://github.com/KDAB/cxx-qt.git", rev = "2a968056c60c31a1a610e00c511f9b02b6033f13" }
cxx-qt-build = { git = "https://github.com/KDAB/cxx-qt.git", rev = "2a968056c60c31a1a610e00c511f9b02b6033f13" }
```

**`crates/coldvox-gui/build.rs`**
```rust
use cxx_qt_build::CxxQtBuilder;

fn main() {
    CxxQtBuilder::new()
        // Link the Rust code in bridge.rs
        .file("src/bridge.rs")
        // Link the necessary Qt modules
        .qt_module("Core")
        .qt_module("Gui")
        .qt_module("Qml")
        .qt_module("Quick")
        .build();
}
```

### 2.5 Initial Build (Validation)

Create a minimal `main.rs` and `bridge.rs` to confirm that the toolchain is fully functional before writing any QML.

**`crates/coldvox-gui/src/bridge.rs`**
```rust
#[cxx_qt::bridge]
pub mod ffi {
    #[cxx_qt::qobject(qml_uri = "com.coldvox.app", qml_version = "1.0")]
    #[derive(Default)]
    pub struct GuiBridge {
        #[qproperty]
        status: cxx_qt_lib::QString,
    }
}
```

**`crates/coldvox-gui/src/main.rs`**
```rust
mod bridge;

fn main() {
    // This is enough to validate that linking against Qt works.
    let _app = cxx_qt_lib::QGuiApplication::new();
    assert!(cxx_qt_lib::QCoreApplication::instance().is_some());
    println!("Toolchain validation successful: Qt libraries linked.");
}
```

**Build and Verify:**
```bash
cargo clean
cargo build -p coldvox-gui -v
```
*This command **must** complete successfully. If it fails, do not proceed. Use the troubleshooting guide (Section 6.2) to resolve the first error.*

---

## 3.0 Phase 2: Minimal Viable GUI (Smoke Test)

**Goal:** To render a simple QML window that demonstrates two-way communication with the Rust backend. This proves the entire Rust -> C++ -> QML pipeline is working.

### 3.1 The Rust-QML Bridge

Expand the bridge to include a property and an invokable method.

**`crates/coldvox-gui/src/bridge.rs`**
```rust
use cxx_qt_lib::QString;

#[cxx_qt::bridge(qml_uri = "com.coldvox.app", qml_version = "1.0")]
pub mod ffi {
    // A QObject exposed to QML. `qml_element` registers it as a QML type.
    #[qobject(qml_element)]
    #[derive(Default)]
    pub struct GuiBridge {
        // A property that can be read and written from QML.
        #[qproperty]
        status: QString,
    }

    // This block is for CXX-Qt internals and default values.
    impl qobject::GuiBridge {
        pub fn default() -> Self {
            Self {
                status: QString::from("ready"),
            }
        }
    }

    // This block is for methods exposed to QML.
    #[cxx_qt::inherit]
    impl qobject::GuiBridge {
        // A function that can be called from QML.
        #[qinvokable]
        pub fn ping(self: core::pin::Pin<&mut qobject::GuiBridge>, s: &QString) {
            let mut cur = self.as_ref().status().to_string();
            cur.push_str(":");
            cur.push_str(&s.to_string());
            self.as_ref().set_status(QString::from(cur));
        }
    }
}
```

### 3.2 The QML Scene

Create a QML file that uses the `GuiBridge` object.

**`crates/coldvox-gui/qml/main.qml`**
```qml
import QtQuick
import QtQuick.Controls
// Import the Rust module defined by the qml_uri
import com.coldvox.app 1.0

ApplicationWindow {
    id: win
    visible: true
    width: 480; height: 320
    title: "ColdVox Minimal"

    // Instantiate the Rust QObject
    GuiBridge { id: bridge }

    Column {
        anchors.centerIn: parent
        spacing: 10

        // Bind this Text's content to the Rust property
        Text { text: "Status: " + bridge.status }

        // Call the Rust method when the button is clicked
        Button { text: "Ping"; onClicked: bridge.ping("ok") }
    }
}
```

### 3.3 The Application Main Loop

Update `main.rs` to load and execute the QML engine.

**`crates/coldvox-gui/src/main.rs`**
```rust
mod bridge;

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QUrl};

fn main() {
    let _app = QGuiApplication::new();
    let mut engine = QQmlApplicationEngine::new();

    // Ensure the QML file can be found relative to the executable.
    // This path may need adjustment depending on your final bundle structure.
    let qml_path = std::env::current_dir().unwrap().join("crates/coldvox-gui/qml/main.qml");
    let qml_url = QUrl::from(format!("file://{}", qml_path.display()).as_str());

    engine.load_url(&qml_url);

    // Start the Qt event loop.
    QGuiApplication::exec();
}
```

### 3.4 Validation

Run the application.

```bash
cargo run -p coldvox-gui
```
*You should see a window appear. The text should initially show "Status: ready". Clicking the "Ping" button should append ":ok" to the status string repeatedly.*

---

## 4.0 Phase 3: Core Feature Implementation & Backend Integration

**Goal:** To replace the minimal smoke-test bridge with a real data model capable of displaying transcription data.

### 4.1 Strategy: Incremental Refinement

We will build the data model in steps, verifying each change before proceeding. This isolates potential issues and simplifies debugging.

### 4.2 Step A: A Simple List Model

First, expose a simple list of strings to represent transcribed words.

**`crates/coldvox-gui/src/bridge.rs`**
```rust
// ... existing bridge ...
#[cxx_qt::bridge(qml_uri = "com.coldvox.app", qml_version = "1.0")]
pub mod ffi {
    // Define a QObject to hold our list data
    #[qobject]
    #[derive(Default)]
    pub struct TranscriptModel {
        #[qproperty]
        words: Vec<QString>,
    }

    #[qobject(qml_element)]
    #[derive(Default)]
    pub struct GuiBridge {
        #[qproperty(c_type = "TranscriptModel*", read = get_transcript_model)]
        transcript_model: *mut TranscriptModel,
        // ... other properties
    }

    #[cxx_qt::inherit]
    impl qobject::GuiBridge {
        // Getter for the model property
        pub fn get_transcript_model(&self) -> *mut TranscriptModel {
            self.transcript_model
        }

        // Method to initialize the model and set its parent
        #[qinvokable]
        pub fn initialize(self: core::pin::Pin<&mut qobject::GuiBridge>) {
            let model = TranscriptModel::new();
            // Set the parent to ensure the model is deleted with the bridge
            model.set_parent(self.as_ref().pin());
            self.set_transcript_model(model.pin());

            // Add some dummy data
            let mut words = self.as_ref().transcript_model().words();
            words.append(QString::from("hello"));
            words.append(QString::from("world"));
            self.as_ref().transcript_model().set_words(words);
        }
    }
    // ...
}
```
*You will need to call `bridge.initialize()` from QML, for example in `Component.onCompleted`.*

### 4.3 Step B: A Richer Gadget-Based Model

A `Vec<QString>` is limiting. Use a `#[qgadget]` to create a custom struct that can be used in `Vec`s exposed to QML. This allows passing structured data (e.g., word + timestamp).

**`crates/coldvox-gui/src/bridge.rs`**
```rust
// ...
#[cxx_qt::bridge(qml_uri = "com.coldvox.app", qml_version = "1.0")]
pub mod ffi {
    // A qgadget is a value-type struct usable in QML models.
    #[qgadget]
    #[derive(Default, Clone)]
    pub struct TranscriptWord {
        pub text: QString,
        pub timestamp: f64,
    }

    #[qobject]
    #[derive(Default)]
    pub struct TranscriptModel {
        #[qproperty]
        words: Vec<TranscriptWord>,
    }
    // ... update GuiBridge and its impls to use this new model ...
}
```
*In QML, your `ListView` delegate can now access `modelData.text` and `modelData.timestamp`.*

### 4.4 Next Steps: Connecting the Backend

At this point, the GUI has a functional data model but it's populated with static data. The next step is to connect this model to the ColdVox `AppHandle` to receive real-time `TranscriptionEvent`s and update the `TranscriptModel` accordingly. This will involve using Qt's signal/slot mechanism across the Rust/C++ boundary.

---

## 5.0 Phase 4: Advanced Visuals & User Experience

**Goal:** To transform the basic window into a modern, frameless overlay.

### 5.1 Creating an Overlay Window

In `qml/main.qml`, modify the `ApplicationWindow` to remove borders and stay on top.

```qml
ApplicationWindow {
    // ...
    flags: Qt.Window | Qt.FramelessWindowHint | Qt.WindowStaysOnTopHint
}
```

### 5.2 Enabling Translucency

This is a two-part process:

1.  **In `main.rs`**, before creating the `QGuiApplication`, set the translucent background attribute. This hints to the window manager that the window may have transparent areas.

    ```rust
    use cxx_qt_lib::{QCoreApplication, WidgetAttribute};
    // ...
    fn main() {
        QCoreApplication::set_attribute(WidgetAttribute::WA_TranslucentBackground, true);
        let _app = QGuiApplication::new();
        // ...
    }
    ```

2.  **In `qml/main.qml`**, set the window's background color to transparent.

    ```qml
    ApplicationWindow {
        // ...
        color: "transparent"
    }
    ```

### 5.3 Wayland Considerations

-   **Global Hotkeys:** Implementing true global hotkeys on Wayland is complex and desktop-environment specific. This functionality should be deferred. For initial development, use local QML `Shortcut` items for keyboard interactions when the window is focused.
-   **Always-on-Top:** The `Qt.WindowStaysOnTopHint` is generally well-supported by KWin (the KDE window manager) on Wayland.

### 5.4 Future Visuals

The following advanced features are planned for after the core functionality is stable:
-   Collapse/expand functionality via a keyboard shortcut.
-   A fixed-step physics-based animation for words (e.g., a spiral).
-   Custom shaders for visual effects (`SingularityLens.qml`, `StarfieldBackground.qml`).

---

## 6.0 Development Workflow & Troubleshooting

### 6.1 Development Workflow (Justfile)

To streamline common tasks, a `Justfile` is recommended.

**`Justfile`**
```make
set shell := ["bash","-eu","-o","pipefail","-c"]

# Build the GUI crate verbosely
build:
    cargo build -p coldvox-gui -v

# Run the GUI application
run:
    cargo run -p coldvox-gui

# Clean the project
clean:
    cargo clean

# Inspect the dependency tree for cxx-qt and Qt versions
tree:
    cargo tree | grep -E 'cxx-qt($|_|-)|Qt6' -n
```

### 6.2 Troubleshooting Guide

-   **Error: `unsupported attribute` on `#[qenum]`, `#[qgadget]`, etc.**
    -   **Cause:** You are likely not using the macros from the git version of `cxx-qt`.
    -   **Solution:** Verify that your `Cargo.lock` file shows that all `cxx-qt*` crates are sourced from `git+...#<REV>`. Run `cargo tree` to confirm. Ensure the `[patch.crates-io]` section in `Cargo.toml` is correct.

-   **Error: Linker errors (`undefined reference to ...`) related to Qt symbols.**
    -   **Cause:** The linker cannot find the Qt6 libraries.
    -   **Solution:** Reinstall the `qt6-*-devel` packages. Verify that `qmake6 -query QT_VERSION` works and reports a version >= 6.6.

-   **Error: `Only trait declarations, extern C++ functions, and type declarations are supported...`**
    -   **Cause:** You have placed Rust implementation logic (e.g., a function body) inside the `#[cxx_qt::bridge]` block.
    -   **Solution:** Move all Rust `impl` blocks for your QObject outside the bridge block, into a separate `#[cxx_qt::inherit]` block.

-   **Error (QML): `module "com.coldvox.app" is not installed`**
    -   **Cause:** The QML engine cannot find the Rust module.
    -   **Solution:** Ensure the `qml_uri` in your `#[cxx_qt::bridge]` macro exactly matches the `import` statement in your QML file. Also, this error will appear if the Rust code failed to compile, as the module is never generated. Fix any compilation errors first.
