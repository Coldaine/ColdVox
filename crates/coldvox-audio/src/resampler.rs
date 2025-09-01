use rubato::{
    Resampler, SincFixedIn,
    SincInterpolationParameters, SincInterpolationType, WindowFunction,
};

use super::chunker::ResamplerQuality;

/// Streaming resampler for mono i16 audio using Rubato's high-quality sinc interpolation.
///
/// - Maintains internal buffers to handle arbitrary-sized input chunks
/// - Uses Rubato's SincFixedIn for high-quality, configurable resampling
/// - Automatically handles buffering for Rubato's fixed chunk requirements
pub struct StreamResampler {
    in_rate: u32,
    out_rate: u32,
    /// Rubato resampler instance
    resampler: SincFixedIn<f32>,
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
        // For VAD, we want low latency, so use a relatively small chunk size
        // 512 samples at 16kHz = 32ms, which aligns well with typical VAD frame sizes
        let chunk_size = 512;
        
        // Configure sinc interpolation based on quality preset
        let sinc_params = match quality {
            ResamplerQuality::Fast => {
                // Lower quality, faster processing
                SincInterpolationParameters {
                    sinc_len: 32,  // Shorter filter for lower CPU usage
                    f_cutoff: 0.92,  // Slightly more aggressive cutoff
                    interpolation: SincInterpolationType::Linear,  // Simpler interpolation
                    oversampling_factor: 64,  // Lower oversampling
                    window: WindowFunction::Blackman,  // Simple window
                }
            },
            ResamplerQuality::Balanced => {
                // Medium quality, good for speech
                SincInterpolationParameters {
                    sinc_len: 64,  // Medium quality
                    f_cutoff: 0.95,  // Slightly below Nyquist for better anti-aliasing
                    interpolation: SincInterpolationType::Cubic,
                    oversampling_factor: 128,  // Good balance of quality vs memory
                    window: WindowFunction::Blackman2,  // Good stopband attenuation
                }
            },
            ResamplerQuality::Quality => {
                // Higher quality, more CPU usage
                SincInterpolationParameters {
                    sinc_len: 128,  // Longer filter for better quality
                    f_cutoff: 0.97,  // Closer to Nyquist for sharper cutoff
                    interpolation: SincInterpolationType::Cubic,
                    oversampling_factor: 256,  // Higher oversampling for better quality
                    window: WindowFunction::BlackmanHarris2,  // Best stopband attenuation
                }
            },
        };
        
        // Create the resampler
        // We only need 1 channel for mono audio
        let resampler = SincFixedIn::<f32>::new(
            out_rate as f64 / in_rate as f64,  // Resample ratio
            2.0,  // Max resample ratio change (not used in fixed mode)
            sinc_params,
            chunk_size,
            1,  // mono
        ).expect("Failed to create Rubato resampler");
        
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
            return input.to_vec();
        }

        // Convert i16 to f32 and append to input buffer
        for &sample in input {
            self.input_buffer.push(sample as f32 / 32768.0);
        }

        // Process complete chunks
        while self.input_buffer.len() >= self.chunk_size {
            // Prepare input for Rubato (it expects Vec<Vec<f32>> for channels)
            let chunk: Vec<f32> = self.input_buffer.drain(..self.chunk_size).collect();
            let input_frames = vec![chunk];
            
            // Process the chunk
            let output_frames = match self.resampler.process(&input_frames, None) {
                Ok(frames) => frames,
                Err(e) => {
                    eprintln!("Resampler error: {}", e);
                    // Return empty on error to maintain stream continuity
                    return Vec::new();
                }
            };
            
            // Append resampled output (first channel only, since we're mono)
            if !output_frames.is_empty() && !output_frames[0].is_empty() {
                self.output_buffer.extend_from_slice(&output_frames[0]);
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
    pub fn input_rate(&self) -> u32 { self.in_rate }
    
    /// Current output rate.
    pub fn output_rate(&self) -> u32 { self.out_rate }
}

#[cfg(test)]
mod quality_tests {
    use super::*;

    #[test]
    fn process_with_all_quality_presets() {
        // Provide enough samples for internal filter latency to flush
        let input: Vec<i16> = (0..4096).map(|i| ((i % 100) as i16) - 50).collect(); // some signal
        for q in [ResamplerQuality::Fast, ResamplerQuality::Balanced, ResamplerQuality::Quality] {
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
        assert!(all_output.len() >= 1400 && all_output.len() <= 1700, 
                "Expected ~1600 samples, got {}", all_output.len());
    }

    #[test]
    fn upsample_16k_to_48k_constant() {
        let mut rs = StreamResampler::new(16_000, 48_000);
        // Constant tone: output should be approximately constant too
        let input = vec![1000i16; 1600];  // 100ms at 16kHz
        
        // Process in one go
        let out = rs.process(&input);
        
        // We should get approximately 3x the input samples
        // Allow wider variance due to Rubato's buffering strategy
        // The exact output depends on how the chunk size aligns with the resample ratio
        assert!(out.len() >= 4400 && out.len() <= 5000, 
                "Expected ~4800 samples, got {}", out.len());
        
        // Check middle samples are close to the input value
        // (skip edges which may have interpolation artifacts)
        if out.len() > 100 {
            for &s in &out[50..out.len().saturating_sub(50)] {
                assert!((900..=1100).contains(&s), 
                        "Sample {} too far from expected 1000", s);
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
