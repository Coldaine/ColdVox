//! # Compatibility Module
//!
//! This module provides simple JSON compatibility memory for the ColdVox text injection system.
//! It maintains backward compatibility with legacy configuration formats and provides
//! migration utilities for newer versions.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Legacy configuration format version 1
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyConfigV1 {
    /// List of enabled injection methods
    pub enabled_methods: Vec<String>,
    /// Timeout configuration
    pub timeout_ms: u64,
    /// Focus requirements
    pub require_focus: bool,
    /// Application allowlist
    pub allowlist: Vec<String>,
    /// Application blocklist
    pub blocklist: Vec<String>,
}

/// Legacy configuration format version 2
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyConfigV2 {
    /// Method-specific configurations
    pub methods: HashMap<String, LegacyMethodConfig>,
    /// Global timeout configuration
    pub global_timeout_ms: u64,
    /// Focus configuration
    pub focus_config: LegacyFocusConfig,
    /// Application filtering
    pub app_filter: LegacyAppFilter,
    /// Performance settings
    pub performance: LegacyPerformanceConfig,
}

/// Legacy method configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyMethodConfig {
    /// Whether this method is enabled
    pub enabled: bool,
    /// Method-specific timeout
    pub timeout_ms: Option<u64>,
    /// Method-specific priority
    pub priority: i32,
    /// Additional method parameters
    pub params: HashMap<String, serde_json::Value>,
}

/// Legacy focus configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyFocusConfig {
    /// Whether focus is required
    pub require_focus: bool,
    /// Whether to inject on unknown focus
    pub inject_on_unknown_focus: bool,
    /// Focus check interval in milliseconds
    pub check_interval_ms: u64,
}

/// Legacy application filter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyAppFilter {
    /// Allowlist patterns
    pub allowlist: Vec<String>,
    /// Blocklist patterns
    pub blocklist: Vec<String>,
    /// Whether to use regex patterns
    pub use_regex: bool,
}

/// Legacy performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyPerformanceConfig {
    /// Maximum number of retries
    pub max_retries: u32,
    /// Retry delay in milliseconds
    pub retry_delay_ms: u64,
    /// Cooldown settings
    pub cooldown: LegacyCooldownConfig,
}

/// Legacy cooldown configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyCooldownConfig {
    /// Initial cooldown in milliseconds
    pub initial_ms: u64,
    /// Maximum cooldown in milliseconds
    pub max_ms: u64,
    /// Backoff multiplier
    pub backoff_factor: f32,
}

/// Current configuration format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentConfig {
    /// Injection configuration
    pub injection: crate::types::InjectionConfig,
    /// Logging configuration
    pub logging: crate::logging::LoggingConfig,
    /// Additional metadata
    pub metadata: ConfigMetadata,
}

/// Configuration metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigMetadata {
    /// Configuration version
    pub version: String,
    /// Last migration date
    pub last_migrated: Option<chrono::DateTime<chrono::Utc>>,
    /// Original format version
    pub original_format: Option<String>,
    /// Migration notes
    pub migration_notes: Vec<String>,
}

/// Compatibility memory that stores migration history and compatibility information
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompatibilityMemory {
    /// Known legacy configurations
    pub legacy_configs: HashMap<String, LegacyConfigInfo>,
    /// Migration history
    pub migration_history: Vec<MigrationEntry>,
    /// Compatibility rules
    pub compatibility_rules: Vec<CompatibilityRule>,
}

/// Information about a legacy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyConfigInfo {
    /// Configuration path
    pub path: PathBuf,
    /// Format version
    pub version: String,
    /// Last modified timestamp
    pub last_modified: chrono::DateTime<chrono::Utc>,
    /// Whether migration was successful
    pub migrated: bool,
    /// Migration errors, if any
    pub migration_errors: Vec<String>,
}

/// Migration entry in the history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationEntry {
    /// Migration timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Source path
    pub source_path: PathBuf,
    /// Target path
    pub target_path: PathBuf,
    /// Source version
    pub source_version: String,
    /// Target version
    pub target_version: String,
    /// Migration status
    pub status: MigrationStatus,
    /// Migration messages
    pub messages: Vec<String>,
}

/// Migration status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MigrationStatus {
    /// Migration completed successfully
    Success,
    /// Migration failed
    Failed,
    /// Migration completed with warnings
    Warning,
    /// Migration was skipped
    Skipped,
}

/// Compatibility rule for handling specific configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityRule {
    /// Rule name
    pub name: String,
    /// Source version pattern (regex)
    pub source_version_pattern: String,
    /// Rule type
    pub rule_type: CompatibilityRuleType,
    /// Rule configuration
    pub config: HashMap<String, serde_json::Value>,
    /// Whether this rule is enabled
    pub enabled: bool,
}

/// Compatibility rule type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompatibilityRuleType {
    /// Field mapping rule
    FieldMapping,
    /// Value transformation rule
    ValueTransformation,
    /// Default value injection rule
    DefaultInjection,
    /// Validation rule
    Validation,
    /// Custom transformation rule
    Custom,
}

impl CompatibilityMemory {
    /// Create a new compatibility memory instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a legacy configuration to the memory
    pub fn add_legacy_config(
        &mut self,
        path: PathBuf,
        version: String,
        _config: &serde_json::Value,
    ) -> Result<()> {
        let info = LegacyConfigInfo {
            path: path.clone(),
            version: version.clone(),
            last_modified: chrono::Utc::now(),
            migrated: false,
            migration_errors: Vec::new(),
        };

        self.legacy_configs
            .insert(path.to_string_lossy().to_string(), info);
        Ok(())
    }

    /// Record a migration attempt
    pub fn record_migration(&mut self, entry: MigrationEntry) {
        self.migration_history.push(entry);
    }

    /// Add a compatibility rule
    pub fn add_compatibility_rule(&mut self, rule: CompatibilityRule) {
        self.compatibility_rules.push(rule);
    }

    /// Get migration history for a specific path
    pub fn get_migration_history(&self, path: &PathBuf) -> Vec<&MigrationEntry> {
        self.migration_history
            .iter()
            .filter(|entry| entry.source_path == *path || entry.target_path == *path)
            .collect()
    }

    /// Check if a configuration has been migrated
    pub fn is_migrated(&self, path: &Path) -> bool {
        self.legacy_configs
            .get(&path.to_string_lossy().to_string())
            .map(|info| info.migrated)
            .unwrap_or(false)
    }

    /// Get compatible rules for a version
    pub fn get_compatible_rules(&self, version: &str) -> Vec<&CompatibilityRule> {
        self.compatibility_rules
            .iter()
            .filter(|rule| {
                if !rule.enabled {
                    return false;
                }
                // Simple pattern matching - in a real implementation, use regex
                rule.source_version_pattern.contains(version)
                    || version.contains(&rule.source_version_pattern)
            })
            .collect()
    }
}

/// Migration utilities
pub mod migration {
    use super::*;

    /// Migrate from legacy V1 to current configuration
    pub fn migrate_v1_to_current(legacy: LegacyConfigV1) -> Result<CurrentConfig> {
        let injection_config = crate::types::InjectionConfig {
            allow_kdotool: legacy.enabled_methods.contains(&"kdotool".to_string()),
            allow_enigo: legacy.enabled_methods.contains(&"enigo".to_string()),
            max_total_latency_ms: legacy.timeout_ms,
            per_method_timeout_ms: legacy.timeout_ms / 4, // Quarter of total timeout
            require_focus: legacy.require_focus,
            allowlist: legacy.allowlist,
            blocklist: legacy.blocklist,
            ..Default::default()
        };

        let current_config = CurrentConfig {
            injection: injection_config,
            logging: crate::logging::LoggingConfig::default(),
            metadata: ConfigMetadata {
                version: "3.0.0".to_string(),
                last_migrated: Some(chrono::Utc::now()),
                original_format: Some("1.0".to_string()),
                migration_notes: vec![
                    "Migrated from legacy V1 format".to_string(),
                    "Default values applied for new configuration options".to_string(),
                ],
            },
        };

        Ok(current_config)
    }

    /// Migrate from legacy V2 to current configuration
    pub fn migrate_v2_to_current(legacy: LegacyConfigV2) -> Result<CurrentConfig> {
        // Build injection config with all fields initialized at once
        let mut injection_config = crate::types::InjectionConfig {
            max_total_latency_ms: legacy.global_timeout_ms,
            require_focus: legacy.focus_config.require_focus,
            inject_on_unknown_focus: legacy.focus_config.inject_on_unknown_focus,
            allowlist: legacy.app_filter.allowlist,
            blocklist: legacy.app_filter.blocklist,
            cooldown_initial_ms: legacy.performance.cooldown.initial_ms,
            cooldown_max_ms: legacy.performance.cooldown.max_ms,
            cooldown_backoff_factor: legacy.performance.cooldown.backoff_factor,
            ..Default::default()
        };

        // Map method-specific configurations
        for (method_name, method_config) in &legacy.methods {
            match method_name.as_str() {
                "kdotool" => injection_config.allow_kdotool = method_config.enabled,
                "enigo" => injection_config.allow_enigo = method_config.enabled,
                _ => {} // Unknown method, skip
            }

            // Apply method-specific timeout if available
            if let Some(timeout) = method_config.timeout_ms {
                // In a real implementation, you might want to store this in a method-specific config
                if timeout < injection_config.per_method_timeout_ms {
                    injection_config.per_method_timeout_ms = timeout;
                }
            }
        }

        let current_config = CurrentConfig {
            injection: injection_config,
            logging: crate::logging::LoggingConfig::default(),
            metadata: ConfigMetadata {
                version: "3.0.0".to_string(),
                last_migrated: Some(chrono::Utc::now()),
                original_format: Some("2.0".to_string()),
                migration_notes: vec![
                    "Migrated from legacy V2 format".to_string(),
                    "Method-specific configurations preserved".to_string(),
                    "Performance settings migrated".to_string(),
                ],
            },
        };

        Ok(current_config)
    }

    /// Detect legacy configuration format version
    pub fn detect_config_version(config: &serde_json::Value) -> Result<String> {
        // Check for V1 format indicators
        if config.get("enabled_methods").is_some()
            && config.get("timeout_ms").is_some()
            && config.get("require_focus").is_some()
        {
            return Ok("1.0".to_string());
        }

        // Check for V2 format indicators
        if config.get("methods").is_some()
            && config.get("global_timeout_ms").is_some()
            && config.get("focus_config").is_some()
        {
            return Ok("2.0".to_string());
        }

        // Check for current format
        if config.get("injection").is_some() || config.get("logging").is_some() {
            return Ok("3.0".to_string());
        }

        Err(anyhow::anyhow!("Unknown configuration format"))
    }

    /// Auto-migrate a configuration to the current format
    pub fn auto_migrate(config: &serde_json::Value) -> Result<CurrentConfig> {
        let version = detect_config_version(config)?;

        match version.as_str() {
            "1.0" => {
                let legacy: LegacyConfigV1 = serde_json::from_value(config.clone())?;
                migrate_v1_to_current(legacy)
            }
            "2.0" => {
                let legacy: LegacyConfigV2 = serde_json::from_value(config.clone())?;
                migrate_v2_to_current(legacy)
            }
            "3.0" => {
                let current: CurrentConfig = serde_json::from_value(config.clone())?;
                Ok(current)
            }
            _ => Err(anyhow::anyhow!(
                "Unsupported configuration version: {}",
                version
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_legacy_v1_migration() {
        let legacy = LegacyConfigV1 {
            enabled_methods: vec!["kdotool".to_string(), "enigo".to_string()],
            timeout_ms: 1000,
            require_focus: true,
            allowlist: vec!["test_app".to_string()],
            blocklist: vec!["blocked_app".to_string()],
        };

        let current = migration::migrate_v1_to_current(legacy).unwrap();

        assert!(current.injection.allow_kdotool);
        assert!(current.injection.allow_enigo);
        assert_eq!(current.injection.max_total_latency_ms, 1000);
        assert!(current.injection.require_focus);
        assert_eq!(current.injection.allowlist.len(), 1);
        assert_eq!(current.injection.blocklist.len(), 1);
        assert_eq!(current.metadata.original_format, Some("1.0".to_string()));
    }

    #[test]
    fn test_legacy_v2_migration() {
        let mut methods = HashMap::new();
        methods.insert(
            "kdotool".to_string(),
            LegacyMethodConfig {
                enabled: true,
                timeout_ms: Some(500),
                priority: 1,
                params: HashMap::new(),
            },
        );

        let legacy = LegacyConfigV2 {
            methods,
            global_timeout_ms: 2000,
            focus_config: LegacyFocusConfig {
                require_focus: false,
                inject_on_unknown_focus: true,
                check_interval_ms: 100,
            },
            app_filter: LegacyAppFilter {
                allowlist: vec!["allowed".to_string()],
                blocklist: vec!["blocked".to_string()],
                use_regex: false,
            },
            performance: LegacyPerformanceConfig {
                max_retries: 3,
                retry_delay_ms: 100,
                cooldown: LegacyCooldownConfig {
                    initial_ms: 5000,
                    max_ms: 60000,
                    backoff_factor: 2.0,
                },
            },
        };

        let current = migration::migrate_v2_to_current(legacy).unwrap();

        assert!(current.injection.allow_kdotool);
        assert_eq!(current.injection.max_total_latency_ms, 2000);
        assert!(!current.injection.require_focus);
        assert!(current.injection.inject_on_unknown_focus);
        assert_eq!(current.injection.cooldown_initial_ms, 5000);
        assert_eq!(current.metadata.original_format, Some("2.0".to_string()));
    }

    #[test]
    fn test_config_version_detection() {
        let v1_config = serde_json::json!({
            "enabled_methods": ["kdotool"],
            "timeout_ms": 1000,
            "require_focus": true
        });

        let v2_config = serde_json::json!({
            "methods": {"kdotool": {"enabled": true}},
            "global_timeout_ms": 2000,
            "focus_config": {"require_focus": false}
        });

        let v3_config = serde_json::json!({
            "injection": {"allow_kdotool": true},
            "logging": {"level": "INFO"}
        });

        assert_eq!(migration::detect_config_version(&v1_config).unwrap(), "1.0");
        assert_eq!(migration::detect_config_version(&v2_config).unwrap(), "2.0");
        assert_eq!(migration::detect_config_version(&v3_config).unwrap(), "3.0");
    }

    #[test]
    fn test_compatibility_memory() {
        let mut memory = CompatibilityMemory::new();
        let path = PathBuf::from("/test/config.json");

        memory
            .add_legacy_config(path.clone(), "1.0".to_string(), &serde_json::json!({}))
            .unwrap();

        assert!(!memory.is_migrated(&path));

        let entry = MigrationEntry {
            timestamp: chrono::Utc::now(),
            source_path: path.clone(),
            target_path: PathBuf::from("/test/config_new.json"),
            source_version: "1.0".to_string(),
            target_version: "3.0".to_string(),
            status: MigrationStatus::Success,
            messages: vec!["Migration successful".to_string()],
        };

        memory.record_migration(entry);

        assert_eq!(memory.get_migration_history(&path).len(), 1);
    }
}
