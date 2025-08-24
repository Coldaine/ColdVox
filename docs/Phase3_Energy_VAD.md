## Energy VAD Module Specification

A modular, progressive implementation of energy-based VAD that can be used standalone or as a gate/fallback for ML-based VAD.

### Implementation Levels

#### Level 1: Basic Energy Gate (Current MVP)
- Simple RMS energy calculation
- Fixed threshold: -40 dBFS
- Binary decision: process/skip frame
- No state tracking

```rust
fn energy_gate(frame: &[i16]) -> bool {
    let rms = calculate_rms_dbfs(frame);
    rms > -40.0
}
```

#### Level 2: Adaptive Threshold (Next Step)
- RMS energy with dBFS calculation
- EMA noise floor tracking
- Relative threshold: floor + 9dB
- Simple hysteresis: on/off thresholds

```rust
struct AdaptiveGate {
    floor_db: f32,          // EMA noise floor
    on_threshold_db: f32,   // floor + 9dB
    off_threshold_db: f32,  // on - 3dB
}
```

#### Level 3: Debounced State Machine
- All of Level 2
- State tracking (SILENCE/SPEECH)
- Minimum duration requirements
- Debounce timers (200ms speech, 400ms silence)

#### Level 4: Full Energy VAD (from EnergyBasedVAD.md)
- All of Level 3
- Pre-emphasis filter (0.97)
- High-pass filter (100Hz)
- ZCR gating (optional)
- Pre/post-roll buffering
- Clipping detection

### Module Interface

```rust
// Trait that all levels implement
pub trait EnergyVAD: Send {
    /// Process a frame and return activity decision
    fn process_frame(&mut self, frame: &[i16]) -> VadDecision;
    
    /// Reset internal state
    fn reset(&mut self);
    
    /// Get current metrics
    fn metrics(&self) -> EnergyMetrics;
}

pub enum VadDecision {
    Silent,
    Active,
    Unknown,  // Used during warmup
}

pub struct EnergyMetrics {
    pub noise_floor_db: Option<f32>,
    pub last_energy_db: f32,
    pub frames_processed: u64,
}
```

### Progressive Enhancement Path

1. **Start with Level 1** in Phase 3
   - Use as simple gate before Silero
   - Collect metrics on gating effectiveness

2. **Upgrade to Level 2** when needed
   - Better handling of varying noise levels
   - Still lightweight

3. **Add Level 3** for standalone operation
   - Can work without ML VAD
   - Provides event generation

4. **Full Level 4** for production
   - Maximum robustness
   - All sophisticated features

### Integration with Phase 3

```rust
// In vad/mod.rs
pub struct VadProcessor {
    silero: Option<SileroVAD>,
    energy: Box<dyn EnergyVAD>,  // Start with Level1, upgrade later
    config: VadConfig,
}

impl VadProcessor {
    pub fn process_window(&mut self, window: &[i16]) -> Option<f32> {
        // Use energy VAD as gate
        match self.energy.process_frame(window) {
            VadDecision::Silent => {
                // Skip Silero, return 0.0 probability
                self.metrics.frames_gated += 1;
                return Some(0.0);
            }
            VadDecision::Active | VadDecision::Unknown => {
                // Run Silero
                if let Some(silero) = &mut self.silero {
                    return silero.process(window);
                }
            }
        }
        None
    }
}
```

### File Structure

```
crates/app/src/audio/vad/
├── mod.rs           # VadProcessor, traits, common types
├── energy/
│   ├── mod.rs       # EnergyVAD trait and factory
│   ├── level1.rs    # Basic energy gate
│   ├── level2.rs    # Adaptive threshold
│   ├── level3.rs    # Debounced state machine
│   └── level4.rs    # Full implementation
└── silero.rs        # Silero wrapper
```

### Configuration

```rust
pub enum EnergyVadLevel {
    Basic,      // Level 1
    Adaptive,   // Level 2
    Debounced,  // Level 3
    Full,       // Level 4
}

pub struct VadConfig {
    pub energy_level: EnergyVadLevel,
    pub energy_threshold_dbfs: f32,  // For Level 1
    pub energy_params: Option<EnergyParams>,  // For Levels 2-4
}
```

### Benefits

1. **Immediate value**: Level 1 saves CPU today
2. **No overengineering**: Start simple, enhance as needed
3. **Clean separation**: Energy VAD is independent module
4. **Easy testing**: Each level can be tested independently
5. **Graceful upgrades**: Same interface, better implementation
6. **Fallback ready**: If Silero fails, Level 3+ can take over