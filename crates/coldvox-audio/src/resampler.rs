use audioadapter_buffers::owned::SequentialOwned;
use rubato::audioadapter::Adapter;
use rubato::{
    Async, FixedAsync, PolynomialDegree, Resampler, SincInterpolationParameters,
    SincInterpolationType, WindowFunction,
};

use super::chunker::ResamplerQuality;

/// Streaming resampler for mono i16 audio using Rubato's high-quality resampling.
///
/// - Maintains internal buffers to handle arbitrary-sized input chunks
/// - Uses Rubato's Async resampler for high-quality, configurable resampling
/// - Automatically handles buffering for Rubato's fixed chunk requirements
pub struct StreamResampler {
    in_rate: u32,
    out_rate: u32,
    /// Rubato resampler instance
    resampler: Async<f32>,
    /// Input buffer for accumulating samples
    input_buffer: Vec<f32>,
    /// Output buffer for accumulating resampled samples
    output_buffer: Vec<f32>,
    /// Chunk size required by Rubato
    chunk_size: usize,
}

impl StreamResampler {
    /// Create a new mono resampler from in_rate -> out_rate.
    pub fn new(in_rate: u32, out_rate: u32) -> Self {
        Self::new_with_quality(in_rate, out_rate, ResamplerQuality::Balanced)
    }

    /// Create a new mono resampler with specified quality preset.
    pub fn new_with_quality(in_rate: u32, out_rate: u32, quality: ResamplerQuality) -> Self {
        tracing::debug!(
            "Creating resampler: {}Hz -> {}Hz with quality {:?}",
            in_rate,
            out_rate,
            quality
        );
        // For VAD, we want low latency, so use a relatively small chunk size
        // 512 samples at 16kHz = 32ms, which aligns well with typical VAD frame sizes
        let chunk_size = 512;

        // Create the resampler based on quality preset
        let resampler = match quality {
            ResamplerQuality::Fast => {
                // Fast polynomial interpolation (no anti-aliasing)
                Async::<f32>::new_poly(
                    out_rate as f64 / in_rate as f64,
                    2.0, // Max ratio change
                    PolynomialDegree::Linear,
                    chunk_size,
                    1, // mono
                    FixedAsync::Input,
                )
                .expect("Failed to create polynomial resampler")
            }
            ResamplerQuality::Balanced => {
                // Cubic polynomial interpolation (good balance)
                Async::<f32>::new_poly(
                    out_rate as f64 / in_rate as f64,
                    2.0,
                    PolynomialDegree::Cubic,
                    chunk_size,
                    1, // mono
                    FixedAsync::Input,
                )
                .expect("Failed to create polynomial resampler")
            }
            ResamplerQuality::Quality => {
                // Sinc interpolation with anti-aliasing (best quality)
                let sinc_params = SincInterpolationParameters {
                    sinc_len: 128,
                    f_cutoff: 0.95,
                    interpolation: SincInterpolationType::Cubic,
                    oversampling_factor: 128,
                    window: WindowFunction::Blackman2,
                };
                Async::<f32>::new_sinc(
                    out_rate as f64 / in_rate as f64,
                    2.0,
                    &sinc_params,
                    chunk_size,
                    1, // mono
                    FixedAsync::Input,
                )
                .expect("Failed to create sinc resampler")
            }
        };

        Self {
            in_rate,
            out_rate,
            resampler,
            input_buffer: Vec::with_capacity(chunk_size * 2),
            output_buffer: Vec::new(),
            chunk_size,
        }
    }

    /// Process an arbitrary chunk of mono i16 samples.
    /// Returns a freshly allocated Vec with resampled i16 at out_rate.
    pub fn process(&mut self, input: &[i16]) -> Vec<i16> {
        if self.in_rate == self.out_rate {
            // Fast path: just clone input
            tracing::trace!(
                "Resampler: Passthrough {} samples (no rate change)",
                input.len()
            );
            return input.to_vec();
        }

        // Convert i16 to f32 and append to input buffer
        for &sample in input {
            self.input_buffer.push(sample as f32 / 32768.0);
        }

        // Process complete chunks
        while self.input_buffer.len() >= self.chunk_size {
            // Prepare input for Rubato using SequentialOwned adapter
            // SequentialOwned stores all samples for one channel consecutively
            let chunk: Vec<f32> = self.input_buffer.drain(..self.chunk_size).collect();
            let input_adapter = SequentialOwned::new_from(chunk, 1, self.chunk_size)
                .expect("Failed to create input adapter");

            // Process the chunk - use process method which allocates output
            match self.resampler.process(
                &input_adapter,
                0,    // input_offset
                None, // active_channels_mask
            ) {
                Ok(output_frames) => {
                    // output_frames is SequentialOwned - copy data from channel 0
                    let out_frames = output_frames.frames();
                    let mut temp_buffer = vec![0.0f32; out_frames];
                    let copied = output_frames.copy_from_channel_to_slice(0, 0, &mut temp_buffer);
                    self.output_buffer.extend_from_slice(&temp_buffer[..copied]);
                }
                Err(e) => {
                    tracing::error!("Resampler error: {}", e);
                    // Return empty on error to maintain stream continuity
                    return Vec::new();
                }
            }
        }

        // Convert accumulated f32 samples back to i16
        let mut result = Vec::with_capacity(self.output_buffer.len());
        for &sample in &self.output_buffer {
            // Clamp to [-1.0, 1.0] and convert to i16
            let clamped = sample.clamp(-1.0, 1.0);
            let i16_sample = (clamped * 32767.0).round() as i16;
            result.push(i16_sample);
        }

        // Clear the output buffer for next time
        self.output_buffer.clear();

        if !result.is_empty() {
            tracing::trace!(
                "Resampler: Processed {} input samples -> {} output samples ({}Hz -> {}Hz)",
                input.len(),
                result.len(),
                self.in_rate,
                self.out_rate
            );
        }

        result
    }

    /// Reset internal state, clearing buffers and resetting the resampler.
    pub fn reset(&mut self) {
        self.input_buffer.clear();
        self.output_buffer.clear();
        // Reset the resampler's internal state
        self.resampler.reset();
    }

    /// Current input rate.
    pub fn input_rate(&self) -> u32 {
        self.in_rate
    }

    /// Current output rate.
    pub fn output_rate(&self) -> u32 {
        self.out_rate
    }
}

#[cfg(test)]
mod quality_tests {
    use super::*;

    #[test]
    fn process_with_all_quality_presets() {
        // Provide enough samples for internal filter latency to flush
        let input: Vec<i16> = (0..4096).map(|i| ((i % 100) as i16) - 50).collect(); // some signal
        for q in [
            ResamplerQuality::Fast,
            ResamplerQuality::Balanced,
            ResamplerQuality::Quality,
        ] {
            let mut rs = StreamResampler::new_with_quality(48_000, 16_000, q);
            let mut out = rs.process(&input);
            // Process a second chunk to ensure output becomes available
            out.extend(rs.process(&input));
            // Downsampling by ~3x should yield non-empty output
            assert!(!out.is_empty());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downsample_48k_to_16k_ramp() {
        let mut rs = StreamResampler::new(48_000, 16_000);
        // 4.8k samples (~0.1s). Expect ~1.6k out.
        let n_in = 4_800;
        let input: Vec<i16> = (0..n_in).map(|i| (i % 32768) as i16).collect();

        // Process in chunks to test buffering
        let mut all_output = Vec::new();
        for chunk in input.chunks(1000) {
            let out = rs.process(chunk);
            all_output.extend(out);
        }

        // We should get approximately 1/3 of the input samples
        // Allow some variance due to buffering
        assert!(
            all_output.len() >= 1400 && all_output.len() <= 1700,
            "Expected ~1600 samples, got {}",
            all_output.len()
        );
    }

    #[test]
    fn upsample_16k_to_48k_constant() {
        let mut rs = StreamResampler::new(16_000, 48_000);
        // Constant tone: output should be approximately constant too
        let input = vec![1000i16; 1600]; // 100ms at 16kHz

        // Process in one go
        let out = rs.process(&input);

        // We should get approximately 3x the input samples
        // Allow wider variance due to Rubato's buffering strategy
        // The exact output depends on how the chunk size aligns with the resample ratio
        assert!(
            out.len() >= 4400 && out.len() <= 5000,
            "Expected ~4800 samples, got {}",
            out.len()
        );

        // Check middle samples are close to the input value
        // (skip edges which may have interpolation artifacts)
        if out.len() > 100 {
            for &s in &out[50..out.len().saturating_sub(50)] {
                assert!(
                    (900..=1100).contains(&s),
                    "Sample {} too far from expected 1000",
                    s
                );
            }
        }
    }

    #[test]
    fn passthrough_same_rate() {
        let mut rs = StreamResampler::new(16_000, 16_000);
        let input = vec![100i16, 200, 300, 400, 500];
        let output = rs.process(&input);
        assert_eq!(input, output, "Passthrough should return identical data");
    }
}
