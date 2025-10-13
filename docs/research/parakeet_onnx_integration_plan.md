# Project Integration Plan: Parakeet ONNX STT Plugin

This plan outlines the steps to create a new STT plugin for ColdVox that uses a GPU-accelerated Parakeet ASR model via the ONNX Runtime.

#### 1. Create a New Crate for the Plugin

A self-contained crate is the best approach to keep dependencies clean and encapsulate the new functionality.

*   **Action:** Create a new crate within the `crates/` directory named `coldvox-stt-parakeet-onnx`.
*   **Command:** `cargo new --lib crates/coldvox-stt-parakeet-onnx`
*   **Workspace:** Add the new crate to the `[workspace.members]` section of the root `Cargo.toml`.

#### 2. Add Dependencies

The new crate will require the `ort` crate for ONNX Runtime access, along with other standard libraries for async operations and error handling.

*   **Action:** Add the following dependencies to `crates/coldvox-stt-parakeet-onnx/Cargo.toml`:

```toml
[dependencies]
coldvox-stt = { path = "../coldvox-stt" }
ort = { version = "2.0.0", features = ["cuda"] }
ndarray = "0.15"
async-trait = "0.1"
tokio = { version = "1", features = ["sync"] }
thiserror = "1.0"
tracing = "0.1"

# Add other dependencies as needed for audio preprocessing or CTC decoding
```

*   **Note on `ort`:** The `cuda` feature will enable the CUDA execution provider. The `ort` crate will attempt to download pre-built ONNX Runtime binaries with CUDA support during the build process. This requires a compatible NVIDIA GPU and the CUDA Toolkit to be installed on the system.

#### 3. Implement the `StreamingStt` Trait

The most modern and flexible interface in your project is the `StreamingStt` trait. The new plugin will implement this trait.

*   **Action:** Create a `ParakeetTranscriber` struct in `crates/coldvox-stt-parakeet-onnx/src/lib.rs` that will hold the ONNX session and any required state (like an audio buffer).

```rust
// In crates/coldvox-stt-parakeet-onnx/src/lib.rs

use coldvox_stt::{StreamingStt, TranscriptionEvent};
use ort::{Environment, Session, SessionBuilder};
use std::sync::Arc;

pub struct ParakeetTranscriber {
    session: Session,
    // A buffer to hold audio samples for a complete utterance
    audio_buffer: Vec<i16>,
    // Add other state, like a vocabulary for decoding
}

impl ParakeetTranscriber {
    pub fn new() -> Result<Self, anyhow::Error> {
        let environment = Arc::new(Environment::builder().with_name("parakeet").build()?);

        // The model path would be loaded from a config file
        let model_path = "path/to/parakeet.onnx";

        let session = SessionBuilder::new(&environment)?
            .with_cuda_ep(Default::default())? // Enable CUDA
            .with_model_from_file(model_path)?;

        Ok(Self {
            session,
            audio_buffer: Vec::new(),
        })
    }
}

#[async_trait::async_trait]
impl StreamingStt for ParakeetTranscriber {
    async fn on_speech_frame(&mut self, samples: &[i16]) -> Option<TranscriptionEvent> {
        // Append incoming audio to the buffer
        self.audio_buffer.extend_from_slice(samples);
        // For streaming, you might perform partial transcription here if the model supports it.
        // For simplicity, we'll do a full transcription on speech_end.
        None
    }

    async fn on_speech_end(&mut self) -> Option<TranscriptionEvent> {
        // 1. Preprocess the audio buffer (e.g., normalize f32, create tensor)
        // 2. Run inference with self.session.run()
        // 3. Decode the output tensor (logits) into text using a CTC decoder
        // 4. Clear the audio buffer
        // 5. Return a TranscriptionEvent::Final
        todo!()
    }

    async fn reset(&mut self) {
        self.audio_buffer.clear();
    }
}
```

#### 4. Implement the Plugin Factory

To make the plugin discoverable by the `SttPluginManager`, you need to create a factory.

*   **Action:** Implement `SttPlugin` and `SttPluginFactory` to wrap the `ParakeetTranscriber`.

```rust
// In crates/coldvox-stt-parakeet-onnx/src/lib.rs (continued)

use coldvox_stt::plugin::{SttPlugin, SttPluginFactory, PluginInfo, SttPluginError};

pub struct ParakeetPlugin {
    transcriber: Option<ParakeetTranscriber>,
}

// ... implement SttPlugin for ParakeetPlugin, delegating to the transcriber ...

pub struct ParakeetPluginFactory;

impl SttPluginFactory for ParakeetPluginFactory {
    fn create(&self) -> Result<Box<dyn SttPlugin>, SttPluginError> {
        // The creation logic will instantiate the ParakeetPlugin
        todo!()
    }

    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: "parakeet-onnx".to_string(),
            name: "Parakeet ONNX".to_string(),
            description: "GPU-accelerated Parakeet ASR via ONNX Runtime.".to_string(),
            is_available: true, // Could check for GPU/model file here
        }
    }
}
```

#### 5. Handle Audio Preprocessing and Inference

The Parakeet model will expect audio in a specific format (likely a normalized `f32` tensor).

*   **Action:** In the `on_speech_end` method:
    1.  Convert the `audio_buffer` of `i16` samples to a `Vec<f32>` by dividing by `32768.0`.
    2.  Create an `ndarray::Array` from the `Vec<f32>` and shape it according to the model's input specification (e.g., `[1, num_samples]`).
    3.  Use `ort::Value::from_array()` to create an input tensor.
    4.  Run inference: `self.session.run(vec![input_tensor])?`.

#### 6. Decode the Output

The model will output logits, which need to be converted to text. This is typically done with a **CTC (Connectionist Temporal Classification) decoder**.

*   **Action:**
    1.  **Research CTC Decoder Crates:** My web search indicates that pure Rust CTC decoders are not mature. The most viable option is to find a crate with bindings to a C++ implementation like `flashlight` (formerly `wav2letter`) or the one used in `pycwtcdecode`. If none are suitable, a simple **greedy CTC decoder** can be implemented as a starting point.
    2.  **Greedy Decoding (Initial Implementation):**
        *   For each time step in the output tensor, find the index of the highest probability.
        *   Map this index to a character in the model's vocabulary.
        *   Remove duplicate consecutive characters.
        *   Remove blank characters.
    3.  **Vocabulary:** The model will have a vocabulary file that maps the output indices to characters. This file must be loaded and stored in the `ParakeetTranscriber`.

#### 7. Register the Plugin

Finally, make the application aware of the new plugin.

*   **Action:** In `crates/app/src/stt/plugin_manager.rs`, in the `register_builtin_plugins` function, add a new block to register the `ParakeetPluginFactory`. This will likely be behind a new feature flag, e.g., `parakeet-onnx`.

```rust
// In crates/app/src/stt/plugin_manager.rs

// ...
#[cfg(feature = "parakeet-onnx")]
{
    use coldvox_stt_parakeet_onnx::ParakeetPluginFactory;
    registry.register(Box::new(ParakeetPluginFactory));
}
```

*   **Action:** Add the new feature to the `coldvox-app` crate's `Cargo.toml`.

---

### **Comprehensive Research Prompt for AI Agent**

**Objective:** To gather the necessary technical details, code snippets, and strategies to overcome the key challenges in integrating the Parakeet ONNX ASR model into a Rust application.

**Key Challenges & Research Questions:**

**1. Audio Preprocessing for Parakeet:**
   - The application provides raw audio as `Vec<i16>` at 16kHz. The Parakeet ONNX model likely requires a specific input format.
   - **Question:** What are the precise, step-by-step preprocessing requirements for the Parakeet-TDT-v1.1 model?
   - **Deliverable:** Provide a Rust code snippet or detailed pseudocode demonstrating how to transform a `&[i16]` audio buffer into the exact `ndarray::Array` format expected by the model's ONNX input tensor. This should cover:
     - Normalization (e.g., converting `i16` to `f32` in the range `[-1.0, 1.0]`).
     - Any required feature extraction (e.g., Mel spectrogram, MFCC). If so, recommend a Rust crate and provide a code example.
     - The final tensor shape (e.g., `[1, num_samples]` or `[1, num_frames, num_features]`).

**2. CTC Decoding in Rust:**
   - The model's output will be a tensor of logits that requires CTC decoding to produce a text transcription. Pure Rust solutions for this are not readily apparent.
   - **Question:** What is the most robust and performant strategy for CTC decoding in a Rust environment?
   - **Deliverable:**
     - **Option A (Best):** Identify a mature Rust crate for CTC beam search decoding (ideally with KenLM language model support). Provide its name and a usage example.
     - **Option B (Good):** If no mature Rust crate exists, identify a high-performance C++ library (e.g., Flashlight, pyctcdecode's backend) and determine if Rust bindings (`-sys` crate) are available. If so, provide the crate name and a basic FFI usage pattern.
     - **Option C (Fallback):** If neither of the above is feasible, provide a clear, correct, and simple Rust implementation of a **greedy CTC decoder**. This function should take the logits tensor (`ndarray::Array`) and a vocabulary (`Vec<char>`) as input and return the decoded `String`.

**3. ONNX Runtime (`ort` crate) Best Practices:**
   - Ensuring the GPU is used correctly and efficiently is critical.
   - **Question:** How can I effectively use the `ort` crate for GPU-accelerated inference?
   - **Deliverable:** Provide Rust code snippets for the following:
     - How to programmatically check which execution providers (e.g., "CudaExecutionProvider", "CPUExecutionProvider") are available and which one the session is *actually* using.
     - How to inspect the expected input/output names, shapes, and data types of an ONNX model at runtime.
     - The most efficient way to transfer `ndarray` data to the GPU and retrieve the results.

**4. Parakeet Model Artifacts:**
   - The model requires a vocabulary file to map output indices to characters.
   - **Question:** Where can the official vocabulary file for the `nvidia/parakeet-tdt-1.1b` model be found?
   - **Deliverable:** Provide a direct link to the vocabulary file or the Hugging Face repository page where it can be downloaded. Also, specify the exact output shape of the model's logits tensor.
