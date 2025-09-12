---
id: COLDVOX-TST6-004-vad-processor-tests
type: TST
level: 6
title: VAD Processor Tests
status: implemented
owner: @team-audio
updated: 2025-09-11
parent: COLDVOX-SYS4-004-vad-processor
links:
  verifies: [COLDVOX-SYS4-004-vad-processor]
  depends_on: []
  related_to: []
---

## Summary
Test suite for the VAD processor implementation.

## Description
This test suite verifies the correct operation of the VAD processor, including speech detection accuracy and event generation.

## Test Cases
1. Speech detection accuracy
2. Event generation (SpeechStart, SpeechEnd)
3. Threshold configuration testing
4. Debouncing verification
5. Performance benchmarking

## Test Code
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_speech_detection() {
        let config = UnifiedVadConfig {
            threshold: 0.3,
            min_speech_duration: Duration::from_millis(250),
            min_silence_duration: Duration::from_millis(100),
            mode: VadMode::Silero,
        };
        
        let mut vad_engine = SileroEngine::new(&config).unwrap();
        
        // Generate speech-like signal
        let speech_signal: Vec<f32> = (0..512)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 16000.0).sin() * 0.8)
            .collect();
        
        let event = vad_engine.process_frame(&speech_signal).unwrap();
        
        // Should detect speech
        match event {
            VadEvent::SpeechStart { confidence } => {
                assert!(confidence > 0.3);
            }
            _ => panic!("Expected SpeechStart event"),
        }
    }
    
    #[test]
    fn test_noise_detection() {
        let config = UnifiedVadConfig {
            threshold: 0.5,
            min_speech_duration: Duration::from_millis(250),
            min_silence_duration: Duration::from_millis(100),
            mode: VadMode::Silero,
        };
        
        let mut vad_engine = SileroEngine::new(&config).unwrap();
        
        // Generate noise-like signal (low amplitude)
        let noise_signal: Vec<f32> = (0..512)
            .map(|_| rand::random::<f32>() * 0.1)
            .collect();
        
        let event = vad_engine.process_frame(&noise_signal).unwrap();
        
        // Should detect noise
        match event {
            VadEvent::Noise => {
                // Expected
            }
            _ => panic!("Expected Noise event"),
        }
    }
}
```

## Requirements
- Comprehensive test coverage
- Speech detection accuracy verification
- Event generation testing
- Threshold configuration testing

---
verifies: COLDVOX-SYS4-004-vad-processor  
depends_on:  
related_to: