use coldvox_app::audio::AudioConfig;
use coldvox_app::vad::{VadEngineConfig, VadMode};

fn main() {
    // Minimal stub to satisfy cargo fmt and CI when examples feature is gated
    println!("VAD demo stub. Build with --features examples to run the full demo.");
    let _cfg = (
        VadEngineConfig {
            mode: VadMode::Silero,
        },
        AudioConfig {
            silence_threshold: 120,
        },
    );
}
