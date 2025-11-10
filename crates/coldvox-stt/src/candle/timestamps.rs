use candle::{Result, Tensor, D};
use candle_nn::ops::softmax_last_dim;
use dtw::Dtw;
use candle_transformers::models::whisper::tokenizer::Tokenizer;

#[derive(Debug, Clone)]
pub struct TranscriptionResult {
    pub start: f64,
    pub end: f64,
    pub text: String,
}

const AUDIO_TIME_PER_TOKEN: f64 = 0.02;

/// Performs word alignment on the given tokens and attention weights.
pub fn perform_word_alignment(
    tokens: &[i32],
    attention_weights: &[Tensor],
    tokenizer: &Tokenizer,
    use_space: bool,
) -> Result<Vec<TranscriptionResult>> {
    let (words, word_tokens_indices) = if use_space {
        split_tokens_on_spaces(tokens, tokenizer)
    } else {
        let (words, _, indices) = split_tokens_on_unicode(tokens, tokenizer);
        (words, indices)
    };

    let weights = Tensor::cat(attention_weights, 0)?;
    let num_tokens = weights.dims()[2];
    let num_frames = weights.dims()[3];

    let mut weights = weights.mean(0)?.mean(0)?; // Average over layers and heads
    weights = weights.to_dtype(candle::DType::F64)?;

    let weights_data = weights.flatten_all()?.to_vec1::<f64>()?;

    let mut dtw = Dtw::new(&weights_data, num_frames as usize, num_tokens as usize);
    let alignment = dtw.run();

    let mut jumps = vec![];
    if !alignment.path.is_empty() {
        let mut last_token = alignment.path[0].0;
        jumps.push(alignment.path[0].1);
        for &(token, frame) in &alignment.path {
            if token != last_token {
                jumps.push(frame);
                last_token = token;
            }
        }
        jumps.push(alignment.path.last().unwrap().1);
    }

    let mut word_boundaries = vec![0];
    word_boundaries.extend(word_tokens_indices.iter().scan(0, |acc, tokens| {
        *acc += tokens.len();
        Some(*acc)
    }));

    let begin_times = word_boundaries
        .iter()
        .map(|&boundary| jumps.get(boundary).cloned().unwrap_or(0))
        .collect::<Vec<_>>();
    let end_times = word_boundaries
        .iter()
        .skip(1)
        .map(|&boundary| jumps.get(boundary).cloned().unwrap_or(0))
        .collect::<Vec<_>>();

    let mut results = Vec::new();
    for i in 0..words.len() {
        if words[i].starts_with("<|") {
            continue;
        }
        results.push(TranscriptionResult {
            text: words[i].clone(),
            start: round_timestamp(begin_times[i] as f64 * AUDIO_TIME_PER_TOKEN),
            end: round_timestamp(end_times[i] as f64 * AUDIO_TIME_PER_TOKEN),
        });
    }

    Ok(results)
}

pub fn perform_timestamp_probs_alignment(
    tokens: &[i32],
    logits: &Tensor,
    tokenizer: &Tokenizer,
) -> Result<Vec<TranscriptionResult>> {
    let mut words = vec![];
    let mut current_word = String::new();
    let mut start_time = 0.0;

    let timestamp_begin = tokenizer.timestamp_begin() as usize;
    for (i, &token) in tokens.iter().enumerate() {
        let text = tokenizer.decode(&[token as u32], true).unwrap_or_default();
        if text.starts_with("<|") && text.ends_with("|>") {
            if !current_word.is_empty() {
                let end_time = get_timestamp_from_logits(logits, i, timestamp_begin)?;
                words.push(TranscriptionResult {
                    start: start_time,
                    end: end_time,
                    text: current_word,
                });
            }
            current_word = String::new();
            start_time = get_timestamp_from_logits(logits, i, timestamp_begin)?;
        } else {
            current_word.push_str(&text);
        }
    }

    if !current_word.is_empty() {
        let end_time = get_timestamp_from_logits(logits, tokens.len() - 1, timestamp_begin)?;
        words.push(TranscriptionResult {
            start: start_time,
            end: end_time,
            text: current_word,
        });
    }

    Ok(words)
}

fn get_timestamp_from_logits(logits: &Tensor, index: usize, timestamp_begin: usize) -> Result<f64> {
    let logits = logits.i((0, index, ..))?;
    let probs = softmax_last_dim(&logits)?;
    let probs_data: Vec<f32> = probs.to_vec1()?;

    let mut max_prob = 0.0;
    let mut max_index = 0;
    for (i, &prob) in probs_data.iter().enumerate() {
        if i >= timestamp_begin && prob > max_prob {
            max_prob = prob;
            max_index = i;
        }
    }

    Ok((max_index - timestamp_begin) as f64 * AUDIO_TIME_PER_TOKEN)
}


fn split_tokens_on_spaces(
    tokens: &[i32],
    tokenizer: &Tokenizer,
) -> (Vec<String>, Vec<Vec<i32>>) {
    let (subwords, _, subword_tokens_indices_list) =
        split_tokens_on_unicode(tokens, tokenizer);
    let mut words = vec![];
    let mut word_indices = vec![];

    for (subword, indices) in subwords.into_iter().zip(subword_tokens_indices_list.into_iter()) {
        if subword.starts_with(' ') {
            words.push(subword.trim_start().to_string());
            word_indices.push(indices);
        } else if let Some(last_word) = words.last_mut() {
            *last_word += &subword;
            word_indices.last_mut().unwrap().extend(indices);
        } else {
            words.push(subword);
            word_indices.push(indices);
        }
    }
    (words, word_indices)
}

fn split_tokens_on_unicode(
    tokens: &[i32],
    tokenizer: &Tokenizer,
) -> (Vec<String>, Vec<Vec<String>>, Vec<Vec<i32>>) {
    let mut words = vec![];
    let mut word_tokens = vec![];
    let mut word_tokens_indices = vec![];
    let mut current_tokens = vec![];

    for &token in tokens {
        current_tokens.push(token);
        let u32_tokens: Vec<u32> = current_tokens.iter().map(|&t| t as u32).collect();
        if let Ok(decoded) = tokenizer.decode(&u32_tokens, true) {
            if !decoded.contains('�') {
                words.push(decoded.clone());
                word_tokens.push(vec![decoded]);
                word_tokens_indices.push(current_tokens.clone());
                current_tokens.clear();
            }
        }
    }
    (words, word_tokens, word_tokens_indices)
}

fn round_timestamp(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}
