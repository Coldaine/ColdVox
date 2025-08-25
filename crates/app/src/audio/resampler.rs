use std::cmp;

/// Simple streaming linear resampler for mono i16 audio.
///
/// - Maintains internal accumulator of past input samples so callers can push
///   arbitrary-sized chunks.
/// - Produces as many output samples as possible based on the input provided
///   and the input/output rate ratio.
/// - Uses linear interpolation. Sufficient for VAD and monitoring. Low cost.
pub struct StreamResampler {
    in_rate: u32,
    out_rate: u32,
    /// Input sample accumulator (mono)
    acc: Vec<i16>,
    /// Current fractional read position in `acc` in input-sample units
    phase: f32,
    /// Phase increment per output sample: in_rate / out_rate
    inc: f32,
}

impl StreamResampler {
    /// Create a new mono resampler from in_rate -> out_rate.
    pub fn new(in_rate: u32, out_rate: u32) -> Self {
        let inc = in_rate as f32 / out_rate as f32;
        Self {
            in_rate,
            out_rate,
            acc: Vec::with_capacity((in_rate.min(out_rate)) as usize),
            phase: 0.0,
            inc,
        }
    }

    /// Process an arbitrary chunk of mono i16 samples.
    /// Returns a freshly allocated Vec with resampled i16 at out_rate.
    pub fn process(&mut self, input: &[i16]) -> Vec<i16> {
        if self.in_rate == self.out_rate {
            // Fast path: just clone input
            return input.to_vec();
        }

        // Append to accumulator
        self.acc.extend_from_slice(input);

        // Upper bound on number of outputs we might produce this call
        // Over-allocate a bit to avoid growth
        let max_out = ((self.acc.len() as f32 - self.phase).max(0.0) / self.inc) as usize;
        let mut out = Vec::with_capacity(max_out);

        // We need at least two samples to interpolate
        while (self.phase + 1.0) < (self.acc.len() as f32) {
            let idx = self.phase as usize;
            let frac = self.phase - idx as f32;

            // Safe due to while-condition; idx + 1 < acc.len()
            let s0 = self.acc[idx] as f32;
            let s1 = self.acc[idx + 1] as f32;
            let sample = s0 * (1.0 - frac) + s1 * frac;

            // Convert back to i16 with saturation
            let y = sample.round().clamp(i16::MIN as f32, i16::MAX as f32) as i16;
            out.push(y);

            self.phase += self.inc;
        }

        // Drop fully consumed input samples to keep memory bounded
        let consumed = cmp::min(self.phase as usize, self.acc.len());
        if consumed > 0 {
            self.acc.drain(..consumed);
            self.phase -= consumed as f32;
        }

        out
    }

    /// Reset internal state, clearing buffers and phase.
    pub fn reset(&mut self) {
        self.acc.clear();
        self.phase = 0.0;
    }

    /// Current input rate.
    pub fn input_rate(&self) -> u32 { self.in_rate }
    /// Current output rate.
    pub fn output_rate(&self) -> u32 { self.out_rate }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downsample_48k_to_16k_ramp() {
        let mut rs = StreamResampler::new(48_000, 16_000);
        // 4.8k samples (~0.1s). Expect ~1.6k out.
        let n_in = 4_800;
        let input: Vec<i16> = (0..n_in).map(|i| (i as i16)).collect();
        let out = rs.process(&input);
        assert!(out.len() >= 1500 && out.len() <= 1700, "len {}", out.len());
        // Monotonic non-decreasing for a ramp
        for w in out.windows(2) {
            assert!(w[1] >= w[0]);
        }
    }

    #[test]
    fn upsample_16k_to_48k_constant() {
        let mut rs = StreamResampler::new(16_000, 48_000);
        // Constant tone: output should be approximately constant too
        let input = vec![1000i16; 320];
        let out = rs.process(&input);
        assert!(out.len() >= 900 && out.len() <= 1000, "len {}", out.len());
        // Values within a small band around 1000 due to interpolation edges
        for &s in &out[10..out.len().saturating_sub(10)] {
            assert!(s >= 980 && s <= 1020, "{}", s);
        }
    }
}
