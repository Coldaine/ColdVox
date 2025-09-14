// This file follows the canonical CXX-Qt structure for defining multiple related types.
#[cxx_qt::bridge(qml_uri = "com.coldvox.app", qml_version = "1.0")]
pub mod ffi {
    // 1. Declare any C++ types from Qt that will be used in the bridge.
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qvariant.h");
        type QVariant = cxx_qt_lib::QVariant;
        include!("cxx-qt-lib/qvariantlist.h");
        type QVariantList = cxx_qt_lib::QVariantList;
    }

    // 2. Define any QGadget or freestanding QObject structs.
    // These must be defined before they are used as properties.
    #[cxx_qt::qgadget]
    #[derive(Default, Clone)]
    pub struct TranscriptWord {
        pub text: QString,
        pub progress: f32,
        pub opacity: f32,
        pub is_current: bool,
        pub font_size: f32,
        pub font_weight: i32,
    }

    #[cxx_qt::qobject]
    #[derive(Default)]
    pub struct TranscriptModel {
        #[qproperty]
        words: QVariantList,
    }

    // 3. Define the main QObject that is implemented in Rust.
    // This MUST be inside an `unsafe extern "RustQt"` block.
    // There can only be one such block per file.
    unsafe extern "RustQt" {
        #[cxx_qt::qobject(rust_type = super::GuiBridgeRust, qobject_ident = GuiBridge)]
        #[qproperty(super::ffi::TranscriptModel*, transcript_model, read_only)]
        #[qproperty(super::ffi::AppState, state)]
        ()
    }

    // 4. Define any enums that are associated with the main QObject.
    // This refers to `GuiBridge` which is defined in the block above.
    // The declaration order is critical.
    #[qenum(GuiBridge)]
    pub enum AppState {
        Idle,
        Recording,
        Processing,
    }
}

// 5. All Rust implementation logic goes outside the bridge module.
use ffi::{AppState, GuiBridge, TranscriptModel, TranscriptWord};

impl Default for AppState {
    fn default() -> Self {
        Self::Idle
    }
}

// The backing struct for our main QObject.
pub struct GuiBridgeRust {
    state: AppState,
    transcript_model: cxx_qt::QObject<TranscriptModel>,
}

// The implementation for the main QObject's methods.
impl GuiBridge {
    pub fn state(&self) -> AppState { self.rust().state }
    pub fn set_state(&mut self, state: AppState) { self.rust_mut().state = state; }

    pub fn transcript_model(&self) -> &cxx_qt::QObject<TranscriptModel> {
        &self.rust().transcript_model
    }

    #[qinvokable]
    pub fn initialize(self: core::pin::Pin<&mut Self>) {
        let model = TranscriptModel::new();
        self.rust_mut().transcript_model = model;
    }

    #[qinvokable]
    pub fn add_mock_word(self: core::pin::Pin<&mut Self>) {
        if let Some(model) = self.rust().transcript_model.as_ref() {
            let word = TranscriptWord {
                text: cxx_qt_lib::QString::from("coldvox"),
                progress: 0.0,
                opacity: 1.0,
                is_current: false,
                font_size: 16.0,
                font_weight: 400,
            };
            let mut list = model.get_words();
            list.append(cxx_qt::QVariant::from_qgadget(&word));
            model.set_words(list);
        }
    }
}

// Custom constructor for the backing struct.
impl Default for GuiBridgeRust {
    fn default() -> Self {
        Self {
            state: AppState::default(),
            transcript_model: cxx_qt::QObject::default(),
        }
    }
}
