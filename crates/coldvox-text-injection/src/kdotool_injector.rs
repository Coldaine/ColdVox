use crate::types::{InjectionConfig, InjectionResult};
use crate::TextInjector;
use async_trait::async_trait;
use coldvox_foundation::error::InjectionError;
use std::process::Command;
use std::time::Duration;
use tokio::time::timeout;
use tracing::info;

/// Kdotool injector for KDE window activation/focus assistance
pub struct KdotoolInjector {
    config: InjectionConfig,
    /// Whether kdotool is available on the system
    is_available: bool,
}

impl KdotoolInjector {
    /// Create a new kdotool injector
    pub fn new(config: InjectionConfig) -> Self {
        let is_available = Self::check_kdotool();

        Self {
            config,
            is_available,
        }
    }

    /// Check if kdotool is available on the system
    fn check_kdotool() -> bool {
        Command::new("which")
            .arg("kdotool")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn binary_available(binary: &str) -> bool {
        Command::new("which")
            .arg(binary)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    #[cfg(test)]
    pub(crate) fn binary_available_for_tests(binary: &str) -> bool {
        Self::binary_available(binary)
    }

    /// Get the currently active window ID
    async fn get_active_window(&self) -> Result<String, InjectionError> {
        let output = timeout(
            Duration::from_millis(self.config.discovery_timeout_ms),
            tokio::process::Command::new("kdotool")
                .arg("getactivewindow")
                .output(),
        )
        .await
        .map_err(|_| InjectionError::Timeout(self.config.discovery_timeout_ms))?
        .map_err(|e| InjectionError::Process(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(InjectionError::MethodFailed(format!(
                "kdotool getactivewindow failed: {}",
                stderr
            )));
        }

        let window_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(window_id)
    }

    /// Activate a window by ID
    async fn activate_window(&self, window_id: &str) -> Result<(), InjectionError> {
        let output = timeout(
            Duration::from_millis(self.config.per_method_timeout_ms),
            tokio::process::Command::new("kdotool")
                .args(["windowactivate", window_id])
                .output(),
        )
        .await
        .map_err(|_| InjectionError::Timeout(self.config.per_method_timeout_ms))?
        .map_err(|e| InjectionError::Process(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(InjectionError::MethodFailed(format!(
                "kdotool windowactivate failed: {}",
                stderr
            )));
        }

        info!("Successfully activated window {}", window_id);

        Ok(())
    }

    /// Focus a window by ID
    async fn focus_window(&self, window_id: &str) -> Result<(), InjectionError> {
        let output = timeout(
            Duration::from_millis(self.config.per_method_timeout_ms),
            tokio::process::Command::new("kdotool")
                .args(["windowfocus", window_id])
                .output(),
        )
        .await
        .map_err(|_| InjectionError::Timeout(self.config.per_method_timeout_ms))?
        .map_err(|e| InjectionError::Process(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(InjectionError::MethodFailed(format!(
                "kdotool windowfocus failed: {}",
                stderr
            )));
        }

        info!("Successfully focused window {}", window_id);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::InjectionConfig;
    use std::env;
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;

    // Helper to create a mock kdotool script
    fn create_mock_kdotool(content: &str) -> (PathBuf, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let script_path = dir.path().join("kdotool");
        let mut file = fs::File::create(&script_path).unwrap();
        writeln!(file, "#!/bin/sh").unwrap();
        writeln!(file, "{}", content).unwrap();
        // Make it executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&script_path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&script_path, perms).unwrap();
        }
        (script_path, dir)
    }

    // RAII guard to modify PATH for the duration of a test
    struct PathGuard {
        original_path: String,
    }

    impl PathGuard {
        fn new(temp_dir: &PathBuf) -> Self {
            let original_path = env::var("PATH").unwrap_or_default();
            let new_path = format!("{}:{}", temp_dir.to_str().unwrap(), original_path);
            env::set_var("PATH", new_path);
            Self { original_path }
        }
    }

    impl Drop for PathGuard {
        fn drop(&mut self) {
            env::set_var("PATH", &self.original_path);
        }
    }

    #[tokio::test]
    async fn test_kdotool_injector_new_available() {
        let (_script, dir) = create_mock_kdotool("exit 0");
        let _guard = PathGuard::new(&dir.path().to_path_buf());

        let config = InjectionConfig::default();
        let injector = KdotoolInjector::new(config);
        assert!(injector.is_available);
    }

    #[tokio::test]
    async fn test_kdotool_injector_new_not_available() {
        let original_path = env::var("PATH").unwrap_or_default();
        env::set_var("PATH", "/tmp/non-existent-dir");

        let config = InjectionConfig::default();
        let injector = KdotoolInjector::new(config);
        assert!(!injector.is_available);

        env::set_var("PATH", original_path);
    }

    #[tokio::test]
    async fn test_get_active_window_success() {
        let (_script, dir) = create_mock_kdotool("echo '12345'");
        let _guard = PathGuard::new(&dir.path().to_path_buf());

        let config = InjectionConfig::default();
        let injector = KdotoolInjector::new(config);
        let window_id = injector.get_active_window().await.unwrap();
        assert_eq!(window_id, "12345");
    }

    #[tokio::test]
    async fn test_get_active_window_failure() {
        let (_script, dir) = create_mock_kdotool("echo 'Error' >&2; exit 1");
        let _guard = PathGuard::new(&dir.path().to_path_buf());

        let config = InjectionConfig::default();
        let injector = KdotoolInjector::new(config);
        let result = injector.get_active_window().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_activate_window_success() {
        let (_script, dir) = create_mock_kdotool("exit 0");
        let _guard = PathGuard::new(&dir.path().to_path_buf());

        let config = InjectionConfig::default();
        let injector = KdotoolInjector::new(config);
        let result = injector.activate_window("12345").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_focus_window_success() {
        let (_script, dir) = create_mock_kdotool("exit 0");
        let _guard = PathGuard::new(&dir.path().to_path_buf());

        let config = InjectionConfig::default();
        let injector = KdotoolInjector::new(config);
        let result = injector.focus_window("12345").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_ensure_focus_with_id() {
        let script_content = r#"
            if [ "$1" = "windowfocus" ]; then
                echo "focused"
            elif [ "$1" = "windowactivate" ]; then
                echo "activated"
            fi
        "#;
        let (_script, dir) = create_mock_kdotool(script_content);
        let _guard = PathGuard::new(&dir.path().to_path_buf());

        let config = InjectionConfig::default();
        let injector = KdotoolInjector::new(config);
        let result = injector.ensure_focus(Some("12345")).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_ensure_focus_no_id() {
        let script_content = r#"
            if [ "$1" = "getactivewindow" ]; then
                echo "67890"
            elif [ "$1" = "windowfocus" ]; then
                exit 0
            elif [ "$1" = "windowactivate" ]; then
                exit 0
            fi
        "#;
        let (_script, dir) = create_mock_kdotool(script_content);
        let _guard = PathGuard::new(&dir.path().to_path_buf());

        let config = InjectionConfig::default();
        let injector = KdotoolInjector::new(config);
        let result = injector.ensure_focus(None).await;
        assert!(result.is_ok());
    }
}

#[async_trait]
impl TextInjector for KdotoolInjector {
    fn backend_name(&self) -> &'static str {
        "Kdotool"
    }

    async fn is_available(&self) -> bool {
        self.is_available && self.config.allow_kdotool
    }

    async fn inject_text(
        &self,
        _text: &str,
        _context: Option<&crate::types::InjectionContext>,
    ) -> InjectionResult<()> {
        // Kdotool is only used for window activation/focus assistance
        // It doesn't actually inject text, so this method should not be called
        // directly for text injection
        Err(InjectionError::MethodUnavailable(
            "Kdotool is only for window activation/focus assistance".to_string(),
        ))
    }

    fn backend_info(&self) -> Vec<(&'static str, String)> {
        vec![
            ("type", "window management".to_string()),
            (
                "description",
                "Provides window activation and focus assistance using kdotool".to_string(),
            ),
            ("platform", "KDE/X11".to_string()),
            ("requires", "kdotool command line tool".to_string()),
        ]
    }
}

impl KdotoolInjector {
    /// Ensure the target window is active and focused
    pub async fn ensure_focus(&self, window_id: Option<&str>) -> Result<(), InjectionError> {
        let target_window = match window_id {
            Some(id) => id.to_string(),
            None => self.get_active_window().await?,
        };

        // First focus the window
        self.focus_window(&target_window).await?;

        // Then activate it
        self.activate_window(&target_window).await?;

        Ok(())
    }
}
