One-shot Agent Prompt: Activate Vosk Plugin in ColdVox

Context
------
Repository: ColdVox (multi-crate Rust workspace)
Goal: Implement and activate the Vosk STT plugin so the existing plugin registry can discover, create, and use Vosk via the `vosk` feature flag.

This agent will run in a single shot and has the entire repository source in its context window. Do not include implementation code in this prompt; implement directly in the repository following the steps below.

High-level outcome
------------------
- Add a Vosk plugin factory + plugin implementation that adapts the existing `VoskTranscriber` to the `SttPlugin` plugin interface.
- Export the factory from `crates/coldvox-stt/src/plugins/mod.rs` under the `vosk` feature.
- Replace the TODO registration block in `crates/app/src/stt/plugin_manager.rs` with a concrete registration of the factory (matching the style of other factories already registered).
- Ensure `cargo check --workspace --features vosk` and `cargo run --features vosk` succeed and the plugin is discoverable and instantiable via the plugin registry.

Files of interest (you have them open in your context)
- `crates/app/src/stt/plugin_manager.rs` — plugin registry and TODO registration block.
- `crates/coldvox-stt/src/plugins/mod.rs` — currently contains `#[cfg(feature = "vosk")] pub mod vosk;` but missing re-exports.
- `crates/coldvox-stt/src/plugins/noop.rs` and `crates/coldvox-stt/src/plugins/mock.rs` — use these as patterns for factory shape and registration style.
- `crates/coldvox-stt-vosk/src/vosk_transcriber.rs` — existing VoskTranscriber (implements EventBasedTranscriber + Transcriber traits). Use this internally.

Step-by-step implementation plan (one-shot)
-----------------------------------------
1) Locate and inspect the existing plugin factory examples in `crates/coldvox-stt/src/plugins/` (e.g., `noop.rs`, `mock.rs`, `whisper_plugin.rs`) and the plugin trait definitions in `crates/coldvox-stt/src/plugin` to copy the expected factory and plugin implementation patterns. Follow the exact public types and method names those factories use so the registry will accept the new factory without signature mismatches.

2) Add a new file `crates/coldvox-stt/src/plugins/vosk.rs` implementing:
   - A `VoskPluginFactory` type that matches the other factories' API (constructor style, methods, static metadata if present).
   - A `VoskPlugin` type that implements the `SttPlugin` trait expected by the plugin system. Internals should delegate to `coldvox_stt_vosk::VoskTranscriber`:
     - On load/initialize: create `VoskTranscriber::new(config, sample_rate)` or appropriate constructor using `TranscriptionConfig` from the plugin config.
     - On audio processing: forward PCM frames to the transcriber (`accept_frame` / `accept_pcm16`) and convert `TranscriptionEvent` results into whatever the `SttPlugin` contract requires (e.g., emit events, return strings, or push to channels per existing patterns).
     - Implement `unload()` to drop the transcriber and free resources.
   - Ensure the plugin reports correct `info()` metadata (id: "vosk", name: "Vosk", description) so registry logging and selection logic can see it.

   Implementation notes:
   - Match the error types and conversion utilities used in other plugins. Inspect `noop.rs` and `mock.rs` to mirror `Factory` and `Plugin` shape.
   - If factories in this repo are zero-sized types registered directly (e.g., `registry.register(Box::new(NoOpPluginFactory));`), implement `VoskPluginFactory` the same way (provide `const`/`fn new()` if necessary to match the pattern used by other factories).

3) Update `crates/coldvox-stt/src/plugins/mod.rs` (re-exports)
   - Under the existing `#[cfg(feature = "vosk")] pub mod vosk;` add:
     ```rust
     #[cfg(feature = "vosk")]
     pub use vosk::{VoskPlugin, VoskPluginFactory};
     ```
   - This keeps the repository's plugin layout consistent and allows other crates to `use coldvox_stt::plugins::vosk::VoskPluginFactory`.

4) Patch `crates/app/src/stt/plugin_manager.rs` registration block
   - Replace the existing TODO block in `register_builtin_plugins()` with the same registration style as other factories. Example (match the style used elsewhere):
     - If other factories are registered with `registry.register(Box::new(XxxPluginFactory::new()));` then register `VoskPluginFactory::new()`.
     - If other factories are registered with a value directly like `registry.register(Box::new(NoOpPluginFactory));` then register the new factory the same way.
   - Keep the `#[cfg(feature = "vosk")]` guard.

5) Build and test
   - From repo root run:
     ```sh
     cd crates/app
     cargo check --features vosk
     cargo build --features vosk
     cargo run --features vosk -- --log-level "info,stt=debug"
     ```
   - Acceptance criteria:
     - Build succeeds (`cargo check`/`cargo build`).
     - On run, logs show plugin discovery and a `vosk` plugin info entry.
     - `registry.create_plugin("vosk")` (triggered by manager initialization) succeeds and returns a usable plugin instance (no NotAvailable errors).
     - A small transcription smoke-test: use `crates/app/test_audio_16k.wav` or `recording_16khz_10s_1756081007.wav` to exercise the pipeline (there is an existing test harness in the app; if available, run that e2e test). The plugin should return or emit transcription events for speech segments.

6) If tests fail, collect logs and revert gracefully
   - Revert changes to `plugin_manager.rs` if registration causes runtime errors while factory is incomplete.
   - Suggested rollback commands:
     ```sh
     git checkout -- crates/app/src/stt/plugin_manager.rs
     git checkout -- crates/coldvox-stt/src/plugins/mod.rs
     ```

7) Commit and push (if everything passes)
   - Create a focused commit with message: "stt: add VoskPlugin + factory and register under vosk feature"
   - Example:
     ```sh
     git add crates/coldvox-stt/src/plugins/vosk.rs crates/coldvox-stt/src/plugins/mod.rs crates/app/src/stt/plugin_manager.rs
     git commit -m "stt: add VoskPlugin + factory and register under vosk feature"
     git push origin HEAD
     ```

Extra guidance (telemetry/types note — optional follow-up)
---------------------------------------------------
- The workspace currently has `coldvox-telemetry` importing `InjectionMetrics` from `coldvox-text-injection` behind a `text-injection` feature. If you plan to extract telemetry types into `crates/coldvox-telemetry-types`, do that as a separate PR after the Vosk activation work. Audit `crates/coldvox-text-injection/src/types.rs` and move only the serializable data structures, leaving complex helper methods in the source crate or providing thin adapter functions.

What to log / return at the end of this one-shot
------------------------------------------------
Return a short JSON object (printed to stdout) with these fields:
- `status`: "ok" or "error"
- `errors`: list of error strings if any
- `built`: boolean if `cargo build --features vosk` succeeded
- `plugin_discovered`: boolean if plugin registry reports `vosk` available
- `plugin_created`: boolean if registry successfully created the `vosk` plugin
- `smoke_test_passed`: boolean if the test audio produced at least one transcription event

Important execution notes for the agent
-------------------------------------
- Use the repository's existing patterns: mirror other plugin factories and plugins for style and signatures.
- Do not change unrelated files or reformat the entire workspace. Keep edits minimal and focused.
- If you need to inspect small helper files to match interfaces, read the existing plugin files (`noop.rs`, `mock.rs`, `whisper_plugin.rs`) and the plugin trait definitions first before coding.

End of prompt
