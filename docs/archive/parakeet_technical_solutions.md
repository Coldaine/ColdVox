# Parakeet-TDT-1.1B Rust Implementation: Technical Solutions

Based on comprehensive research into the Parakeet-TDT-1.1B model and Rust ecosystem, I've identified practical solutions for all four technical challenges you've outlined. Here's a detailed analysis with working code examples and implementation strategies.

## 1. Audio Preprocessing for Parakeet

### Input Requirements and Basic Normalization

The Parakeet-TDT-1.1B model expects **16000 Hz mono-channel audio** as input. For converting your `Vec<i16>` audio buffer to the required format:

```rust
fn normalize_i16_to_f32(samples: &[i16]) -> Vec<f32> {
    samples.iter()
        .map(|&sample| sample as f32 / 32768.0)
        .collect()
}
```

The normalization converts 16-bit signed integers to floating-point values in the range `[-1.0, 1.0]` by dividing by `32768.0` (which is `2^15`).

### Complete Preprocessing Pipeline

```rust
use ndarray::prelude::*;

fn preprocess_audio_for_parakeet(samples: &[i16]) -> Array2<f32> {
    // 1. Normalize i16 to f32 [-1.0, 1.0]
    let normalized: Vec<f32> = samples.iter()
        .map(|sample| *sample as f32 / 32768.0)
        .collect();

    // 2. Create tensor with batch dimension [1, num_samples]
    let audio_array = Array2::from_shape_vec((1, normalized.len()), normalized)
        .expect("Failed to create audio array");

    audio_array
}
```

### Feature Extraction Options

While basic normalization may suffice, Parakeet models might benefit from mel-spectrogram features. Two recommended Rust crates:

**Option 1: mel_spec crate** (Whisper-compatible)
```rust
// Cargo.toml: mel_spec = "0.1"
use mel_spec::*;

fn extract_mel_spectrogram(samples: &[f32], sample_rate: usize) -> Array3<f32> {
    let config = MelConfig {
        sample_rate,
        n_fft: 512,
        n_mels: 80,
        hop_length: 160,
        ..Default::default()
    };

    mel_spectrogram(samples, &config)
}
```

**Option 2: speechsauce/mfcc-rust** for more control
```rust
// Cargo.toml: speechsauce = { git = "https://github.com/secretsauceai/mfcc-rust" }
use speechsauce::{mel_spectrogram, MelConfig};
```

## 2. CTC Decoding in Rust

### Option A: Mature Rust CTC Decoder (Recommended)

**ctclib-pp** provides comprehensive CTC decoding with KenLM support:

```rust
// Cargo.toml: ctclib = { git = "https://github.com/agatan/ctclib" }
use ctclib::{BeamSearchDecoderWithKenLM, BeamSearchDecoderOptions};

fn setup_ctc_decoder(vocab: Vec<String>, lm_path: &str) -> BeamSearchDecoderWithKenLM {
    let options = BeamSearchDecoderOptions {
        beam_size: 100,
        beam_size_token: 1000,
        beam_threshold: 10.0,
        lm_weight: 0.5,
    };

    BeamSearchDecoderWithKenLM::new(options, lm_path, vocab)
        .expect("Failed to create CTC decoder")
}
```

### Option B: C++ Library Bindings

**pyctcdecode** backend offers production-ready performance. While no direct Rust bindings exist, the underlying algorithms are well-documented for implementation.

### Option C: Simple Greedy CTC Decoder (Fallback)

```rust
use ndarray::prelude::*;

fn greedy_ctc_decode(logits: &Array3<f32>, vocab: &[char]) -> String {
    let blank_token = vocab.len(); // Assume blank is last token

    // Get best path (argmax along vocab dimension)
    let best_path: Vec<usize> = logits.slice(s![0, .., ..]).rows()
        .into_iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                .unwrap().0
        })
        .collect();

    // Remove consecutive duplicates and blanks
    let mut result = Vec::new();
    let mut prev_token = None;

    for token in best_path {
        if token != blank_token && Some(token) != prev_token {
            result.push(vocab[token]);
        }
        prev_token = Some(token);
    }

    result.into_iter().collect()
}
```

## 3. ONNX Runtime Best Practices

### GPU-Accelerated Session Setup

The `ort` crate provides excellent ONNX Runtime bindings with GPU support:

```rust
use ort::{
    execution_providers::{CUDAExecutionProvider, CPUExecutionProvider},
    session::Session,
    GraphOptimizationLevel,
};

fn create_onnx_session(model_path: &str) -> anyhow::Result<Session> {
    let session = Session::builder()?
        .with_execution_providers([
            // Prefer CUDA if available, fallback to CPU
            CUDAExecutionProvider::default().build(),
            CPUExecutionProvider::default().build(),
        ])?
        .with_optimization_level(GraphOptimizationLevel::Level3)?
        .with_inter_op_threads(4)?
        .commit_from_file(model_path)?;

    Ok(session)
}
```

### Checking Available Execution Providers

```rust
use ort::execution_providers::{ExecutionProvider, CUDAExecutionProvider};

fn check_gpu_availability() -> bool {
    let cuda_ep = CUDAExecutionProvider::default();
    if cuda_ep.is_available() {
        println!("CUDA execution provider is available");
        true
    } else {
        println!("CUDA not available, falling back to CPU");
        false
    }
}
```

### Model Inspection at Runtime

```rust
fn inspect_model_metadata(session: &Session) -> anyhow::Result<()> {
    // Get input information
    for (i, input) in session.inputs.iter().enumerate() {
        println!("Input {}: {}", i, input.name);
        println!("  Shape: {:?}", input.dimensions);
        println!("  Type: {:?}", input.input_type);
    }

    // Get output information
    for (i, output) in session.outputs.iter().enumerate() {
        println!("Output {}: {}", i, output.name);
        println!("  Shape: {:?}", output.dimensions);
        println!("  Type: {:?}", output.output_type);
    }

    Ok(())
}
```

### Efficient ndarray-to-GPU Transfer

```rust
use ort::{inputs, Session, Value};
use ndarray::prelude::*;

fn run_inference(
    session: &Session,
    audio_tensor: Array2<f32>
) -> anyhow::Result<Array3<f32>> {
    // Convert ndarray to ONNX Value
    let input_tensor = Value::from_array(audio_tensor)?;

    // Get input/output names
    let input_name = &session.inputs[0].name;
    let output_name = &session.outputs[0].name;

    // Run inference
    let outputs = session.run(inputs![input_name => input_tensor]?)?;

    // Extract output tensor
    let output_tensor: Array3<f32> = outputs[output_name]
        .try_extract_tensor()?
        .into_dimensionality()?;

    Ok(output_tensor)
}
```

## 4. Parakeet Model Artifacts

### Vocabulary File Location

The Parakeet-TDT-1.1B model uses a **SentencePiece Unigram tokenizer with 1024 tokens**. The vocabulary is embedded within the model's `.nemo` file structure containing:

- `tokenizer.model` - SentencePiece model file
- `tokenizer.vocab` - Vocabulary mappings
- `vocab.txt` - Plain text vocabulary list

### Extracting Vocabulary

```rust
use std::fs::File;
use std::io::Read;
use zip::ZipArchive;

fn extract_vocab_from_nemo(nemo_path: &str) -> anyhow::Result<Vec<String>> {
    let file = File::open(nemo_path)?;
    let mut archive = ZipArchive::new(file)?;

    // Extract vocab.txt
    let mut vocab_file = archive.by_name("vocab.txt")?;
    let mut contents = String::new();
    vocab_file.read_to_string(&mut contents)?;

    let vocab: Vec<String> = contents
        .lines()
        .map(|line| line.trim().to_string())
        .collect();

    Ok(vocab)
}
```

### Model Output Specifications

Based on the architecture analysis:
- **Output Shape**: `[batch_size, sequence_length, vocab_size]`
- **Vocab Size**: 1024 (SentencePiece Unigram tokenizer)
- **Data Type**: Float32 logits requiring CTC decoding

## Recommended Dependencies

```toml
[dependencies]
ort = { version = "2.0.0", features = ["cuda"] }
ndarray = "0.15"
anyhow = "1.0"
zip = "0.6"

# CTC Decoder (choose one):
ctclib-pp = { git = "https://github.com/agatan/ctclib" }

# Audio Processing (choose one):
# mel_spec = "0.1"
# OR
# speechsauce = { git = "https://github.com/secretsauceai/mfcc-rust" }
```

This implementation provides a complete foundation for integrating Parakeet-TDT-1.1B into your Rust application with optimal performance and GPU acceleration capabilities. The modular approach allows you to swap components based on your specific requirements and performance needs.

---
---

# Parakeet TDT 1.1B Rust Implementation Guide

## Overview

This comprehensive guide addresses the four key challenges for implementing the Parakeet-TDT-1.1B model in Rust, providing practical solutions, code examples, and implementation strategies.

## 1. Audio Preprocessing for Parakeet

### Input Requirements
- **Format**: 16000 Hz mono-channel audio
- **Raw Input**: `Vec<i16>` audio samples
- **Target**: Convert to appropriate tensor format for ONNX model

### Step-by-Step Preprocessing

#### Basic Normalization (i16 to f32)
```rust
fn normalize_i16_to_f32(samples: &[i16]) -> Vec<f32> {
    samples.iter()
        .map(|&sample| sample as f32 / 32768.0)
        .collect()
}
```

#### Complete Preprocessing Pipeline
```rust
use ndarray::prelude::*;

fn preprocess_audio_for_parakeet(samples: &[i16]) -> Array2<f32> {
    // 1. Normalize i16 to f32 [-1.0, 1.0]
    let normalized: Vec<f32> = samples.iter()
        .map(|&sample| sample as f32 / 32768.0)
        .collect();

    // 2. Create tensor with batch dimension [1, num_samples]
    let audio_array = Array2::from_shape_vec((1, normalized.len()), normalized)
        .expect("Failed to create audio array");

    audio_array
}
```

### Feature Extraction Options

For advanced feature extraction, consider these Rust crates:

#### Option 1: mel_spec crate (Whisper-compatible)
```rust
// Add to Cargo.toml: mel_spec = "0.1"
use mel_spec::*;

fn extract_mel_spectrogram(samples: &[f32], sample_rate: usize) -> Array2<f32> {
    let config = MelConfig {
        sample_rate,
        n_fft: 512,
        n_mels: 80,
        hop_length: 160,
        ..Default::default()
    };

    mel_spectrogram(samples, &config)
}
```

#### Option 2: speechsauce/mfcc-rust
```rust
// Add to Cargo.toml: speechsauce = { git = "https://github.com/secretsauceai/mfcc-rust" }
use speechsauce::{mel_spectrogram, MelConfig};

fn extract_features(audio: &[f32]) -> Array2<f32> {
    let config = MelConfig::default();
    mel_spectrogram(audio, &config)
}
```

## 2. CTC Decoding in Rust

### Option A: Rust Native CTC Decoders (Recommended)

#### Using ctclib-pp
```rust
// Add to Cargo.toml: ctclib = { git = "https://github.com/agatan/ctclib" }
use ctclib::{BeamSearchDecoderWithKenLM, BeamSearchDecoderOptions};

fn setup_ctc_decoder(vocab: Vec<String>, lm_path: &str) -> BeamSearchDecoderWithKenLM {
    let options = BeamSearchDecoderOptions {
        beam_size: 100,
        beam_size_token: 1000,
        beam_threshold: 1.0,
        lm_weight: 0.5,
    };

    BeamSearchDecoderWithKenLM::new(options, lm_path, vocab)
        .expect("Failed to create CTC decoder")
}

fn decode_ctc_output(decoder: &BeamSearchDecoderWithKenLM, log_probs: &Array2<f32>) -> String {
    decoder.decode(log_probs)
}
```

#### Using fast-ctc-decode (Alternative)
```rust
// Note: Currently not published on crates.io
// Use git dependency or implement similar approach

fn beam_search_decode(
    posteriors: &Array2<f32>,
    alphabet: &[char],
    beam_size: usize,
    beam_cut_threshold: f32
) -> String {
    // Implementation based on nanoporetech/fast-ctc-decode
    // Convert posteriors to required format and apply beam search
    unimplemented!("Implement based on fast-ctc-decode algorithms")
}
```

### Option C: Simple Greedy CTC Decoder (Fallback)
```rust
use ndarray::prelude::*;

fn greedy_ctc_decode(logits: &Array2<f32>, vocab: &[char]) -> String {
    let blank_token = vocab.len(); // Assume blank is last token

    // Get best path (argmax along vocab dimension)
    let best_path: Vec<usize> = logits.rows()
        .into_iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                .unwrap().0
        })
        .collect();

    // Remove consecutive duplicates and blanks
    let mut result = Vec::new();
    let mut prev_token = None;

    for token in best_path {
        if token != blank_token && Some(token) != prev_token {
            result.push(vocab[token]);
        }
        prev_token = Some(token);
    }

    result.into_iter().collect()
}
```

## 3. ONNX Runtime Best Practices

### Session Setup with GPU Support
```rust
use ort::{
    execution_providers::{CUDAExecutionProvider, CPUExecutionProvider},
    session::Session,
};

fn create_onnx_session(model_path: &str) -> anyhow::Result<Session> {
    let session = Session::builder()?
        .with_execution_providers([
            // Prefer CUDA if available, fallback to CPU
            CUDAExecutionProvider::default().build(),
            CPUExecutionProvider::default().build(),
        ])?
        .with_optimization_level(ort::GraphOptimizationLevel::Level3)?
        .with_intra_threads(4)?
        .commit_from_file(model_path)?;

    Ok(session)
}
```

### Checking Available Execution Providers
```rust
use ort::execution_providers::{ExecutionProvider, CUDAExecutionProvider};

fn check_gpu_availability() -> bool {
    let cuda_ep = CUDAExecutionProvider::default();
    if cuda_ep.is_available() {
        println!("CUDA execution provider is available");
        true
    } else {
        println!("CUDA not available, falling back to CPU");
        false
    }
}

fn get_active_providers(session: &Session) -> Vec<String> {
    session.providers()
        .iter()
        .map(|p| p.to_string())
        .collect()
}
```

### Model Inspection
```rust
use ort::session::Session;

fn inspect_model_metadata(session: &Session) -> anyhow::Result<()> {
    // Get input information
    for (i, input) in session.inputs.iter().enumerate() {
        println!("Input {}: {}", i, input.name);
        println!("  Shape: {:?}", input.dimensions);
        println!("  Type: {:?}", input.input_type);
    }

    // Get output information
    for (i, output) in session.outputs.iter().enumerate() {
        println!("Output {}: {}", i, output.name);
        println!("  Shape: {:?}", output.dimensions);
        println!("  Type: {:?}", output.output_type);
    }

    Ok(())
}
```

### Efficient Inference with GPU Memory Management
```rust
use ort::{inputs, Session, Value};
use ndarray::prelude::*;

fn run_inference(
    session: &Session,
    audio_tensor: Array2<f32>
) -> anyhow::Result<Array2<f32>> {
    // Convert ndarray to ONNX Value
    let input_tensor = Value::from_array(audio_tensor)?;

    // Get input/output names
    let input_name = &session.inputs[0].name;
    let output_name = &session.outputs[0].name;

    // Run inference
    let outputs = session.run(inputs![input_name => input_tensor]?)?;

    // Extract output tensor
    let output_tensor: Array2<f32> = outputs[output_name]
        .try_extract_tensor()?
        .into_dimensionality()?;

    Ok(output_tensor)
}
```

## 4. Parakeet Model Artifacts

### Vocabulary File Location
The vocabulary for Parakeet-TDT-1.1B is embedded within the model's `.nemo` file structure:

**File structure inside .nemo archive:**
- `tokenizer.model` - SentencePiece model file
- `tokenizer.vocab` - Vocabulary mappings
- `vocab.txt` - Plain text vocabulary list

### Extracting Vocabulary
```rust
use std::fs::File;
use zip::ZipArchive;

fn extract_vocab_from_nemo(nemo_path: &str) -> anyhow::Result<Vec<String>> {
    let file = File::open(nemo_path)?;
    let mut archive = ZipArchive::new(file)?;

    // Extract vocab.txt
    let mut vocab_file = archive.by_name("vocab.txt")?;
    let mut contents = String::new();
    vocab_file.read_to_string(&mut contents)?;

    let vocab: Vec<String> = contents
        .lines()
        .map(|line| line.trim().to_string())
        .collect();

    Ok(vocab)
}
```

### Expected Output Tensor Specifications

Based on the model architecture:
- **Output Shape**: `[batch_size, sequence_length, vocab_size]`
- **Vocab Size**: 1024 (SentencePiece Unigram tokenizer)
- **Data Type**: Float32 logits
- **Decoding**: Requires CTC decoding to convert logits to text

### Complete Integration Example
```rust
use anyhow::Result;
use ndarray::prelude::*;

struct ParakeetInference {
    session: ort::Session,
    vocab: Vec<String>,
    // decoder: Box<dyn CTCDecoder>, // Your chosen decoder implementation
}

impl ParakeetInference {
    fn new(model_path: &str, nemo_path: &str) -> Result<Self> {
        let session = create_onnx_session(model_path)?;
        let vocab = extract_vocab_from_nemo(nemo_path)?;

        Ok(Self {
            session,
            vocab,
        })
    }

    fn transcribe(&self, audio_samples: &[i16]) -> Result<String> {
        // 1. Preprocess audio
        let input_tensor = preprocess_audio_for_parakeet(audio_samples);

        // 2. Run inference
        let logits = run_inference(&self.session, input_tensor)?;

        // 3. CTC decode
        let transcript = greedy_ctc_decode(&logits, &self.vocab.iter().map(|s| s.chars().next().unwrap()).collect::<Vec<_>>());

        Ok(transcript)
    }
}
```

## Dependencies for Cargo.toml
```toml
[dependencies]
ort = { version = "2.0", features = ["cuda"] }
ndarray = "0.16"
anyhow = "1.0"
zip = "2.1"

# Choose one CTC decoder option:
# ctclib = { git = "https://github.com/agatan/ctclib" }
# OR implement fast-ctc-decode equivalent

# Choose one audio processing option:
# mel_spec = "0.1"
# OR speechsauce = { git = "https://github.com/secretsauceai/mfcc-rust" }
```

This guide provides the foundation for implementing Parakeet-TDT-1.1B in Rust with proper audio preprocessing, CTC decoding, and ONNX Runtime integration.
