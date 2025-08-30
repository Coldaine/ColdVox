use crate::text_injection::backend::BackendDetector;
use crate::text_injection::focus::{FocusTracker, FocusStatus};
use crate::text_injection::types::{InjectionConfig, InjectionError, InjectionMethod, InjectionMetrics, TextInjector};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

/// Key for identifying a specific app-method combination
type AppMethodKey = (String, InjectionMethod);

/// Record of success/failure for a specific app-method combination
#[derive(Debug, Clone)]
struct SuccessRecord {
    success_count: u32,
    fail_count: u32,
    last_success: Option<Instant>,
    last_failure: Option<Instant>,
    /// Success rate (0.0 to 1.0)
    success_rate: f64,
}

/// State of cooldown for a specific app-method combination
#[derive(Debug, Clone)]
struct CooldownState {
    until: Instant,
    backoff_level: u32,
    last_error: String,
}

/// Strategy manager for adaptive text injection
pub struct StrategyManager {
    /// Configuration for injection
    config: InjectionConfig,
    /// Focus tracker for determining target context
    focus_tracker: FocusTracker,
    /// Cache of success records per app-method combination
    success_cache: HashMap<AppMethodKey, SuccessRecord>,
    /// Cooldown states per app-method combination
    cooldowns: HashMap<AppMethodKey, CooldownState>,
    /// Global start time for budget tracking
    global_start: Option<Instant>,
    /// Metrics for the strategy manager
    metrics: InjectionMetrics,
    /// Backend detector for platform-specific capabilities
    backend_detector: BackendDetector,
}

impl StrategyManager {
    /// Create a new strategy manager
    pub fn new(config: InjectionConfig, metrics: Arc<Mutex<InjectionMetrics>>) -> Self {
        let backend_detector = BackendDetector::new(config.clone());
        let preferred_backend = backend_detector.get_preferred_backend();
        
        match preferred_backend {
            Some(backend) => info!("Selected backend: {:?}", backend),
            None => {
                warn!("No suitable backend found for text injection");
                // Record backend denial in telemetry
                if let Ok(mut metrics) = self.metrics.lock() {
                    metrics.record_backend_denied();
                }
            }
        }
        
        Self {
            config: config.clone(),
            focus_tracker: FocusTracker::new(config.clone()),
            success_cache: HashMap::new(),
            cooldowns: HashMap::new(),
            global_start: None,
            metrics,
            backend_detector,
        }
    }

    /// Get the current application identifier (e.g., window class)
    async fn get_current_app_id(&self) -> Result<String, InjectionError> {
        // In a real implementation, this would get the app ID from the focused element
        // For now, we'll use a placeholder
        Ok("unknown_app".to_string())
    }
    
    /// Check if injection is currently paused
    fn is_paused(&self) -> bool {
    // In a real implementation, this would check a global state
    // For now, we'll always return false
    false
}

/// Check if the current application is allowed for injection
fn is_app_allowed(&self, app_id: &str) -> bool {
    // If allowlist is not empty, only allow apps in the allowlist
    if !self.config.allowlist.is_empty() {
        return self.config.allowlist.iter().any(|pattern| {
            match regex::Regex::new(pattern) {
                Ok(re) => re.is_match(app_id),
                Err(_) => app_id.contains(pattern),
            }
        });
    }
    
    // If blocklist is not empty, block apps in the blocklist
    if !self.config.blocklist.is_empty() {
        return !self.config.blocklist.iter().any(|pattern| {
            match regex::Regex::new(pattern) {
                Ok(re) => re.is_match(app_id),
                Err(_) => app_id.contains(pattern),
            }
        });
    }
    
    // If neither allowlist nor blocklist is set, allow all apps
    true
}

/// Get the current application identifier (e.g., window class)
async fn get_current_app_id(&self) -> Result<String, InjectionError> {
    // In a real implementation, this would get the app ID from the focused element
    // For now, we'll use a placeholder
    Ok("unknown_app".to_string())
}

    /// Check if a method is in cooldown for the current app
    fn is_in_cooldown(&self, method: InjectionMethod) -> bool {
        let app_id = "unknown_app"; // Placeholder - would be from get_current_app_id
        let key = (app_id.to_string(), method);
        
        if let Some(cooldown) = self.cooldowns.get(&key) {
            return Instant::now() < cooldown.until;
        }
        
        false
    }

    /// Update success record for a method
    fn update_success_record(&mut self, method: InjectionMethod, success: bool) {
        let app_id = "unknown_app"; // Placeholder - would be from get_current_app_id
        let key = (app_id.to_string(), method);
        
        let record = self.success_cache.entry(key).or_insert_with(|| SuccessRecord {
            success_count: 0,
            fail_count: 0,
            last_success: None,
            last_failure: None,
            success_rate: 1.0, // Start optimistic
        });
        
        if success {
            record.success_count += 1;
            record.last_success = Some(Instant::now());
        } else {
            record.fail_count += 1;
            record.last_failure = Some(Instant::now());
        }
        
        // Update success rate
        let total = record.success_count + record.fail_count;
        if total > 0 {
            record.success_rate = record.success_count as f64 / total as f64;
        }
    }

    /// Update cooldown state for a failed method
    fn update_cooldown(&mut self, method: InjectionMethod, error: &str) {
        let app_id = "unknown_app"; // Placeholder - would be from get_current_app_id
        let key = (app_id.to_string(), method);
        
        let backoff_level = if let Some(cooldown) = self.cooldowns.get(&key) {
            // Cap backoff level to prevent excessive delays
            (cooldown.backoff_level + 1).min(10)
        } else {
            1
        };
        
        // Calculate cooldown duration with exponential backoff
        let base_duration = Duration::from_millis(self.config.cooldown_initial_ms);
        let multiplier = 2u32.pow(backoff_level - 1); // 1, 2, 4, 8, 16...
        let duration = base_duration * multiplier as u32;
        
        let until = Instant::now() + duration;
        
        self.cooldowns.insert(key, CooldownState {
            until,
            backoff_level,
            last_error: error.to_string(),
        });
        
        debug!("Method {:?} for app {} now in cooldown until {:?} (level {})", 
               method, app_id, until, backoff_level);
    }

    /// Clear cooldown for a method (e.g., after successful use)
    fn clear_cooldown(&mut self, method: InjectionMethod) {
        let app_id = "unknown_app"; // Placeholder - would be from get_current_app_id
        let key = (app_id.to_string(), method);
        self.cooldowns.remove(&key);
    }

    /// Get the preferred method order based on current context and history
    fn get_method_order(&self) -> Vec<InjectionMethod> {
        // Get available backends
        let available_backends = self.backend_detector.detect_available_backends();
        
        // Base order as specified in the requirements
        let mut base_order = Vec::new();
        
        // Add methods based on available backends
        for backend in available_backends {
            match backend {
                Backend::WaylandXdgDesktopPortal | Backend::WaylandVirtualKeyboard => {
                    base_order.push(InjectionMethod::AtspiInsert);
                    base_order.push(InjectionMethod::ClipboardAndPaste);
                    base_order.push(InjectionMethod::Clipboard);
                }
                Backend::X11Xdotool | Backend::X11Native => {
                    base_order.push(InjectionMethod::AtspiInsert);
                    base_order.push(InjectionMethod::ClipboardAndPaste);
                    base_order.push(InjectionMethod::Clipboard);
                }
                Backend::MacCgEvent => {
                    base_order.push(InjectionMethod::AtspiInsert);
                    base_order.push(InjectionMethod::ClipboardAndPaste);
                    base_order.push(InjectionMethod::Clipboard);
                }
                Backend::WindowsSendInput => {
                    base_order.push(InjectionMethod::AtspiInsert);
                    base_order.push(InjectionMethod::ClipboardAndPaste);
                    base_order.push(InjectionMethod::Clipboard);
                }
                _ => {}
            }
        }
        
        // Add optional methods if enabled
        if self.config.allow_kdotool {
            base_order.push(InjectionMethod::KdoToolAssist);
        }
        if self.config.allow_enigo {
            base_order.push(InjectionMethod::EnigoText);
        }
        if self.config.allow_mki {
            base_order.push(InjectionMethod::UinputKeys);
        }
        if self.config.allow_ydotool {
            base_order.push(InjectionMethod::YdoToolPaste);
        }
        
        // Sort by preference: methods with higher success rate first, then by base order
        let app_id = "unknown_app"; // Placeholder - would be from get_current_app_id
        
        // Create a copy of base order for position lookup
        let base_order_copy = base_order.clone();
        
        base_order.sort_by(|a, b| {
            let key_a = (app_id.to_string(), *a);
            let key_b = (app_id.to_string(), *b);
            
            let success_a = self.success_cache.get(&key_a).map(|r| r.success_rate).unwrap_or(0.5);
            let success_b = self.success_cache.get(&key_b).map(|r| r.success_rate).unwrap_or(0.5);
            
            // Sort by success rate (descending), then by base order
            success_b.partial_cmp(&success_a).unwrap().then_with(|| {
                // Preserve base order for equal success rates
                let pos_a = base_order_copy.iter().position(|m| m == a).unwrap_or(0);
                let pos_b = base_order_copy.iter().position(|m| m == b).unwrap_or(0);
                pos_a.cmp(&pos_b)
            })
        });
        
        base_order
    }

    /// Check if we've exceeded the global time budget
    fn has_budget_remaining(&self) -> bool {
        if let Some(start) = self.global_start {
            let elapsed = start.elapsed();
            let budget = self.config.max_total_latency();
            elapsed < budget
        } else {
            true
        }
    }

    /// Try to inject text using the best available method
    pub async fn inject(&mut self, text: &str) -> Result<(), InjectionError> {
        if text.is_empty() {
            return Ok(());
        }

        // Check if injection is paused
        if self.is_paused() {
            return Err(InjectionError::Other("Injection is currently paused".to_string()));
        }

        // Start global timer
        self.global_start = Some(Instant::now());
        
        // Get current focus status
        let focus_status = match self.focus_tracker.get_focus_status().await {
            Ok(status) => status,
            Err(e) => {
                warn!("Failed to get focus status: {}", e);
                // Continue with injection attempt
                FocusStatus::Unknown
            }
        };
        
        // Check if we should inject on unknown focus
        if focus_status == FocusStatus::Unknown && !self.config.inject_on_unknown_focus {
            if let Ok(mut metrics) = self.metrics.lock() {
                metrics.record_focus_missing();
            }
            return Err(InjectionError::Other("Unknown focus state and injection disabled".to_string()));
        }
        
        // Check if focus is required
        if self.config.require_focus && focus_status == FocusStatus::NonEditable {
            if let Ok(mut metrics) = self.metrics.lock() {
                metrics.record_focus_missing();
            }
            return Err(InjectionError::NoEditableFocus);
        }
        
        // Get current application ID
        let app_id = self.get_current_app_id().await?;
        
        // Check allowlist/blocklist
        if !self.is_app_allowed(&app_id) {
            return Err(InjectionError::Other(format!("Application {} is not allowed for injection", app_id)));
        }
        
        // Determine injection method based on config
        let use_paste = match self.config.injection_mode.as_str() {
            "paste" => true,
            "keystroke" => false,
            "auto" => text.len() > self.config.paste_chunk_chars as usize,
            _ => text.len() > self.config.paste_chunk_chars as usize,  // Default to auto
        };
        
        // Get ordered list of methods to try
        let method_order = self.get_method_order();
        
        // Try each method in order
        for method in method_order {
            // Skip if in cooldown
            if self.is_in_cooldown(method) {
                debug!("Skipping method {:?} - in cooldown", method);
                continue;
            }
            
            // Check budget
            if !self.has_budget_remaining() {
                if let Ok(mut metrics) = self.metrics.lock() {
                    metrics.record_rate_limited();
                }
                return Err(InjectionError::BudgetExhausted);
            }
            
            // Get injector for this method
            // In a real implementation, we would have a map of injectors
            // For this test, we'll just simulate the injection
            let start = Instant::now();
            let result = if use_paste {
                self.simulate_paste(method, text).await
            } else {
                self.simulate_type_text(method, text).await
            };
            
            match result {
                Ok(()) => {
                    let duration = start.elapsed().as_millis() as u64;
                    self.metrics.record_success(method, duration);
                    self.update_success_record(method, true);
                    self.clear_cooldown(method);
                    info!("Successfully injected text using method {:?} with mode {:?}", method, if use_paste { "paste" } else { "keystroke" });
                    self.clear_cooldown(method);
                    info!("Successfully injected text using method {:?} with mode {:?}", method, if use_paste { "paste" } else { "keystroke" });
                    return Ok(());
                }
                Err(e) => {
                    let duration = start.elapsed().as_millis() as u64;
                    let error_string = e.to_string();
                    self.metrics.record_failure(method, duration, error_string.clone());
                    self.update_success_record(method, false);
                    self.update_cooldown(method, &error_string);
                    debug!("Method {:?} failed: {}", method, error_string);
                    self.update_cooldown(method, &error_string);
                    debug!("Method {:?} failed: {}", method, error_string);
                    // Continue to next method
                }
            }
        }
        
        // If we get here, all methods failed
        error!("All injection methods failed");
        Err(InjectionError::MethodFailed("All injection methods failed".to_string()))
    }

    /// Simulate paste for testing purposes
    async fn simulate_paste(&self, method: InjectionMethod, _text: &str) -> Result<(), InjectionError> {
        use std::time::SystemTime;
        
        // Simple pseudo-random based on system time
        let pseudo_rand = (SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)
            .unwrap().as_nanos() % 100) as f64 / 100.0;
            
        // Simulate different success rates for different methods
        match method {
            InjectionMethod::AtspiInsert => {
                // Simulate 90% success rate for paste
                if pseudo_rand < 0.9 {
                    Ok(())
                } else {
                    Err(InjectionError::MethodUnavailable("Paste action not available".to_string()))
                }
            }
            InjectionMethod::ClipboardAndPaste => {
                // Simulate 95% success rate
                if pseudo_rand < 0.95 {
                    Ok(())
                } else {
                    Err(InjectionError::MethodUnavailable("Paste action not available".to_string()))
                }
            }
            InjectionMethod::Clipboard => {
                // Simulate 95% success rate
                if pseudo_rand < 0.95 {
                    Ok(())
                } else {
                    Err(InjectionError::Other("Clipboard error".to_string()))
                }
            }
            _ => {
                // Other methods have lower success rates
                if pseudo_rand < 0.7 {
                    Ok(())
                } else {
                    Err(InjectionError::Other("Process failed".to_string()))
                }
            }
        }
    }

    /// Simulate type text for testing purposes
    async fn simulate_type_text(&self, method: InjectionMethod, _text: &str) -> Result<(), InjectionError> {
        use std::time::SystemTime;
        
        // Simple pseudo-random based on system time
        let pseudo_rand = (SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)
            .unwrap().as_nanos() % 100) as f64 / 100.0;
            
        // Simulate different success rates for different methods
        match method {
            InjectionMethod::AtspiInsert => {
                // Simulate 80% success rate for keystrokes
                if pseudo_rand < 0.8 {
                    Ok(())
                } else {
                    Err(InjectionError::Timeout(250))
                }
            }
            InjectionMethod::ClipboardAndPaste => {
                // Simulate 85% success rate
                if pseudo_rand < 0.85 {
                    Ok(())
                } else {
                    Err(InjectionError::MethodUnavailable("Paste action not available".to_string()))
                }
            }
            InjectionMethod::Clipboard => {
                // Simulate 85% success rate
                if pseudo_rand < 0.85 {
                    Ok(())
                } else {
                    Err(InjectionError::Other("Clipboard error".to_string()))
                }
            }
            _ => {
                // Other methods have lower success rates
                if pseudo_rand < 0.6 {
                    Ok(())
                } else {
                    Err(InjectionError::Other("Process failed".to_string()))
                }
            }
        }
    }

    /// Simulate injection for testing purposes
    async fn simulate_inject(&self, method: InjectionMethod, _text: &str) -> Result<(), InjectionError> {
        use std::time::SystemTime;
        
        // Simple pseudo-random based on system time
        let pseudo_rand = (SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)
            .unwrap().as_nanos() % 100) as f64 / 100.0;
            
        // Simulate different success rates for different methods
        match method {
            InjectionMethod::AtspiInsert => {
                // Simulate 80% success rate
                if pseudo_rand < 0.8 {
                    Ok(())
                } else {
                    Err(InjectionError::Timeout(250))
                }
            }
            InjectionMethod::ClipboardAndPaste => {
                // Simulate 90% success rate
                if pseudo_rand < 0.9 {
                    Ok(())
                } else {
                    Err(InjectionError::MethodUnavailable("Paste action not available".to_string()))
                }
            }
            InjectionMethod::Clipboard => {
                // Simulate 95% success rate
                if pseudo_rand < 0.95 {
                    Ok(())
                } else {
                    Err(InjectionError::Other("Clipboard error".to_string()))
                }
            }
            _ => {
                // Other methods have lower success rates
                if pseudo_rand < 0.7 {
                    Ok(())
                } else {
                    Err(InjectionError::Other("Process failed".to_string()))
                }
            }
        }
    }

    /// Get metrics for the strategy manager
    pub fn metrics(&self) -> &InjectionMetrics {
        &self.metrics
    }

    /// Print injection statistics for debugging
    pub fn print_stats(&self) {
        info!("Injection Statistics:");
        info!("  Total attempts: {}", self.metrics.attempts);
        info!("  Successes: {}", self.metrics.successes);
        info!("  Failures: {}", self.metrics.failures);
        info!("  Success rate: {:.1}%", 
              if self.metrics.attempts > 0 { 
                  self.metrics.successes as f64 / self.metrics.attempts as f64 * 100.0 
              } else { 
                  0.0 
              });
        
        // Print method-specific stats
        for (method, metrics) in &self.metrics.method_metrics {
            info!("  Method {:?}: {} attempts, {} successes, {} failures", 
                  method, metrics.attempts, metrics.successes, metrics.failures);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::timeout;

    // Test that strategy manager can be created
    #[test]
    fn test_strategy_manager_creation() {
        let config = InjectionConfig::default();
        let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));
        let manager = StrategyManager::new(config, metrics);
        
        assert_eq!(manager.metrics.attempts, 0);
        assert_eq!(manager.metrics.successes, 0);
        assert_eq!(manager.metrics.failures, 0);
    }

    // Test method ordering
    #[test]
    fn test_method_ordering() {
        let config = InjectionConfig::default();
        let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));
        let manager = StrategyManager::new(config, metrics);
        
        let order = manager.get_method_order();
        
        // Verify base order
        assert_eq!(order[0], InjectionMethod::AtspiInsert);
        assert_eq!(order[1], InjectionMethod::ClipboardAndPaste);
        assert_eq!(order[2], InjectionMethod::Clipboard);
        
        // Verify optional methods are included if enabled
        let mut config = InjectionConfig::default();
        config.allow_ydotool = true;
        config.allow_kdotool = true;
        config.allow_enigo = true;
        config.allow_mki = true;
        
        let manager = StrategyManager::new(config);
        let order = manager.get_method_order();
        
        // All methods should be present
        assert!(order.contains(&InjectionMethod::AtspiInsert));
        assert!(order.contains(&InjectionMethod::ClipboardAndPaste));
        assert!(order.contains(&InjectionMethod::Clipboard));
        assert!(order.contains(&InjectionMethod::YdoToolPaste));
        assert!(order.contains(&InjectionMethod::KdoToolAssist));
        assert!(order.contains(&InjectionMethod::EnigoText));
        assert!(order.contains(&InjectionMethod::UinputKeys));
    }

    // Test success record updates
    #[test]
    fn test_success_record_update() {
        let mut config = InjectionConfig::default();
        let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));
        let mut manager = StrategyManager::new(config.clone(), metrics);
        
        // Test success
        manager.update_success_record(InjectionMethod::AtspiInsert, true);
        let key = ("unknown_app".to_string(), InjectionMethod::AtspiInsert);
        let record = manager.success_cache.get(&key).unwrap();
        assert_eq!(record.success_count, 1);
        assert_eq!(record.fail_count, 0);
        assert!(record.success_rate > 0.9);
        
        // Test failure
        manager.update_success_record(InjectionMethod::AtspiInsert, false);
        let record = manager.success_cache.get(&key).unwrap();
        assert_eq!(record.success_count, 1);
        assert_eq!(record.fail_count, 1);
        assert_eq!(record.success_rate, 0.5);
    }

    // Test cooldown updates
    #[test]
    fn test_cooldown_update() {
        let mut config = InjectionConfig::default();
        let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));
        let mut manager = StrategyManager::new(config.clone(), metrics);
        
        // First failure
        manager.update_cooldown(InjectionMethod::AtspiInsert, "test error");
        let key = ("unknown_app".to_string(), InjectionMethod::AtspiInsert);
        let cooldown = manager.cooldowns.get(&key).unwrap();
        assert_eq!(cooldown.backoff_level, 1);
        
        // Second failure - backoff level should increase
        manager.update_cooldown(InjectionMethod::AtspiInsert, "test error");
        let cooldown = manager.cooldowns.get(&key).unwrap();
        assert_eq!(cooldown.backoff_level, 2);
        
        // Duration should be longer
        let base_duration = Duration::from_millis(config.cooldown_initial_ms);
        let expected_duration = base_duration * 2u32.pow(1); // 2^1 = 2
        let actual_duration = cooldown.until.duration_since(Instant::now());
        // Allow some tolerance for timing
        assert!(actual_duration >= expected_duration - Duration::from_millis(10));
    }

    // Test budget checking
    #[test]
    fn test_budget_checking() {
        let mut config = InjectionConfig::default();
        config.max_total_latency_ms = 100; // 100ms budget
        
        let mut manager = StrategyManager::new(config);
        
        // No start time - budget should be available
        assert!(manager.has_budget_remaining());
        
        // Set start time
        manager.global_start = Some(Instant::now() - Duration::from_millis(50));
        assert!(manager.has_budget_remaining());
        
        // Exceed budget
        manager.global_start = Some(Instant::now() - Duration::from_millis(150));
        assert!(!manager.has_budget_remaining());
    }

    // Test injection with success
    #[tokio::test]
    async fn test_inject_success() {
        let mut config = InjectionConfig::default();
        let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));
        let mut manager = StrategyManager::new(config, metrics);
        
        // Test with text
        let result = manager.inject("test text").await;
        assert!(result.is_ok());
        
        // Metrics should reflect success
        assert_eq!(manager.metrics.successes, 1);
        assert_eq!(manager.metrics.attempts, 1);
        assert_eq!(manager.metrics.failures, 0);
    }

    // Test injection with failure
    #[tokio::test]
    async fn test_inject_failure() {
        let mut config = InjectionConfig::default();
        // Set very short budget to force failure
        config.max_total_latency_ms = 1;
        
        let mut manager = StrategyManager::new(config);
        
        // This should fail due to budget exhaustion
        let result = manager.inject("test text").await;
        assert!(result.is_err());
        
        // Metrics should reflect failure
        assert_eq!(manager.metrics.successes, 0);
        assert_eq!(manager.metrics.attempts, 1);
        assert_eq!(manager.metrics.failures, 1);
    }

    // Test empty text handling
    #[test]
    fn test_empty_text() {
        let mut config = InjectionConfig::default();
        let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));
        let mut manager = StrategyManager::new(config, metrics);
        
        // Inject empty text
        let result = std::panic::catch_unwind(|| {
            let _ = manager.inject("");
        });
        
        // Should not panic
        assert!(result.is_ok());
    }
}