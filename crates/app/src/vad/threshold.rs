use crate::vad::types::VadConfig;

pub struct AdaptiveThreshold {
    noise_floor_db: f32,
    
    ema_alpha: f32,
    
    onset_offset_db: f32,
    
    offset_offset_db: f32,
    
    min_floor_db: f32,
    
    max_floor_db: f32,
}

impl AdaptiveThreshold {
    pub fn new(config: &VadConfig) -> Self {
        Self {
            noise_floor_db: config.initial_floor_db,
            ema_alpha: config.ema_alpha,
            onset_offset_db: config.onset_threshold_db,
            offset_offset_db: config.offset_threshold_db,
            min_floor_db: -80.0,
            max_floor_db: -20.0,
        }
    }
    
    pub fn update(&mut self, energy_db: f32, is_speech: bool) {
        if !is_speech && energy_db > self.min_floor_db && energy_db < self.max_floor_db {
            self.noise_floor_db = (1.0 - self.ema_alpha) * self.noise_floor_db
                + self.ema_alpha * energy_db;
            
            self.noise_floor_db = self
                .noise_floor_db
                .clamp(self.min_floor_db, self.max_floor_db);
        }
    }
    
    pub fn onset_threshold(&self) -> f32 {
        self.noise_floor_db + self.onset_offset_db
    }
    
    pub fn offset_threshold(&self) -> f32 {
        self.noise_floor_db + self.offset_offset_db
    }
    
    pub fn current_floor(&self) -> f32 {
        self.noise_floor_db
    }
    
    pub fn should_activate(&self, energy_db: f32) -> bool {
        energy_db >= self.onset_threshold()
    }
    
    pub fn should_deactivate(&self, energy_db: f32) -> bool {
        energy_db < self.offset_threshold()
    }
    
    pub fn reset(&mut self, initial_floor_db: f32) {
        self.noise_floor_db = initial_floor_db.clamp(self.min_floor_db, self.max_floor_db);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_threshold_initialization() {
        let config = VadConfig::default();
        let threshold = AdaptiveThreshold::new(&config);
        
        assert_eq!(threshold.current_floor(), -50.0);
        assert_eq!(threshold.onset_threshold(), -41.0);
        assert_eq!(threshold.offset_threshold(), -44.0);
    }
    
    #[test]
    fn test_noise_floor_adaptation() {
        let config = VadConfig {
            ema_alpha: 0.1,
            ..Default::default()
        };
        let mut threshold = AdaptiveThreshold::new(&config);
        
        threshold.update(-40.0, false);
        assert!((threshold.current_floor() - (-49.0)).abs() < 0.01);
        
        threshold.update(-40.0, false);
        assert!((threshold.current_floor() - (-48.1)).abs() < 0.01);
    }
    
    #[test]
    fn test_no_update_during_speech() {
        let config = VadConfig::default();
        let mut threshold = AdaptiveThreshold::new(&config);
        
        let initial_floor = threshold.current_floor();
        
        threshold.update(-30.0, true);
        threshold.update(-25.0, true);
        
        assert_eq!(threshold.current_floor(), initial_floor);
    }
    
    #[test]
    fn test_activation_deactivation() {
        let config = VadConfig::default();
        let threshold = AdaptiveThreshold::new(&config);
        
        assert!(threshold.should_activate(-40.0));
        assert!(!threshold.should_activate(-42.0));
        
        assert!(threshold.should_deactivate(-45.0));
        assert!(!threshold.should_deactivate(-43.0));
    }
}