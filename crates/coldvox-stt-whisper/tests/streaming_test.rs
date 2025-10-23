use coldvox_stt::plugin::SttPlugin;
use coldvox_stt::types::{TranscriptionConfig, TranscriptionEvent};
use coldvox_stt_whisper::FasterWhisperPlugin;
use hound::WavReader;
use std::path::Path;
use rubato::{Resampler, SincFixedIn, SincInterpolationType, SincInterpolationParameters, WindowFunction};

async fn create_test_plugin() -> FasterWhisperPlugin {
    let mut plugin = FasterWhisperPlugin::new();
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let model_path = Path::new(manifest_dir)
        .join("../../models/whisper/ggml-tiny.en.bin")
        .canonicalize()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    let config = TranscriptionConfig {
        model_path,
        ..Default::default()
    };
    plugin.initialize(config).await.unwrap();
    plugin
}

fn load_audio_file(path: &str) -> Vec<i16> {
    let reader = WavReader::open(path).unwrap();
    let spec = reader.spec();
    let samples: Vec<f32> = reader.into_samples::<i16>().map(|s| s.unwrap() as f32 / 32768.0).collect();

    let mono_samples = if spec.channels == 2 {
        samples.chunks_exact(2).map(|chunk| (chunk[0] + chunk[1]) / 2.0).collect()
    } else {
        samples.clone()
    };

    if spec.sample_rate == 16000 {
        return mono_samples.into_iter().map(|s| (s * 32768.0) as i16).collect();
    }

    let params = SincInterpolationParameters {
        sinc_len: 256,
        f_cutoff: 0.95,
        interpolation: SincInterpolationType::Linear,
        oversampling_factor: 256,
        window: WindowFunction::BlackmanHarris2,
    };
    let mut resampler = SincFixedIn::<f32>::new(
        16000 as f64 / spec.sample_rate as f64,
        2.0,
        params,
        mono_samples.len(),
        1,
    ).unwrap();

    let resampled = resampler.process(&[mono_samples], None).unwrap();
    resampled[0].iter().map(|&s| (s * 32768.0) as i16).collect()
}

#[tokio::test]
async fn test_final_event_with_correct_content() {
    let mut plugin = create_test_plugin().await;
    let audio = load_audio_file("test_data/LRMonoPhase4.wav");

    plugin.process_audio(&audio).await.unwrap();
    let result = plugin.finalize().await.unwrap();

    assert!(matches!(result, Some(TranscriptionEvent::Final { .. })));
    if let Some(TranscriptionEvent::Final { text, .. }) = result {
        let lower_text = text.to_lowercase();
        assert!(lower_text.contains("left"));
        assert!(lower_text.contains("right"));
    }
}

#[tokio::test]
async fn test_reset_clears_buffer() {
    let mut plugin = create_test_plugin().await;
    let audio = load_audio_file("test_data/LRMonoPhase4.wav");

    plugin.process_audio(&audio[..16000]).await.unwrap();
    plugin.reset().await.unwrap();
    let result = plugin.finalize().await.unwrap();

    assert!(result.is_none());
}