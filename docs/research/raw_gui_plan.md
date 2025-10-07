# Raw Plan for GUI Integration

> ⚠️ **RESEARCH DOCUMENT - WORK IN PROGRESS**
> Contains incomplete sections and future work markers.
> Last updated: 2025-10-07

This is a raw, unedited plan for setting up a Qt-based GUI on Fedora/Nobara.

### 0) Preconditions

* Nobara/Fedora 39–41, KDE Plasma on Wayland.
* You have sudo.

### 1) System packages

```bash
sudo dnf groupinstall -y "Development Tools"
sudo dnf install -y cmake ninja-build pkgconf-pkg-config \
  qt6-qtbase-devel qt6-qtdeclarative-devel qt6-qtquickcontrols2-devel \
  qt6-qttools-devel qt6-qtshadertools-devel \
  mesa-libEGL-devel mesa-libGL-devel mesa-dri-drivers \
  libX11-devel libxcb-devel wayland-devel
# optional QML tooling
sudo dnf install -y qt6-qtdeclarative-devel-tools
# verify Qt
/usr/lib64/qt6/bin/qmake6 -query QT_VERSION
```

Expect Qt ≥ 6.6.x.

### 2) Rust toolchain

```bash
# if not already present
curl https://sh.rustup.rs -sSf | sh -s -- -y
source "$HOME/.cargo/env"
rustup default stable
rustup component add rustfmt clippy
cargo --version && rustc --version
```

### 3) Project wiring for CXX-Qt 0.8 from git

Pick and record a fixed commit SHA from KDAB/cxx-qt `main` (example placeholder `REV`).

**crates/coldvox-gui/Cargo.toml**

```toml
[package]
name = "coldvox-gui"
version = "0.1.0"
edition = "2021"

[dependencies]
cxx = "1"
cxx-qt = { git = "https://github.com/KDAB/cxx-qt.git", rev = "<REV>" }
cxx-qt-lib = { git = "https://github.com/KDAB/cxx-qt.git", rev = "<REV>", features = ["full"] }

[build-dependencies]
cxx-qt-build = { git = "https://github.com/KDAB/cxx-qt.git", rev = "<REV>" }

[patch.crates-io]
cxx-qt = { git = "https://github.com/KDAB/cxx-qt.git", rev = "<REV>" }
cxx-qt-lib = { git = "https://github.com/KDAB/cxx-qt.git", rev = "<REV>" }
cxx-qt-build = { git = "https://github.com/KDAB/cxx-qt.git", rev = "<REV>" }
```

**crates/coldvox-gui/build.rs**

```rust
use cxx_qt_build::CxxQtBuilder;
fn main() {
    CxxQtBuilder::new()
        .file("src/bridge.rs")
        .qt_module("Core")
        .qt_module("Gui")
        .qt_module("Qml")
        .qt_module("Quick")
        .build();
}
```

### 4) Minimal compile target (no GUI yet)

Goal: prove toolchain. One QML-exposed QObject, no QML scene.

**crates/coldvox-gui/src/bridge.rs**

```rust
use cxx_qt_lib::QString;

#[cxx_qt::bridge(qml_uri = "com.coldvox.app", qml_version = "1.0")]
pub mod ffi {
    #[qobject(qml_element)]
    pub struct GuiBridge {
        #[qproperty]
        status: QString,
    }

    impl Default for qobject::GuiBridge {
        fn default() -> Self {
            Self { status: QString::from("ready") }
        }
    }

    #[cxx_qt::inherit]
    impl qobject::GuiBridge {
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

**crates/coldvox-gui/src/main.rs**

```rust
use cxx_qt_lib::{QCoreApplication, QGuiApplication};

fn main() {
    // No QML load yet. Just init/teardown to validate link.
    let _app = QGuiApplication::new();
    // Quick smoke: ensure QCoreApplication exists.
    assert!(QCoreApplication::instance().is_some());
}
```

Build:

```bash
cargo clean
cargo build -p coldvox-gui -v
cargo tree | grep -E 'cxx-qt($|_|-)|Qt6' -n
```

This must compile. If not, stop and fix the first error.

### 5) Wire basic QML (still minimal)

Add `qml/main.qml`:

```qml
import QtQuick
import QtQuick.Controls
import com.coldvox.app 1.0

ApplicationWindow {
  id: win
  visible: true
  width: 480; height: 320
  title: "ColdVox Minimal"

  GuiBridge { id: bridge }
  Column {
    anchors.centerIn: parent
    Text { text: bridge.status }
    Button { text: "Ping"; onClicked: bridge.ping("ok") }
  }
}
```

Update **main.rs** to load QML:

```rust
use cxx_qt_lib::{QCoreApplication, QGuiApplication, QQmlApplicationEngine, QUrl};

fn main() {
    let _app = QGuiApplication::new();
    let mut engine = QQmlApplicationEngine::new();
    // Use file URL to your qml/main.qml path
    let path = std::env::current_dir().unwrap().join("qml/main.qml");
    engine.load_url(QUrl::from(format!("file://{}", path.display()).as_str()));
    QGuiApplication::exec();
}
```

Run:

```bash
cargo run -p coldvox-gui
```

### 6) Enable overlay traits for KDE (Wayland)

* In QML later: `flags: Qt.Window | Qt.FramelessWindowHint | Qt.WindowStaysOnTopHint`.
* For translucency:

  * Add before QGuiApplication in **main.rs**:

    ```rust
    use cxx_qt_lib::WidgetAttribute;
    QCoreApplication::set_attribute(WidgetAttribute::WA_TranslucentBackground, true);
    ```
  * In QML: `color: "transparent"`.
* KWin settings: “Allow applications to block compositing” off (default ok). Window rules not required for dev.

### 7) Move toward your real model incrementally

Replace the minimal bridge with the 0.8 pattern you want, stepwise:

**Step A: add TranscriptModel**

```rust
#[qobject]
#[derive(Default)]
pub struct TranscriptModel {
    #[qproperty]
    words: Vec<QString>,
}
```

Expose it from `GuiBridge`:

```rust
#[qobject(qml_element)]
pub struct GuiBridge {
    #[qproperty(c_type = "TranscriptModel*", read = get_transcript_model)]
    transcript_model: *mut TranscriptModel,
}
// getter + initialize() that new_cpp_object() + set_parent()
```

Verify QML can `model: bridge.transcript_model ? bridge.transcript_model.words : []`.

**Step B: switch to TranscriptWord gadget**

```rust
#[qgadget]
#[derive(Default, Clone)]
pub struct TranscriptWord { pub text: QString, pub timestamp: f64 }

#[qobject]
#[derive(Default)]
pub struct TranscriptModel { #[qproperty] words: Vec<TranscriptWord> }
```

Rebuild and adjust QML delegate `modelData.text`.

### 8) Dev QoL (optional)

Add a **Justfile**:

```make
set shell := ["bash","-eu","-o","pipefail","-c"]

build:    cargo build -p coldvox-gui -v
run:      cargo run -p coldvox-gui
clean:    cargo clean
tree:     cargo tree | rg 'cxx-qt|Qt6'
```

### 9) Wayland notes

* Global hotkeys: defer. KGlobalAccel alternatives on Wayland are DE-specific. Keep local QML `Shortcut` first.
* Always-on-top is honored by KWin for decorated and frameless windows.

### 10) Troubleshooting

* “unsupported attribute”: you’re not using the git 0.8 macros. Check `Cargo.lock` and `cargo tree`. All `cxx-qt*` must be `git+…#<REV>`.
* Linker errors to Qt: reinstall Qt dev packages; ensure `qmake6 -query QT_VERSION` ≥ 6.6.
* “Only trait declarations…”: move impl bodies outside `#[cxx_qt::bridge]`.
* QML “module not installed”: `qml_uri` and `import` must match (`com.coldvox.app 1.0`), and the bridge must compile.

### 11) Next features after minimal window

* Add overlay flags and translucency.
* Add collapse/expand via `Shortcut { sequence: "Ctrl+Shift+Space" }`.
* Implement fixed-step spiral physics in QML.
* Introduce `SingularityLens.qml` shader and `StarfieldBackground.qml`.

Follow this order. Build after each step. If any error appears, stop and fix the first failing line.

```
