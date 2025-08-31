use crate::types::{InjectionConfig, InjectionError};
use std::time::{Duration, Instant};
use tracing::debug;

/// Status of current focus in the system
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusStatus {
    /// Focus is on an editable text element
    EditableText,
    /// Focus is on a non-editable element
    NonEditable,
    /// Focus status is unknown or could not be determined
    Unknown,
}

/// Tracks the current focused element for text injection targeting
pub struct FocusTracker {
    _config: InjectionConfig,
    last_check: Option<Instant>,
    cached_status: Option<FocusStatus>,
    cache_duration: Duration,
}

impl FocusTracker {
    /// Create a new focus tracker
    pub fn new(config: InjectionConfig) -> Self {
        let cache_duration = Duration::from_millis(config.focus_cache_duration_ms);
        Self {
            _config: config,
            last_check: None,
            cached_status: None,
            cache_duration,
        }
    }

    /// Get the current focus status
    pub async fn get_focus_status(&mut self) -> Result<FocusStatus, InjectionError> {
        // Check if we have a valid cached result
        if let (Some(last_check), Some(status)) = (self.last_check, self.cached_status) {
            if last_check.elapsed() < self.cache_duration {
                debug!("Using cached focus status: {:?}", status);
                return Ok(status);
            }
        }

        // Get fresh focus status
        let status = self.check_focus_status().await?;
        
        // Cache the result
        self.last_check = Some(Instant::now());
        self.cached_status = Some(status);
        
        debug!("Focus status determined: {:?}", status);
        Ok(status)
    }

    /// Check the actual focus status
    async fn check_focus_status(&self) -> Result<FocusStatus, InjectionError> {
        #[cfg(feature = "atspi")]
        {
            // TODO: Implement real AT-SPI focus detection once API is stable
            // For now, return a reasonable default
            debug!("AT-SPI focus detection placeholder - returning Unknown");
            return Ok(FocusStatus::Unknown);
        }
        
        #[cfg(not(feature = "atspi"))]
        {
            // Fallback: Without AT-SPI, we can't reliably determine focus
            debug!("AT-SPI not available, returning unknown focus status");
            Ok(FocusStatus::Unknown)
        }
    }

    /// Clear the focus cache (useful when window focus changes)
    pub fn clear_cache(&mut self) {
        self.last_check = None;
        self.cached_status = None;
        debug!("Focus cache cleared");
    }

    /// Get the cached focus status without checking
    pub fn cached_focus_status(&self) -> Option<FocusStatus> {
        self.cached_status
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_focus_tracker_creation() {
        let config = InjectionConfig::default();
        let tracker = FocusTracker::new(config);
        
        assert!(tracker.cached_focus_status().is_none());
    }

    #[tokio::test]
    async fn test_focus_status_caching() {
        let config = InjectionConfig::default();
        let mut tracker = FocusTracker::new(config);
        
        // First check should not use cache
        let status1 = tracker.get_focus_status().await.unwrap();
        assert!(tracker.cached_focus_status().is_some());
        
        // Second check should use cache
        let status2 = tracker.get_focus_status().await.unwrap();
        assert_eq!(status1, status2);
    }

    #[test]
    fn test_cache_clearing() {
        let config = InjectionConfig::default();
        let mut tracker = FocusTracker::new(config);
        
        // Manually set cache
        tracker.cached_status = Some(FocusStatus::EditableText);
        tracker.last_check = Some(Instant::now());
        
        assert!(tracker.cached_focus_status().is_some());
        
        // Clear cache
        tracker.clear_cache();
        assert!(tracker.cached_focus_status().is_none());
    }
}