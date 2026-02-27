use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use coldvox_audio_quality::{AudioQualityMonitor, QualityConfig, LevelMonitor, SpectralAnalyzer};

/// Generate test signal at specific amplitude
fn generate_signal(samples: usize, amplitude: f32) -> Vec<i16> {
    (0..samples)
        .map(|i| {
            let t = i as f32 / 16000.0;
            let signal = (2.0 * std::f32::consts::PI * 1000.0 * t).sin();
            (signal * amplitude * 32767.0) as i16
        })
        .collect()
}

fn bench_rms_calculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("rms_calculation");

    for size in [512, 1024, 2048].iter() {
        let samples = generate_signal(*size, 0.5);
        let mut monitor = LevelMonitor::new(16000, 500, 1000);

        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &samples,
            |b, s| b.iter(|| {
                monitor.update_rms(black_box(s))
            }),
        );
    }

    group.finish();
}

fn bench_peak_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("peak_detection");

    for size in [512, 1024, 2048].iter() {
        let samples = generate_signal(*size, 0.5);
        let mut monitor = LevelMonitor::new(16000, 500, 1000);

        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &samples,
            |b, s| b.iter(|| {
                monitor.update_peak(black_box(s))
            }),
        );
    }

    group.finish();
}

fn bench_spectral_analysis(c: &mut Criterion) {
    let mut group = c.benchmark_group("spectral_analysis");

    for size in [512, 1024, 2048].iter() {
        let samples = generate_signal(*size, 0.5);
        let mut analyzer = SpectralAnalyzer::new(16000);

        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &samples,
            |b, s| b.iter(|| {
                analyzer.detect_off_axis(black_box(s))
            }),
        );
    }

    group.finish();
}

fn bench_full_analysis(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_analysis");

    for size in [512, 1024, 2048].iter() {
        let samples = generate_signal(*size, 0.5);
        let config = QualityConfig::default();
        let mut monitor = AudioQualityMonitor::new(config);

        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &samples,
            |b, s| b.iter(|| {
                monitor.analyze(black_box(s))
            }),
        );
    }

    group.finish();
}

fn bench_frame_budget_compliance(c: &mut Criterion) {
    // Critical test: Verify we meet the 1.6% overhead budget for 512 samples @ 16kHz
    // Frame duration: 512 samples / 16000 Hz = 32ms
    // Budget: 32ms * 1.6% = 512 microseconds (0.512ms)

    let samples = generate_signal(512, 0.5);
    let config = QualityConfig::default();
    let mut monitor = AudioQualityMonitor::new(config);

    c.bench_function("frame_budget_512_samples", |b| {
        b.iter(|| {
            monitor.analyze(black_box(&samples))
        })
    });
}

criterion_group!(
    benches,
    bench_rms_calculation,
    bench_peak_detection,
    bench_spectral_analysis,
    bench_full_analysis,
    bench_frame_budget_compliance
);
criterion_main!(benches);
