---
id: CODE:repo://crates/coldvox-text-injection/src/manager.rs
type: IMP
level: 6
title: Text Injection Manager Implementation
status: implemented
area: Text Injection
module: Strategy
owners: [@team-injection]
updated: 2025-09-11
links:
  implements: [COLDVOX-SPEC5-005-injection-backend-interface]
  depends_on: []
  verified_by: [COLDVOX-TST6-006-injection-manager-tests]
  related_to: []
---

## Summary
Implementation of adaptive text injection strategy manager.

## Description
This implementation provides the adaptive strategy manager that dynamically selects the most appropriate text injection method based on environment and historical success rates.

## Key Components
- Strategy manager with backend selection logic
- Success rate tracking and learning
- Fallback chain management
- Application-specific strategy caching

## Code Structure
```rust
// Strategy manager implementation
pub struct StrategyManager {
    backends: Vec<Box<dyn Backend>>,
    success_tracker: SuccessTracker,
    config: InjectionConfig,
}

impl StrategyManager {
    pub fn new(config: InjectionConfig) -> Result<Self, InjectionError> {
        let backends = initialize_backends(&config)?;
        let success_tracker = SuccessTracker::new();
        
        Ok(Self {
            backends,
            success_tracker,
            config,
        })
    }
    
    pub fn inject_text(&mut self, text: &str) -> Result<(), InjectionError> {
        let app_info = get_focused_app_info()?;
        let mut sorted_backends = self.rank_backends(&app_info);
        
        for backend in sorted_backends.iter_mut() {
            match backend.inject_text(text) {
                Ok(InjectionResult::Success) => {
                    self.success_tracker.record_success(backend.name(), &app_info);
                    return Ok(());
                }
                Ok(InjectionResult::Failed { reason, retryable }) => {
                    self.success_tracker.record_failure(backend.name(), &app_info);
                    if !retryable {
                        continue;
                    }
                }
                Ok(InjectionResult::NotSupported) => {
                    continue;
                }
                Err(e) => {
                    self.success_tracker.record_failure(backend.name(), &app_info);
                    return Err(e);
                }
            }
        }
        
        Err(InjectionError::NoAvailableBackend)
    }
}
```

## Dependencies
- anyhow = "1.0"
- thiserror = "1.0"
- regex = "1.0"

---
implements: COLDVOX-SPEC5-005-injection-backend-interface  
depends_on:  
verified_by: COLDVOX-TST6-006-injection-manager-tests  
related_to: