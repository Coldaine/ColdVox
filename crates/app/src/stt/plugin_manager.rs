//! STT Plugin Manager for ColdVox
//!
//! This module manages STT plugin selection and fallback logic

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use coldvox_stt::plugin::{
    SttPlugin, SttPluginRegistry, PluginSelectionConfig, FailoverConfig, GcPolicy, MetricsConfig, SttPluginError
};
use coldvox_stt::plugins::{NoOpPlugin, MockPlugin};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::{info, warn, error, debug};

/// Manages STT plugin lifecycle and selection
pub struct SttPluginManager {
    registry: Arc<RwLock<SttPluginRegistry>>,
    current_plugin: Arc<RwLock<Option<Box<dyn SttPlugin>>>>,
    selection_config: PluginSelectionConfig,
    
    // Failover tracking
    consecutive_errors: Arc<RwLock<HashMap<String, u32>>>,
    last_failover: Arc<RwLock<Option<Instant>>>,
    failed_plugins_cooldown: Arc<RwLock<HashMap<String, Instant>>>,
    
    // GC management
    gc_task: Arc<RwLock<Option<JoinHandle<()>>>>,
    last_activity: Arc<RwLock<HashMap<String, Instant>>>,
    
    // Metrics
    failover_count: Arc<std::sync::atomic::AtomicU64>,
    total_errors: Arc<std::sync::atomic::AtomicU64>,
}

impl SttPluginManager {
    /// Create a new plugin manager with default configuration
    pub fn new() -> Self {
        let mut registry = SttPluginRegistry::new();

        // Register built-in plugins
        Self::register_builtin_plugins(&mut registry);

        Self {
            registry: Arc::new(RwLock::new(registry)),
            current_plugin: Arc::new(RwLock::new(None)),
            selection_config: PluginSelectionConfig::default(),
            consecutive_errors: Arc::new(RwLock::new(HashMap::new())),
            last_failover: Arc::new(RwLock::new(None)),
            failed_plugins_cooldown: Arc::new(RwLock::new(HashMap::new())),
            gc_task: Arc::new(RwLock::new(None)),
            last_activity: Arc::new(RwLock::new(HashMap::new())),
            failover_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            total_errors: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Update plugin selection configuration at runtime
    pub async fn set_selection_config(&mut self, cfg: PluginSelectionConfig) {
        let gc_enabled = cfg.gc_policy.as_ref().map_or(false, |gc| gc.enabled);
        
        self.selection_config = cfg;
        
        // Start or stop GC task based on configuration
        if gc_enabled {
            self.start_gc_task().await;
        } else {
            self.stop_gc_task().await;
        }
        
        info!("Updated STT plugin selection configuration");
    }

    /// Start the garbage collection task
    async fn start_gc_task(&self) {
        let gc_policy = match &self.selection_config.gc_policy {
            Some(policy) if policy.enabled => policy.clone(),
            _ => return,
        };

        let mut gc_task = self.gc_task.write().await;
        
        // Stop existing task if running
        if let Some(handle) = gc_task.take() {
            handle.abort();
        }

    let last_activity = self.last_activity.clone();
    let ttl_secs = if gc_policy.model_ttl_secs == 0 { 1 } else { gc_policy.model_ttl_secs }; // prevent zero-second TTL causing rapid loop
        
        // Spawn new GC task
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(ttl_secs / 2));
            
            loop {
                interval.tick().await;
                
                let now = Instant::now();
                let mut activity = last_activity.write().await;
                let inactive_plugins: Vec<String> = activity
                    .iter()
                    .filter_map(|(plugin_id, last_used)| {
                        if now.duration_since(*last_used).as_secs() > ttl_secs as u64 {
                            Some(plugin_id.clone())
                        } else {
                            None
                        }
                    })
                    .collect();
                
                for plugin_id in inactive_plugins {
                    debug!("GC: Marking plugin {} as inactive (TTL expired)", plugin_id);
                    activity.remove(&plugin_id);
                    // TODO: Actually unload plugin models when plugins support it
                }
                
                drop(activity);
            }
        });
        
        *gc_task = Some(handle);
        debug!("Started STT plugin GC task with TTL {}s", ttl_secs);
    }

    /// Stop the garbage collection task
    async fn stop_gc_task(&self) {
        let mut gc_task = self.gc_task.write().await;
        if let Some(handle) = gc_task.take() {
            handle.abort();
            debug!("Stopped STT plugin GC task");
        }
    }

    /// Garbage collect inactive plugin models.
    pub async fn gc_inactive_models(&self) {
        let gc_policy = match &self.selection_config.gc_policy {
            Some(policy) if policy.enabled => policy,
            _ => return,
        };

        let now = Instant::now();
        let mut activity = self.last_activity.write().await;
        let ttl_secs = gc_policy.model_ttl_secs as u64;
        
        let inactive_plugins: Vec<String> = activity
            .iter()
            .filter_map(|(plugin_id, last_used)| {
                if now.duration_since(*last_used).as_secs() > ttl_secs {
                    Some(plugin_id.clone())
                } else {
                    None
                }
            })
            .collect();
        
        for plugin_id in inactive_plugins {
            debug!("GC: Removing inactive plugin {}", plugin_id);
            activity.remove(&plugin_id);
            // TODO: Actually unload plugin models when plugins support it
        }
        
        debug!("GC completed, processed {} plugins", activity.len());
    }

    /// Register all built-in plugins
    fn register_builtin_plugins(registry: &mut SttPluginRegistry) {
        use coldvox_stt::plugins::noop::NoOpPluginFactory;
        use coldvox_stt::plugins::mock::MockPluginFactory;

        // Always available plugins
        registry.register(Box::new(NoOpPluginFactory));
        registry.register(Box::new(MockPluginFactory::default()));

        // Conditionally register Vosk if feature is enabled
        #[cfg(feature = "vosk")]
        {
            use coldvox_stt::plugins::vosk_plugin::VoskPluginFactory;
            registry.register(Box::new(VoskPluginFactory::new()));
        }

        // Future: Register other plugins
        // #[cfg(feature = "whisper")]
        // registry.register(Box::new(WhisperPluginFactory::new()));

        // #[cfg(feature = "gcloud-stt")]
        // registry.register(Box::new(GoogleCloudSttFactory::new()));
    }

    /// Initialize the plugin manager and select the best available plugin
    pub async fn initialize(&mut self) -> Result<String, SttPluginError> {
        let registry = self.registry.read().await;

        // List available plugins
        let available = registry.available_plugins();
        info!("Available STT plugins:");
        for plugin_info in &available {
            info!(
                "  - {} ({}): {} [Available: {}]",
                plugin_info.id,
                plugin_info.name,
                plugin_info.description,
                plugin_info.is_available
            );
        }

        // Try to create the best available plugin
        let plugin = if let Some(ref preferred) = self.selection_config.preferred_plugin {
            // Try preferred plugin first
            match registry.create_plugin(preferred) {
                Ok(p) => {
                    info!("Using preferred STT plugin: {}", preferred);
                    p
                }
                Err(e) => {
                    warn!("Preferred plugin '{}' not available: {}", preferred, e);
                    // Fall back to best available
                    self.create_fallback_plugin(&*registry)?
                }
            }
        } else {
            // Use best available
            self.create_fallback_plugin(&*registry)?
        };

        let plugin_id = plugin.info().id.clone();

        // Store the selected plugin
        let mut current = self.current_plugin.write().await;
        *current = Some(plugin);

        // Record initial activity to avoid immediate GC
        {
            let mut activity = self.last_activity.write().await;
            activity.insert(plugin_id.clone(), Instant::now());
        }

        Ok(plugin_id)
    }

    /// Create a fallback plugin when preferred isn't available
    fn create_fallback_plugin(
        &self,
        registry: &SttPluginRegistry
    ) -> Result<Box<dyn SttPlugin>, SttPluginError> {
        // Try fallback plugins in order
        for fallback_id in &self.selection_config.fallback_plugins {
            match registry.create_plugin(fallback_id) {
                Ok(p) => {
                    info!("Using fallback STT plugin: {}", fallback_id);
                    return Ok(p);
                }
                Err(e) => {
                    warn!("Fallback plugin '{}' not available: {}", fallback_id, e);
                }
            }
        }

        // Last resort: try any available plugin
        match registry.create_best_available() {
            Ok(p) => {
                info!("Using best available STT plugin: {}", p.info().id);
                Ok(p)
            }
            Err(_) => {
                // Ultimate fallback: NoOp plugin
                warn!("No STT plugins available, using NoOp plugin");
                Ok(Box::new(NoOpPlugin::new()))
            }
        }
    }

    /// Get the current plugin
    pub async fn current_plugin(&self) -> Option<String> {
        let current = self.current_plugin.read().await;
        current.as_ref().map(|p| p.info().id.clone())
    }

    /// Switch to a different plugin
    pub async fn switch_plugin(&mut self, plugin_id: &str) -> Result<(), SttPluginError> {
        let registry = self.registry.read().await;
        let new_plugin = registry.create_plugin(plugin_id)?;

        info!("Switching to STT plugin: {}", plugin_id);

        let mut current = self.current_plugin.write().await;
        *current = Some(new_plugin);

        Ok(())
    }

    /// Get information about all available plugins
    pub async fn list_plugins(&self) -> Vec<coldvox_stt::plugin::PluginInfo> {
        let registry = self.registry.read().await;
        registry.available_plugins()
    }

    /// Process audio with the current plugin, handling failover on errors
    pub async fn process_audio(
        &mut self,
        samples: &[i16]
    ) -> Result<Option<coldvox_stt::types::TranscriptionEvent>, String> {
        let mut current = self.current_plugin.write().await;

        if let Some(ref mut plugin) = *current {
            let plugin_id = plugin.info().id.clone();
            
            // Update last activity for GC
            {
                let mut activity = self.last_activity.write().await;
                activity.insert(plugin_id.clone(), Instant::now());
            }
            
            match plugin.process_audio(samples).await {
                Ok(result) => {
                    // Reset error count on success
                    {
                        let mut errors = self.consecutive_errors.write().await;
                        errors.remove(&plugin_id);
                    }
                    Ok(result)
                }
                Err(e) => {
                    // Track error and potentially trigger failover
                    self.total_errors.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    
                    let should_failover = {
                        let mut errors = self.consecutive_errors.write().await;
                        let current_errors = errors.entry(plugin_id.clone()).or_insert(0);
                        *current_errors += 1;
                        
                        let threshold = self.selection_config.failover
                            .as_ref()
                            .map_or(3, |f| f.failover_threshold);
                        
                        *current_errors >= threshold
                    };
                    
                    if should_failover {
                        warn!("Plugin {} exceeded error threshold, attempting failover", plugin_id);
                        drop(current); // Release the lock before attempting failover
                        
                        // Attempt failover
                        match self.attempt_failover(&plugin_id).await {
                            Ok(new_plugin_id) => {
                                info!("Successfully failed over from {} to {}", plugin_id, new_plugin_id);
                                self.failover_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                {
                                    let mut lf = self.last_failover.write().await;
                                    *lf = Some(Instant::now());
                                }
                                
                                // Record cooldown for failed plugin
                                {
                                    let mut cooldown = self.failed_plugins_cooldown.write().await;
                                    cooldown.insert(plugin_id, Instant::now());
                                }
                                
                                // Try processing with new plugin
                                let mut current = self.current_plugin.write().await;
                                if let Some(ref mut new_plugin) = *current {
                                    new_plugin.process_audio(samples).await.map_err(|e| e.to_string())
                                } else {
                                    Err("Failover succeeded but no plugin available".to_string())
                                }
                            }
                            Err(failover_err) => {
                                error!("Failover failed: {}", failover_err);
                                Err(format!("STT processing failed: {}, failover failed: {}", e, failover_err))
                            }
                        }
                    } else {
                        Err(e.to_string())
                    }
                }
            }
        } else {
            Err("No STT plugin selected".to_string())
        }
    }
    
    /// Attempt to failover to a different plugin
    async fn attempt_failover(&mut self, failed_plugin_id: &str) -> Result<String, String> {
        let registry = self.registry.read().await;
        let now = Instant::now();
        
        // Get cooldown period
        let cooldown_secs = self.selection_config.failover
            .as_ref()
            .map_or(30, |f| f.failover_cooldown_secs);
        let cooldown_duration = std::time::Duration::from_secs(cooldown_secs as u64);
        
        // Check cooldown for failed plugins
        let cooldown = self.failed_plugins_cooldown.read().await;
        
        // Try fallback plugins in order, skipping ones in cooldown
        for fallback_id in &self.selection_config.fallback_plugins {
            if fallback_id == failed_plugin_id {
                continue; // Skip the failed plugin
            }
            
            // Check if plugin is in cooldown
            if let Some(last_failure) = cooldown.get(fallback_id) {
                if now.duration_since(*last_failure) < cooldown_duration {
                    debug!("Plugin {} still in cooldown, skipping", fallback_id);
                    continue;
                }
            }
            
            match registry.create_plugin(fallback_id) {
                Ok(new_plugin) => {
                    let new_plugin_id = new_plugin.info().id.clone();
                    
                    // Replace current plugin
                    {
                        let mut current = self.current_plugin.write().await;
                        *current = Some(new_plugin);
                    }
                    
                    // Reset error count for new plugin
                    {
                        let mut errors = self.consecutive_errors.write().await;
                        errors.remove(&new_plugin_id);
                    }
                    
                    return Ok(new_plugin_id);
                }
                Err(e) => {
                    debug!("Fallback plugin {} not available: {}", fallback_id, e);
                }
            }
        }
        
        // Last resort: NoOp plugin
        let noop_plugin = Box::new(NoOpPlugin::new());
        let noop_id = noop_plugin.info().id.clone();
        
        {
            let mut current = self.current_plugin.write().await;
            *current = Some(noop_plugin);
        }
        
        warn!("All fallback plugins failed, using NoOp plugin");
        Ok(noop_id)
    }
    
    /// Get current failover metrics
    pub fn get_metrics(&self) -> (u64, u64) {
        let failover_count = self.failover_count.load(std::sync::atomic::Ordering::Relaxed);
        let total_errors = self.total_errors.load(std::sync::atomic::Ordering::Relaxed);
        (failover_count, total_errors)
    }

    /// Get Instant of last failover (if any)
    pub async fn last_failover_instant(&self) -> Option<Instant> {
        *self.last_failover.read().await
    }
}

impl Drop for SttPluginManager {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.gc_task.try_write() {
            if let Some(handle) = guard.take() {
                handle.abort();
            }
        }
    }
}

/// Example usage in tests or examples
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_plugin_manager_initialization() {
        let mut manager = SttPluginManager::new();

        // Should initialize with some plugin (at least NoOp)
        let plugin_id = manager.initialize().await.unwrap();
        assert!(!plugin_id.is_empty());

        // Should be able to list plugins
        let plugins = manager.list_plugins().await;
        assert!(!plugins.is_empty());

        // At minimum, NoOp and Mock should be available
        let plugin_ids: Vec<String> = plugins.iter().map(|p| p.id.clone()).collect();
        assert!(plugin_ids.contains(&"noop".to_string()));
        assert!(plugin_ids.contains(&"mock".to_string()));
    }

    #[tokio::test]
    async fn test_plugin_switching() {
        let mut manager = SttPluginManager::new();
        manager.initialize().await.unwrap();

        // Should be able to switch to mock plugin
        manager.switch_plugin("mock").await.unwrap();
        assert_eq!(manager.current_plugin().await, Some("mock".to_string()));

        // Should be able to switch to noop plugin
        manager.switch_plugin("noop").await.unwrap();
        assert_eq!(manager.current_plugin().await, Some("noop".to_string()));
    }

    #[tokio::test]
    async fn test_fallback_to_noop() {
        let mut manager = SttPluginManager::new();

        // Configure to prefer a non-existent plugin
        manager.selection_config.preferred_plugin = Some("non-existent".to_string());
        manager.selection_config.fallback_plugins = vec!["also-non-existent".to_string()];

        // Should fall back to NoOp
        let plugin_id = manager.initialize().await.unwrap();
        assert_eq!(plugin_id, "noop");
    }
}
