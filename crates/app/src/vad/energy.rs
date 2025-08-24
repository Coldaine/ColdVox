pub struct EnergyCalculator {
    epsilon: f32,
}

impl EnergyCalculator {
    pub fn new() -> Self {
        Self {
            epsilon: 1e-10,
        }
    }
    
    pub fn calculate_rms(&self, frame: &[i16]) -> f32 {
        if frame.is_empty() {
            return 0.0;
        }
        
        let sum_squares: i64 = frame
            .iter()
            .map(|&sample| {
                let s = sample as i64;
                s * s
            })
            .sum();
        
        let mean_square = sum_squares as f64 / frame.len() as f64;
        (mean_square.sqrt() / 32768.0) as f32
    }
    
    pub fn rms_to_dbfs(&self, rms: f32) -> f32 {
        if rms <= self.epsilon {
            return -100.0;
        }
        20.0 * rms.log10()
    }
    
    pub fn calculate_dbfs(&self, frame: &[i16]) -> f32 {
        let rms = self.calculate_rms(frame);
        self.rms_to_dbfs(rms)
    }
    
    pub fn calculate_energy_ratio(&self, frame: &[i16], reference_db: f32) -> f32 {
        let current_db = self.calculate_dbfs(frame);
        current_db - reference_db
    }
}

impl Default for EnergyCalculator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_silence_returns_low_dbfs() {
        let calc = EnergyCalculator::new();
        let silence = vec![0i16; 320];
        let db = calc.calculate_dbfs(&silence);
        assert!(db <= -100.0);
    }
    
    #[test]
    fn test_full_scale_returns_zero_dbfs() {
        let calc = EnergyCalculator::new();
        let full_scale = vec![32767i16; 320];
        let db = calc.calculate_dbfs(&full_scale);
        assert!((db - 0.0).abs() < 0.1);
    }
    
    #[test]
    fn test_half_scale_returns_minus_six_dbfs() {
        let calc = EnergyCalculator::new();
        let half_scale = vec![16384i16; 320];
        let db = calc.calculate_dbfs(&half_scale);
        assert!((db - (-6.0)).abs() < 0.1);
    }
    
    #[test]
    fn test_rms_calculation() {
        let calc = EnergyCalculator::new();
        
        let sine_wave: Vec<i16> = (0..320)
            .map(|i| {
                let phase = 2.0 * std::f32::consts::PI * i as f32 / 320.0;
                (phase.sin() * 16384.0) as i16
            })
            .collect();
        
        let rms = calc.calculate_rms(&sine_wave);
        
        assert!((rms - 0.354).abs() < 0.01);
    }
}