use criterion::{
    black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput,
};

// Old approach: allocate Vec<char> and chunk by character count
fn chunk_old_collect(text: &str, chunk_chars: usize) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let mut count = 0usize;
    let mut i = 0usize;
    while i < chars.len() {
        let end = (i + chunk_chars).min(chars.len());
        let _chunk: String = chars[i..end].iter().collect();
        count += 1;
        i = end;
    }
    count
}

// New approach: byte slicing on UTF-8 boundaries without intermediate allocation
fn chunk_new_iter(text: &str, chunk_chars: usize) -> usize {
    let mut count = 0usize;
    let mut chars_seen = 0usize;
    let mut start = 0usize;
    for (i, ch) in text.char_indices() {
        // end_byte should point after the current char
        let end_byte = i + ch.len_utf8();
        chars_seen += 1;
        if chars_seen == chunk_chars {
            let _chunk = &text[start..end_byte];
            count += 1;
            start = end_byte;
            chars_seen = 0;
        }
    }
    if start < text.len() {
        let _chunk = &text[start..];
        count += 1;
    }
    count
}

fn make_test_string(target_len: usize) -> String {
    // Mix ASCII and multi-byte emoji to enforce boundary handling
    let pattern = "Hello, 世界 🌍🚀✨ — The quick brown 🦊 jumps over the lazy 🐶. ";
    let mut s = String::with_capacity(target_len + pattern.len());
    while s.len() < target_len {
        s.push_str(pattern);
    }
    s
}

pub fn chunking_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("text_chunking");
    let sizes = [10_000usize, 50_000, 100_000]; // bytes target (approx)
    let chunk_chars = 256usize;

    for size in sizes.iter() {
        // Prepare input once per size
        let input = make_test_string(*size);
        group.throughput(Throughput::Bytes(input.len() as u64));

        group.bench_with_input(BenchmarkId::new("old_collect", size), &input, |b, input| {
            b.iter_batched(
                || input.clone(),
                |text| black_box(chunk_old_collect(&text, chunk_chars)),
                BatchSize::SmallInput,
            )
        });

        group.bench_with_input(BenchmarkId::new("new_iter", size), &input, |b, input| {
            b.iter(|| black_box(chunk_new_iter(input, chunk_chars)))
        });
    }

    group.finish();
}

criterion_group!(benches, chunking_benchmark);
criterion_main!(benches);
