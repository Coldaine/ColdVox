//! STT Plugin Manager for ColdVox
//!
//! This module manages STT plugin selection and fallback logic

use std::sync::Arc;
use std::time::{Duration, Instant};

use coldvox_stt::plugin::{
    SttPlugin, SttPluginRegistry, PluginSelectionConfig, SttPluginError
};
use coldvox_stt::plugins::{NoOpPlugin, MockPlugin};
use coldvox_telemetry::PipelineMetrics;
use tokio::sync::RwLock;
use tracing::{info, warn, error};

/// Configuration for plugin manager failover behavior
#[derive(Debug, Clone)]
pub struct FailoverConfig {
    /// Maximum number of retries before failover
    pub max_retries: usize,
    /// Cooldown period between failovers (prevents failover storms)
    pub failover_cooldown_secs: u64,
    /// Model TTL in seconds (unload if inactive)
    pub model_ttl_seconds: u64,
    /// Maximum memory usage in MB
    pub max_memory_mb: Option<u64>,
}

impl Default for FailoverConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            failover_cooldown_secs: 2,
            model_ttl_seconds: 300, // 5 minutes
            max_memory_mb: None,
        }
    }
}

/// Manages STT plugin lifecycle and selection
pub struct SttPluginManager {
    registry: Arc<RwLock<SttPluginRegistry>>,
    current_plugin: Arc<RwLock<Option<Box<dyn SttPlugin>>>>,
    selection_config: PluginSelectionConfig,
    failover_config: FailoverConfig,
    metrics: Option<Arc<PipelineMetrics>>,
    last_failover: Arc<RwLock<Option<Instant>>>,
    retry_count: Arc<RwLock<usize>>,
    last_activity: Arc<RwLock<Option<Instant>>>,
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
            failover_config: FailoverConfig::default(),
            metrics: None,
            last_failover: Arc::new(RwLock::new(None)),
            retry_count: Arc::new(RwLock::new(0)),
            last_activity: Arc::new(RwLock::new(None)),
        }
    }

    /// Create with custom configuration
    pub fn with_config(selection_config: PluginSelectionConfig, failover_config: FailoverConfig) -> Self {
        let mut manager = Self::new();
        manager.selection_config = selection_config;
        manager.failover_config = failover_config;
        manager
    }

    /// Set metrics for telemetry
    pub fn set_metrics(&mut self, metrics: Arc<PipelineMetrics>) {
        self.metrics = Some(metrics);
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
            use coldvox_stt::plugins::VoskPluginFactory;
            registry.register(Box::new(VoskPluginFactory::new()));
        }

        // Conditionally register Whisper if feature is enabled
        #[cfg(feature = "whisper")]
        {
            use coldvox_stt::plugins::WhisperPluginFactory;
            registry.register(Box::new(WhisperPluginFactory::new()));
        }

        // Future: Register other plugins
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

    /// Get the current plugin instance (for creating adapters)
    pub async fn current_plugin_instance(&mut self) -> Option<Box<dyn SttPlugin>> {
        let registry = self.registry.read().await;
        let current = self.current_plugin.read().await;
        
        if let Some(ref plugin) = *current {
            let plugin_id = plugin.info().id.clone();
            // Create a new instance of the same plugin type
            registry.create_plugin(&plugin_id).ok()
        } else {
            None
        }
    }

    /// Get information about all available plugins
    pub async fn list_plugins(&self) -> Vec<coldvox_stt::plugin::PluginInfo> {
        let registry = self.registry.read().await;
        registry.available_plugins()
    }
    pub async fn process_audio(
        &mut self,
        samples: &[i16]
    ) -> Result<Option<coldvox_stt::types::TranscriptionEvent>, String> {
        *self.last_activity.write().await = Some(Instant::now());

        let mut attempt = 0;
        let max_retries = self.failover_config.max_retries;

        loop {
            let result = {
                let mut current = self.current_plugin.write().await;
                if let Some(ref mut plugin) = *current {
                    plugin.process_audio(samples).await
                } else {
                    return Err("No STT plugin selected".to_string());
                }
            };

            match result {
                Ok(event) => {
                    // Success - reset retry count
                    *self.retry_count.write().await = 0;
                    return Ok(event);
                }
                Err(e) => {
                    attempt += 1;
                    *self.retry_count.write().await = attempt;

                    if e.is_transient() && attempt <= max_retries {
                        // Transient error - retry with exponential backoff
                        let backoff_ms = (1u64 << attempt.min(6)) * 1000; // 1s, 2s, 4s, 8s, 16s, 32s max
                        warn!("STT transient error (attempt {}): {}. Retrying in {}ms", 
                              attempt, e, backoff_ms);
                        tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                        continue;
                    } else if e.should_failover() {
                        // Permanent error or retry exhausted - try failover
                        warn!("STT error requiring failover: {}", e);
                        if let Err(failover_err) = self.handle_failover().await {
                            error!("Failover failed: {}", failover_err);
                            return Err(format!("STT failed and failover failed: {} -> {}", e, failover_err));
                        }
                        // Reset retry count after successful failover
                        *self.retry_count.write().await = 0;
                        continue;
                    } else {
                        // Non-retryable, non-failover error
                        error!("STT error: {}", e);
                        return Err(e.to_string());
                    }
                }
            }
        }
    }

    /// Handle failover to the next available plugin
    async fn handle_failover(&mut self) -> Result<(), String> {
        // Check cooldown period
        if let Some(last_failover) = *self.last_failover.read().await {
            let elapsed = last_failover.elapsed().as_secs();
            if elapsed < self.failover_config.failover_cooldown_secs {
                return Err(format!("Failover cooldown active ({} seconds remaining)", 
                                 self.failover_config.failover_cooldown_secs - elapsed));
            }
        }

        let current_plugin_id = {
            let current = self.current_plugin.read().await;
            current.as_ref().map(|p| p.info().id.clone())
        };

        info!("Attempting failover from plugin: {:?}", current_plugin_id);

        // Update metrics
        if let Some(ref metrics) = self.metrics {
            metrics.increment_stt_failover();
        }

        // Unload current plugin model to free memory
        if let Some(mut current) = self.current_plugin.write().await.take() {
            if let Err(e) = current.unload_model().await {
                warn!("Failed to unload model during failover: {}", e);
            }
        }

        // Try to find next available plugin in fallback order
        let registry = self.registry.read().await;
        
        // Find next plugin in fallback order
        let fallback_order = &self.selection_config.fallback_plugins;
        let current_index = current_plugin_id.as_ref()
            .and_then(|id| fallback_order.iter().position(|p| p == id))
            .unwrap_or(0);

        // Try remaining plugins in order
        for (idx, plugin_id) in fallback_order.iter().enumerate().skip(current_index + 1) {
            match registry.create_plugin(plugin_id) {
                Ok(plugin) => {
                    info!("Failover successful to plugin: {}", plugin_id);
                    
                    // Update metrics with new backend
                    if let Some(ref metrics) = self.metrics {
                        metrics.set_stt_backend(plugin_id.clone());
                    }

                    // Log structured failover event
                    tracing::warn!(
                        event = "stt_failover", 
                        from = %current_plugin_id.unwrap_or_else(|| "none".to_string()),
                        to = %plugin_id,
                        reason = "plugin_error"
                    );

                    *self.current_plugin.write().await = Some(plugin);
                    *self.last_failover.write().await = Some(Instant::now());
                    return Ok(());
                }
                Err(e) => {
                    warn!("Failover candidate '{}' not available: {}", plugin_id, e);
                }
            }
        }

        // If all fallbacks failed, use NoOp as last resort
        warn!("All failover options exhausted, using NoOp plugin");
        let noop_plugin = Box::new(NoOpPlugin::new());
        
        if let Some(ref metrics) = self.metrics {
            metrics.set_stt_backend("noop".to_string());
        }

        tracing::warn!(
            event = "stt_failover", 
            from = %current_plugin_id.unwrap_or_else(|| "none".to_string()),
            to = "noop",
            reason = "all_plugins_failed"
        );

        *self.current_plugin.write().await = Some(noop_plugin);
        *self.last_failover.write().await = Some(Instant::now());
        
        Ok(())
    }

    /// Check and unload inactive models to manage memory
    pub async fn gc_inactive_models(&mut self) -> Result<(), String> {
        let ttl = Duration::from_secs(self.failover_config.model_ttl_seconds);
        
        if let Some(last_activity) = *self.last_activity.read().await {
            if last_activity.elapsed() > ttl {
                info!("Unloading inactive STT model (TTL: {} seconds)", ttl.as_secs());
                
                let mut current = self.current_plugin.write().await;
                if let Some(ref mut plugin) = *current {
                    if let Err(e) = plugin.unload_model().await {
                        warn!("Failed to unload inactive model: {}", e);
                    } else {
                        info!("Successfully unloaded inactive model");
                    }
                }
            }
        }
        
        Ok(())
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
