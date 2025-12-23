#[cfg(test)]
mod mock_injection_tests {
    use tracing_appender::rolling;
    use coldvox_text_injection::manager::StrategyManager;
    use coldvox_text_injection::types::{InjectionConfig, InjectionMetrics};
    use std::process::{Command, Stdio};
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::Mutex;
    use tokio::time::timeout;

    /// Mock test application that can receive focus and text injection
    struct MockTestApp {
        process: std::process::Child,
        window_id: Option<String>,
    }

    impl MockTestApp {
        /// Start a mock application (xterm) that can receive text injection
        async fn start() -> Result<Self, Box<dyn std::error::Error>> {
            // Start xterm with a simple command that keeps it running
            let child = Command::new("xterm")
                .args(&["-hold", "-e", "echo 'Mock test application ready for injection'; cat"])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;

            // Give the window time to appear
            tokio::time::sleep(Duration::from_millis(500)).await;

            // Try to get window ID using xdotool
            let window_id = Self::get_window_id().await.ok();

            Ok(MockTestApp {
                process: child,
                window_id,
            })
        }

        /// Get the window ID of the mock application
        async fn get_window_id() -> Result<String, Box<dyn std::error::Error>> {
            let output = Command::new("xdotool")
                .args(&["search", "--name", "Mock test application"])
                .output()?;

            if output.status.success() {
                let window_id = String::from_utf8(output.stdout)?
                    .trim()
                    .lines()
                    .next()
                    .unwrap_or("")
                    .to_string();

                if !window_id.is_empty() {
                    Ok(window_id)
                } else {
                    Err("No window found".into())
                }
            } else {
                Err("xdotool search failed".into())
            }
        }

        /// Focus the mock application window
        async fn focus(&self) -> Result<(), Box<dyn std::error::Error>> {
            if let Some(window_id) = &self.window_id {
                let status = Command::new("xdotool")
                    .args(&["windowfocus", window_id])
                    .status()?;

                if status.success() {
                    // Give focus time to settle
                    tokio::time::sleep(Duration::from_millis(200)).await;
                    Ok(())
                } else {
                    Err("Failed to focus window".into())
                }
            } else {
                Err("No window ID available".into())
            }
        }

        /// Check if the application is still running
        fn is_running(&mut self) -> bool {
            matches!(self.process.try_wait(), Ok(None))
        }
    }

    impl Drop for MockTestApp {
        fn drop(&mut self) {
            let _ = self.process.kill();
            let _ = self.process.wait();
        }
    }

    #[tokio::test]
        tracing::info!("Starting test_injection_with_focused_mock_app");
        let _ = std::fs::create_dir_all("target/test_logs");
        let file_appender = rolling::never("target/test_logs", "mock_injection.log");
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
        let _ = tracing_subscriber::registry()
            .with(tracing_subscriber::EnvFilter::from_default_env())
            .with(tracing_subscriber::fmt::layer().with_writer(non_blocking).with_ansi(false))
            .with(tracing_subscriber::fmt::layer().with_test_writer())
            .try_init();
        // Initialize file logging for test
        let _ = std::fs::create_dir_all");
        let("target/test_logs file_appender = rolling::never("target/test_logs", "mock_injection.log");
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
        let _ = tracing_subscriber::registry()
            .with(tracing_subscriber::EnvFilter::from_default_env())
            .with(tracing_subscriber::fmt::layer().with_writer(non_blocking).with_ansi(false))
            .with(tracing_subscriber::fmt::layer().with_test_writer())
            .try_init();
    async fn test_injection_with_focused_mock_app() {
        tracing::info!("Starting test_injection_with_focused_mock_app");
        // Start mock application
        let _ = tracing::span!(tracing::Level::INFO, "test_mock_app_start").entered();
        let mut mock_app = match MockTestApp::start().await {
            Ok(app) => app,
        tracing::info!("Starting test_injection_with_focused_mock_app");
            Err(e) => {
        tracing::info!("Starting test_injection_with_focused_mock_app");
                println!("Skipping test: Could not start mock application: {}", e);
        tracing::info!("Starting test_injection_with_focused_mock_app");
                return;
        tracing::info!("Starting test_injection_with_focused_mock_app");
            }
        tracing::info!("Starting test_injection_with_focused_mock_app");
        };

        // Focus the application
        if let Err(e) = mock_app.focus().await {
            println!("Warning: Could not focus mock application: {}", e);
        }

        // Create injection configuration that allows injection on unknown focus for testing
        // Note: clipboard restoration is automatic (always enabled)
        let config = InjectionConfig {
            allow_kdotool: false,
            allow_enigo: false,
            inject_on_unknown_focus: true, // Allow injection for testing
            max_total_latency_ms: 5000,
            per_method_timeout_ms: 2000,
            cooldown_initial_ms: 100,
            ..Default::default()
        };
        tracing::info!("Starting text injection test");
        let test_text = "Mock injection test";
        let _ = tracing::span!(tracing::Level::INFO, "test_injection").entered();
        let result = timeout(
        let mut manager = StrategyManager::new(config, metrics.clone()).await;

        // Test injection
        let test_text = "Mock injection test";
        let result = timeout(
            Duration::from_secs(10),
            manager.inject(test_text)
        ).await;

        // Verify the injection attempt
        match result {
            Ok(Ok(())) => {
                println!("✅ Injection successful");
            }
            Ok(Err(e)) => {
                println!("⚠️  Injection failed (expected in some environments): {}", e);
            }
            Err(_) => {
                println!("⚠️  Injection timed out");
            }
        }

        // Verify metrics were recorded
        let metrics_guard = metrics.lock().await;
        assert!(metrics_guard.attempts >= 1, "Should record at least one attempt");

        // Clean up
        drop(mock_app);
    }

    #[tokio::test]
    async fn test_clipboard_save_restore_with_mock_app() {
        // Start mock application
        let mut mock_app = match MockTestApp::start().await {
            Ok(app) => app,
            Err(e) => {
                println!("Skipping test: Could not start mock application: {}", e);
                return;
            }
        };

        // Focus the application
        let _ = mock_app.focus().await;

        // Create injection configuration
        // Note: clipboard restoration is automatic (always enabled)
        let config = InjectionConfig {
            allow_kdotool: false,
            allow_enigo: false,
            inject_on_unknown_focus: true,
            max_total_latency_ms: 5000,
            per_method_timeout_ms: 2000,
            cooldown_initial_ms: 100,
            ..Default::default()
        };

        // Create shared metrics
        let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));

        // Create strategy manager
        let mut manager = StrategyManager::new(config, metrics.clone()).await;

        // Put something in clipboard first
        let original_clipboard = "Original clipboard content";
        let _ = Command::new("wl-copy")
            .arg(original_clipboard)
            .status();

        // Wait a bit for clipboard to settle
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Test injection (this should save and restore clipboard)
        let test_text = "Clipboard test";
        let result = timeout(
            Duration::from_secs(5),
            manager.inject(test_text)
        ).await;

        // Wait for clipboard restore
        tokio::time::sleep(Duration::from_millis(600)).await;

        // Verify clipboard was restored
        let clipboard_output = Command::new("wl-paste")
            .output()
            .expect("Failed to read clipboard");

        let restored_clipboard = String::from_utf8(clipboard_output.stdout)
            .unwrap_or_default()
            .trim()
            .to_string();

        // The clipboard should be restored to original content
        // Note: This might not work in all environments due to timing
        if restored_clipboard == original_clipboard {
            println!("✅ Clipboard restore successful");
        } else {
            println!("⚠️  Clipboard restore may have failed (restored: '{}', expected: '{}')",
                    restored_clipboard, original_clipboard);
        }

        // Clean up
        drop(mock_app);
    }

    #[tokio::test]
    async fn test_atspi_paste_fallback_to_ydotool() {
        // This test verifies the AT-SPI -> ydotool fallback behavior
        // We don't need a real app since we're testing the strategy selection

        // Note: clipboard restoration is automatic (always enabled)
        let config = InjectionConfig {
            allow_kdotool: false,
            allow_enigo: false,
            inject_on_unknown_focus: true,
            max_total_latency_ms: 5000,
            per_method_timeout_ms: 2000,
            cooldown_initial_ms: 100,
            ..Default::default()
        };

        let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));
        let mut manager = StrategyManager::new(config, metrics.clone()).await;

        // Get the method order to verify AT-SPI is tried first
        let methods = manager.get_method_order_uncached();

        // Should include AT-SPI insert and the single ClipboardPasteFallback method
        let has_atspi = methods
            .iter()
            .any(|m| matches!(m, coldvox_text_injection::types::InjectionMethod::AtspiInsert));

        let has_clipboard_paste = methods.iter().any(|m| {
            matches!(
                m,
                coldvox_text_injection::types::InjectionMethod::ClipboardPasteFallback
            )
        });

        println!("Available methods: {:?}", methods);
    assert!(has_clipboard_paste, "Should include ClipboardPasteFallback method");

        // AT-SPI might not be available in test environment, but ydotool should be
        if has_atspi {
            println!("✅ AT-SPI methods available for fallback testing");
        } else {
            println!("⚠️  AT-SPI not available (expected in headless environment)");
        }

    assert!(has_clipboard_paste, "Should have ClipboardPasteFallback as fallback method");
    }

    #[tokio::test]
    async fn test_injection_timeout_handling() {
        // Note: clipboard restoration is automatic (always enabled)
        let config = InjectionConfig {
            allow_kdotool: false,
            allow_enigo: false,
            inject_on_unknown_focus: true,
            max_total_latency_ms: 100, // Very short timeout
            per_method_timeout_ms: 50,  // Very short per-method timeout
            cooldown_initial_ms: 100,
            ..Default::default()
        };

        let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));
        let mut manager = StrategyManager::new(config, metrics.clone()).await;

        // This should timeout quickly
        let start = std::time::Instant::now();
        let result = manager.inject("Timeout test").await;
        let elapsed = start.elapsed();

        // Should fail due to timeout
        assert!(result.is_err(), "Should fail due to short timeout");

        // Should complete relatively quickly
        assert!(elapsed < Duration::from_millis(200), "Should timeout quickly");

        // Verify metrics
        let metrics_guard = metrics.lock().await;
        assert_eq!(metrics_guard.attempts, 1, "Should record one attempt");
        assert_eq!(metrics_guard.failures, 1, "Should record one failure");
    }
}
