//! STT Plugin Manager for ColdVox
//! 
//! This module manages STT plugin selection and fallback logic

use coldvox_stt::plugin::{
    SttPlugin, SttPluginRegistry, PluginSelectionConfig, SttPluginError
};
use coldvox_stt::plugins::{NoOpPlugin, MockPlugin};
use tracing::{info, warn, error};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Manages STT plugin lifecycle and selection
pub struct SttPluginManager {
    registry: Arc<RwLock<SttPluginRegistry>>,
    current_plugin: Arc<RwLock<Option<Box<dyn SttPlugin>>>>,
    selection_config: PluginSelectionConfig,
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
        }
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
    
    /// Process audio with the current plugin
    pub async fn process_audio(
        &mut self,
        samples: &[i16]
    ) -> Result<Option<coldvox_stt::types::TranscriptionEvent>, String> {
        let mut current = self.current_plugin.write().await;
        
        if let Some(ref mut plugin) = *current {
            plugin.process_audio(samples)
                .await
                .map_err(|e| e.to_string())
        } else {
            Err("No STT plugin selected".to_string())
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