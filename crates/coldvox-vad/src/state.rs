use crate::types::{VadConfig, VadEvent, VadState};
use std::time::Instant;

pub struct VadStateMachine {
    state: VadState,
    
    speech_frames: u32,
    
    silence_frames: u32,
    
    speech_debounce_frames: u32,
    
    silence_debounce_frames: u32,
    
    speech_start_time: Option<Instant>,
    
    frames_since_start: u64,
    
    frame_duration_ms: f32,
}

impl VadStateMachine {
    pub fn new(config: &VadConfig) -> Self {
        Self {
            state: VadState::Silence,
            speech_frames: 0,
            silence_frames: 0,
            speech_debounce_frames: config.speech_debounce_frames(),
            silence_debounce_frames: config.silence_debounce_frames(),
            speech_start_time: None,
            frames_since_start: 0,
            frame_duration_ms: config.frame_duration_ms(),
        }
    }
    
    pub fn process(
        &mut self,
        is_speech_candidate: bool,
        energy_db: f32,
    ) -> Option<VadEvent> {
        self.frames_since_start += 1;
        
        match self.state {
            VadState::Silence => {
                if is_speech_candidate {
                    self.speech_frames += 1;
                    self.silence_frames = 0;
                    
                    if self.speech_frames >= self.speech_debounce_frames {
                        self.state = VadState::Speech;
                        self.speech_start_time = Some(Instant::now());
                        self.speech_frames = 0;
                        
                        return Some(VadEvent::SpeechStart {
                            timestamp_ms: self.current_timestamp_ms(),
                            energy_db,
                        });
                    }
                } else {
                    self.speech_frames = 0;
                }
            }
            
            VadState::Speech => {
                if !is_speech_candidate {
                    self.silence_frames += 1;
                    self.speech_frames = 0;
                    
                    if self.silence_frames >= self.silence_debounce_frames {
                        self.state = VadState::Silence;
                        
                        let duration_ms = if let Some(start) = self.speech_start_time {
                            let elapsed = start.elapsed().as_millis() as u64;
                            elapsed.max(1)
                        } else {
                            (self.silence_frames as f32 * self.frame_duration_ms).max(1.0) as u64
                        };
                        
                        self.speech_start_time = None;
                        self.silence_frames = 0;
                        
                        return Some(VadEvent::SpeechEnd {
                            timestamp_ms: self.current_timestamp_ms(),
                            duration_ms,
                            energy_db,
                        });
                    }
                } else {
                    self.silence_frames = 0;
                }
            }
        }
        
        None
    }
    
    pub fn current_state(&self) -> VadState {
        self.state
    }
    
    pub fn reset(&mut self) {
        self.state = VadState::Silence;
        self.speech_frames = 0;
        self.silence_frames = 0;
        self.speech_start_time = None;
        self.frames_since_start = 0;
    }
    
    fn current_timestamp_ms(&self) -> u64 {
        (self.frames_since_start as f32 * self.frame_duration_ms) as u64
    }
    
    pub fn force_end(&mut self, energy_db: f32) -> Option<VadEvent> {
        if self.state == VadState::Speech {
            self.state = VadState::Silence;
            
            let duration_ms = if let Some(start) = self.speech_start_time {
                let elapsed = start.elapsed().as_millis() as u64;
                elapsed.max(1)
            } else {
                (self.frames_since_start as f32 * self.frame_duration_ms * 0.1).max(1.0) as u64
            };
            
            self.speech_start_time = None;
            self.speech_frames = 0;
            self.silence_frames = 0;
            
            return Some(VadEvent::SpeechEnd {
                timestamp_ms: self.current_timestamp_ms(),
                duration_ms,
                energy_db,
            });
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{FRAME_SIZE_SAMPLES, SAMPLE_RATE_HZ};
    
    #[test]
    fn test_initial_state() {
        let config = VadConfig::default();
        let state_machine = VadStateMachine::new(&config);
        
        assert_eq!(state_machine.current_state(), VadState::Silence);
    }
    
    #[test]
    fn test_speech_onset_debouncing() {
        let config = VadConfig {
            speech_debounce_ms: 100,
            frame_size_samples: FRAME_SIZE_SAMPLES,
            sample_rate_hz: SAMPLE_RATE_HZ,
            ..Default::default()
        };
        let mut state_machine = VadStateMachine::new(&config);
        
        assert_eq!(state_machine.process(true, -30.0), None);
        assert_eq!(state_machine.current_state(), VadState::Silence);
        
        assert_eq!(state_machine.process(true, -30.0), None);
        assert_eq!(state_machine.current_state(), VadState::Silence);
        
        assert_eq!(state_machine.process(true, -30.0), None);
        assert_eq!(state_machine.current_state(), VadState::Silence);
        
        // Speech should trigger on the 4th frame (100ms debounce with ~32ms frames)
        if let Some(VadEvent::SpeechStart { .. }) = state_machine.process(true, -30.0) {
            assert_eq!(state_machine.current_state(), VadState::Speech);
        } else {
            panic!("Expected SpeechStart event");
        }
    }
    
    #[test]
    fn test_speech_offset_debouncing() {
        let config = VadConfig {
            speech_debounce_ms: 60,
            silence_debounce_ms: 100,
            frame_size_samples: FRAME_SIZE_SAMPLES,
            sample_rate_hz: SAMPLE_RATE_HZ,
            ..Default::default()
        };
        let mut state_machine = VadStateMachine::new(&config);
        
        for _ in 0..3 {
            state_machine.process(true, -30.0);
        }
        assert_eq!(state_machine.current_state(), VadState::Speech);
        
        for _ in 0..3 {
            assert_eq!(state_machine.process(false, -50.0), None);
            assert_eq!(state_machine.current_state(), VadState::Speech);
        }
        
        // SpeechEnd should trigger on the 4th silence frame (100ms debounce with ~32ms frames)
        if let Some(VadEvent::SpeechEnd { duration_ms, .. }) = state_machine.process(false, -50.0)
        {
            assert_eq!(state_machine.current_state(), VadState::Silence);
            assert!(duration_ms > 0);
        } else {
            panic!("Expected SpeechEnd event");
        }
    }
    
    #[test]
    fn test_speech_continuation() {
        let config = VadConfig {
            speech_debounce_ms: 60,
            silence_debounce_ms: 100,
            ..Default::default()
        };
        let mut state_machine = VadStateMachine::new(&config);
        
        for _ in 0..3 {
            state_machine.process(true, -30.0);
        }
        assert_eq!(state_machine.current_state(), VadState::Speech);
        
        state_machine.process(false, -50.0);
        state_machine.process(false, -50.0);
        
        state_machine.process(true, -30.0);
        
        assert_eq!(state_machine.current_state(), VadState::Speech);
    }
}