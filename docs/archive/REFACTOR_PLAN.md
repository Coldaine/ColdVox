<!-- Archived from crates/app/REFACTOR_PLAN.md on 2025-08-26 -->
# Audio Pipeline Refactoring and STT Integration Plan

This document outlines a detailed plan to refactor the ColdVox audio pipeline. The goals are to fix architectural issues, improve robustness and efficiency, and—most importantly—integrate the Speech-to-Text (STT) engine to create a complete, end-to-end voice processing application.

## Part 1: STT Integration (High Priority)

### 1.1. The Problem: Disconnected STT Engine

The VAD correctly identifies speech and silence, generating events. However, these events are only logged. The `VoskTranscriber` is implemented but sits idle, completely disconnected from the live audio stream. The primary goal of the application is unfulfilled.

### 1.2. The Solution: A Decoupled, Event-Driven Architecture

We will introduce a new `SttProcessor` task and use a `tokio::sync::broadcast` channel to allow multiple components to consume the audio stream concurrently.

The new data flow will be:

1.  `AudioChunker`: Reads from the ring buffer and **broadcasts** `AudioFrame`s.
2.  `VadProcessor`: Subscribes to the audio broadcast, processes frames, and produces `VadEvent`s on a standard `mpsc` channel.
3.  `SttProcessor`: A new component that subscribes to **both** the audio broadcast and the VAD event channel. It uses VAD events to gate the transcription of the audio stream.

![New Architecture Diagram](https://i.imgur.com/9A7X6Zt.png)

### 1.3. Implementation Steps

#### 1. Create the `SttProcessor`

Create a new file: `src/stt/processor.rs`.

```rust
// src/stt/processor.rs

use tokio::sync::{broadcast, mpsc};
use crate::audio::vad_processor::AudioFrame;
use crate::stt::vosk::VoskTranscriber;
use crate::vad::types::VadEvent;

pub struct SttProcessor {
	audio_rx: broadcast::Receiver<AudioFrame>,
	vad_event_rx: mpsc::Receiver<VadEvent>,
	transcriber: VoskTranscriber,
	is_speaking: bool,
}

impl SttProcessor {
	pub fn new(
		audio_rx: broadcast::Receiver<AudioFrame>,
		vad_event_rx: mpsc::Receiver<VadEvent>,
	) -> Result<Self, String> {
		// In the future, model_path and sample_rate will come from config
		let transcriber = VoskTranscriber::new("vosk-model", 16000.0)?;
		Ok(Self {
			audio_rx,
			vad_event_rx,
			transcriber,
			is_speaking: false,
		})
	}

	pub async fn run(mut self) {
		loop {
			tokio::select! {
				// Listen for a VAD event
				Some(event) = self.vad_event_rx.recv() => {
					match event {
						VadEvent::SpeechStart { .. } => {
							tracing::debug!("STT processor received SpeechStart");
							self.is_speaking = true;
						}
						VadEvent::SpeechEnd { .. } => {
							tracing::debug!("STT processor received SpeechEnd");
							self.is_speaking = false;
							if let Ok(Some(text)) = self.transcriber.finalize() {
								tracing::info!(target: "stt", "Final transcription: {}", text);
							}
						}
					}
				}

				// Listen for an audio frame
				Ok(frame) = self.audio_rx.recv() => {
					if self.is_speaking {
						if let Ok(Some(partial_text)) = self.transcriber.accept_pcm16(&frame.data) {
							 tracing::info!(target: "stt", "Partial: {}", partial_text);
						}
					}
				}
			}
		}
	}
}
```

Make the new module public in `src/stt/mod.rs`:
```rust
// src/stt/mod.rs
pub mod processor;
pub mod vosk;
// ...
```

#### 2. Update `main.rs` to Wire Everything Together

1.  **Change Channel to Broadcast:** The channel between the chunker and its consumers will now be a `broadcast` channel.

	```rust
	// In main.rs
	// OLD: let (vad_in_tx, vad_in_rx) = mpsc::channel(...);
	let (audio_tx, _) = broadcast::channel::<coldvox_app::audio::vad_processor::AudioFrame>(200);
	```

2.  **Update Chunker:** Give the `broadcast::Sender` to the `AudioChunker`.

	```rust
	// In main.rs
	let chunker = AudioChunker::new(frame_reader, audio_tx.clone(), chunker_cfg);
	```

3.  **Update VAD Processor:** The `VadProcessor` will subscribe to the broadcast channel.

	```rust
	// In main.rs
	let vad_audio_rx = audio_tx.subscribe();
	let vad_thread = coldvox_app::audio::vad_processor::VadProcessor::spawn(
		vad_cfg,
		vad_audio_rx, // <-- Use the new receiver
		event_tx,
		// ...
	);
	```

4.  **Instantiate and Spawn `SttProcessor`:**

	```rust
	// In main.rs, after spawning VAD
	use coldvox_app::stt::processor::SttProcessor;

	let stt_audio_rx = audio_tx.subscribe();
	let stt_processor = SttProcessor::new(stt_audio_rx, event_rx)
		.expect("Failed to create STT processor");
	let stt_handle = tokio::spawn(stt_processor.run());

	// The old event_logger task is no longer needed and can be removed.
	```

5.  **Update Shutdown Logic:** Ensure the `stt_handle` is awaited during shutdown.

## Part 2: Architectural Improvements (Medium Priority)

### 2.1. Fix Sample Rate Brittleness

-   **Problem:** `main.rs` hardcodes the `16000` sample rate for downstream components, but `AudioCapture` might negotiate a different rate.
-   **Solution:** Make the `AudioCapture` module the source of truth for the sample rate.
-   **Implementation:**
	1.  In `audio/capture.rs`, modify `AudioCaptureThread::spawn` to return the effective output sample rate. The resampler always outputs 16kHz, so we can hardcode that for now, but this makes the architecture ready for dynamic rates.
		```rust
		// In audio/capture.rs
		impl AudioCaptureThread {
			pub fn spawn(...) -> Result<(Self, u32), AudioError> {
				// ... spawn logic ...
				let output_sample_rate = 16_000; // The resampler's target rate
				Ok((Self { handle, shutdown }, output_sample_rate))
			}
		}
		```
	2.  In `main.rs`, use the returned value.
		```rust
		// In main.rs
		let (audio_capture, sample_rate) = AudioCaptureThread::spawn(...)
			.expect("Failed to start audio capture");

		let frame_reader = FrameReader::new(audio_consumer, sample_rate);
		let chunker_cfg = ChunkerConfig { frame_size_samples: 512, sample_rate_hz: sample_rate };
		```

### 2.2. Remove Inefficient Async Polling

-   **Problem:** `AudioChunker` and `VadProcessor` use `time::sleep`, which is inefficient for async tasks.
-   **Solution:** Use proper async blocking and signaling.
-   **Implementation:**
	1.  **`VadProcessor`:** This is an easy fix. Remove the `sleep` from the `tokio::select!`. The `.recv()` call on the channel is already an efficient, non-blocking wait.
		```rust
		// In audio/vad_processor.rs, inside run()
		// REMOVE the select! and just use a while let loop
		while let Some(frame) = self.audio_rx.recv().await {
			self.process_frame(frame).await;
		}
		```
	2.  **`AudioChunker`:** The `rtrb` buffer is not async, so polling is required. We will keep the `sleep` for now, as implementing a `Notify`-based mechanism is complex and provides diminishing returns for this use case. The current `sleep(1)` is acceptable.

### 2.3. Simplify Shutdown Logic

-   **Problem:** Shutdown mixes `abort()` with redundant `AtomicBool` flags.
-   **Solution:** Rely on channel closing to gracefully terminate tasks.
-   **Implementation:**
	1.  In `audio/vad_processor.rs`, remove the `shutdown: Arc<AtomicBool>` field and all associated logic. The `while let Some(...)` loop handles shutdown automatically when the channel sender is dropped.
	2.  In `main.rs`, update the `VadProcessor::spawn` call to remove the shutdown argument.
	3.  The shutdown sequence in `main.rs` becomes:
		```rust
		// In main.rs shutdown block
		tracing::info!("Beginning graceful shutdown");
        
		// 1. Stop the source thread
		audio_capture.stop();
        
		// 2. Abort the tasks. This will drop their channel senders,
		//    causing downstream tasks to terminate gracefully.
		chunker_handle.abort();
		vad_thread.abort();
		stt_handle.abort();

		// 3. Await all handles to ensure clean exit
		let _ = tokio::try_join!(chunker_handle, vad_thread, stt_handle);

		tracing::info!("Shutdown complete");
		```

## Part 3: Housekeeping and Future Work (Low Priority)

These items should be addressed after the main refactoring is complete.

-   **3.1. Configuration from File:**
	-   **Action:** Introduce the `config` crate. Create a `src/settings.rs` module.
	-   **Action:** Define a `Settings` struct that can be deserialized from a `config.toml` file.
	-   **Action:** Load settings in `main.rs` and pass them down to components instead of using hardcoded values.

-   **3.2. Stats Reporting:**
	-   **Action:** Modify `AudioCapture::new` to return an `Arc<CaptureStats>`.
	-   **Action:** Pass this `Arc` to a stats-reporting task in `main.rs` that periodically logs the values.

-   **3.3. Mutex on Audio Hot Path:**
	-   **Action:** No immediate action required. Acknowledge this as a minor performance risk. If audio glitches or high CPU usage are observed, this should be investigated with profiling tools. Replacing the `Mutex` with a lock-free alternative is a significant task to be undertaken only if necessary.

