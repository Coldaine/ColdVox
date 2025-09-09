#[cfg(test)]
pub mod util {
    #![allow(dead_code)]
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use crate::types::InjectionMetrics;
    use crate::{
        FocusProvider, FocusStatus, InjectionConfig, InjectionError, InjectionMethod,
        StrategyManager, TextInjector,
    };
    use async_trait::async_trait;

    #[derive(Default)]
    pub struct TestInjectorFactory {
        entries: Vec<(InjectionMethod, Box<dyn TextInjector>)>,
    }

    impl TestInjectorFactory {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn add_mock(
            mut self,
            method: InjectionMethod,
            should_succeed: bool,
            latency_ms: u64,
        ) -> Self {
            self.entries.push((
                method,
                Box::new(MockInjector {
                    name: "mock",
                    should_succeed,
                    latency_ms,
                }) as Box<dyn TextInjector>,
            ));
            self
        }

        pub fn build(self) -> HashMap<InjectionMethod, Box<dyn TextInjector>> {
            let mut map = HashMap::new();
            for (m, inj) in self.entries {
                map.insert(m, inj);
            }
            map
        }
    }

    pub struct MockInjector {
        pub name: &'static str,
        pub should_succeed: bool,
        pub latency_ms: u64,
    }

    #[async_trait]
    impl TextInjector for MockInjector {
        async fn inject_text(&self, _text: &str) -> crate::types::InjectionResult<()> {
            if self.latency_ms > 0 {
                tokio::time::sleep(Duration::from_millis(self.latency_ms)).await;
            }
            if self.should_succeed {
                Ok(())
            } else {
                Err(InjectionError::MethodFailed("mock fail".into()))
            }
        }
        async fn is_available(&self) -> bool {
            true
        }
        fn backend_name(&self) -> &'static str {
            self.name
        }
        fn backend_info(&self) -> Vec<(&'static str, String)> {
            vec![("type", "mock".into())]
        }
    }

    pub struct MockFocusProvider {
        pub status: FocusStatus,
    }

    #[async_trait]
    impl FocusProvider for MockFocusProvider {
        async fn get_focus_status(&mut self) -> Result<FocusStatus, InjectionError> {
            Ok(self.status)
        }
    }

    pub async fn manager_with_focus(
        config: InjectionConfig,
        status: FocusStatus,
    ) -> StrategyManager {
        let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));
        let focus = Box::new(MockFocusProvider { status });
        StrategyManager::new_with_focus_provider(config, metrics, focus).await
    }

    /// Determines if tests that require real GUI/AT-SPI should be skipped in CI environments.
    ///
    /// This function checks for CI environment indicators and verifies that AT-SPI is actually
    /// responsive, not just that D-Bus is available.
    pub fn skip_if_headless_ci() -> bool {
        // Check for various CI environment indicators
        let is_ci = std::env::var("CI").is_ok()
            || std::env::var("CONTINUOUS_INTEGRATION").is_ok()
            || std::env::var("GITHUB_ACTIONS").is_ok()
            || std::env::var("GITLAB_CI").is_ok()
            || std::env::var("BUILD_NUMBER").is_ok(); // Jenkins

        if is_ci {
            eprintln!("CI environment detected, checking GUI availability...");

            // First check: ensure D-Bus session is available
            if std::env::var("DBUS_SESSION_BUS_ADDRESS").is_err() {
                eprintln!("Skipping: No D-Bus session bus available in CI");
                return true;
            }

            // Second check: ensure display is available
            if std::env::var("DISPLAY").is_err() && std::env::var("WAYLAND_DISPLAY").is_err() {
                eprintln!("Skipping: No display server available in CI");
                return true;
            }

            // Third check: verify AT-SPI is actually responding
            if !is_atspi_responsive() {
                eprintln!("Skipping: AT-SPI service not responsive in CI environment");
                return true;
            }

            eprintln!("CI environment has GUI support, proceeding with test");
        }
        false
    }

    /// Quick check if we can connect to AT-SPI within a short timeout.
    ///
    /// This function tests actual AT-SPI connectivity rather than just checking for
    /// environment variables, preventing tests from hanging when AT-SPI is available
    /// but unresponsive.
    fn is_atspi_responsive() -> bool {
        // Quick check if we can connect to AT-SPI within a short timeout
        match tokio::runtime::Handle::try_current() {
            Ok(handle) => handle.block_on(async {
                tokio::time::timeout(Duration::from_millis(1000), test_atspi_connection())
                    .await
                    .unwrap_or(false)
            }),
            Err(_) => {
                // No tokio runtime available, assume unresponsive
                false
            }
        }
    }

    /// Minimal AT-SPI connection test that times out quickly.
    ///
    /// This function attempts a basic AT-SPI connection without any complex operations
    /// to verify that the service is responsive.
    async fn test_atspi_connection() -> bool {
        #[cfg(feature = "atspi")]
        {
            use tokio::time;

            // Try to create a connection with a very short timeout
            match time::timeout(
                Duration::from_millis(500),
                atspi::connection::AccessibilityConnection::new(),
            )
            .await
            {
                Ok(Ok(_)) => true,
                _ => false,
            }
        }
        #[cfg(not(feature = "atspi"))]
        {
            // If AT-SPI feature is not enabled, consider it "responsive"
            // (tests will be skipped for other reasons)
            true
        }
    }

    /// Determines if the current environment can run real injection tests.
    ///
    /// This is a more comprehensive check that considers display availability,
    /// CI environment, and AT-SPI responsiveness.
    pub fn can_run_real_tests() -> bool {
        // Check for display server
        let has_display =
            std::env::var("DISPLAY").is_ok() || std::env::var("WAYLAND_DISPLAY").is_ok();

        if !has_display {
            eprintln!("Skipping: No display server available");
            return false;
        }

        // Check CI-specific conditions
        if skip_if_headless_ci() {
            return false;
        }

        true
    }
}
