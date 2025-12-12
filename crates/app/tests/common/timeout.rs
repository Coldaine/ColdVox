/// Timeout utilities for test execution to prevent hanging in CI/headless environments.
///
/// Provides configurable timeouts for long-running test operations with clear error messages.
use std::time::Duration;
use tokio::time::timeout;

/// Default timeout for most test operations (10 seconds)
pub const DEFAULT_TEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Extended timeout for complex operations like STT model loading (30 seconds)
pub const EXTENDED_TEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Short timeout for quick operations that shouldn't hang (5 seconds)
pub const SHORT_TEST_TIMEOUT: Duration = Duration::from_secs(5);

/// Wrap an async operation with a timeout and provide clear error messaging.
///
/// # Arguments
/// * `future` - The async operation to execute
/// * `timeout_duration` - Maximum time to wait (uses DEFAULT_TEST_TIMEOUT if None)
/// * `operation_name` - Description of the operation for error messages
///
/// # Returns
/// * `Result<T, String>` - Success value or timeout error message
///
/// # Examples
/// ```
/// use crate::timeout::{with_timeout, DEFAULT_TEST_TIMEOUT};
///
/// let result = with_timeout(
///     async_operation(),
///     Some(DEFAULT_TEST_TIMEOUT),
///     "test async operation"
/// ).await?;
/// ```
pub async fn with_timeout<F, T>(
    future: F,
    timeout_duration: Option<Duration>,
    operation_name: &str,
) -> Result<T, String>
where
    F: std::future::Future<Output = T>,
{
    let timeout_duration = timeout_duration.unwrap_or(DEFAULT_TEST_TIMEOUT);

    match timeout(timeout_duration, future).await {
        Ok(result) => Ok(result),
        Err(_) => Err(format!(
            "Test operation '{}' timed out after {:?}. This may indicate:\n  \
            - Environment is headless/unresponsive\n  \
            - Operation is genuinely hanging\n  \
            - Timeout duration is too short for this operation",
            operation_name, timeout_duration
        )),
    }
}

/// Timeout wrapper specifically for STT/transcription tests
/// Uses extended timeout due to model loading overhead
pub async fn with_stt_timeout<F, T>(future: F, operation_name: &str) -> Result<T, String>
where
    F: std::future::Future<Output = T>,
{
    with_timeout(future, Some(EXTENDED_TEST_TIMEOUT), operation_name).await
}

/// Timeout wrapper for text injection tests
/// Uses default timeout but provides injection-specific error context
pub async fn with_injection_timeout<F, T>(future: F, operation_name: &str) -> Result<T, String>
where
    F: std::future::Future<Output = T>,
{
    let result = with_timeout(future, Some(DEFAULT_TEST_TIMEOUT), operation_name).await;

    match result {
        Err(timeout_msg) => Err(format!(
            "{}. Text injection tests require:\n  \
            - Display server (X11/Wayland) availability\n  \
            - Working clipboard utilities\n  \
            - Proper desktop environment setup",
            timeout_msg
        )),
        Ok(value) => Ok(value),
    }
}

/// Test timeout configuration based on environment variables
///
/// Allows customization of timeouts via environment variables:
/// - COLDVOX_TEST_TIMEOUT_SEC: Override default timeout
/// - COLDVOX_TEST_TIMEOUT_EXTENDED_SEC: Override extended timeout
/// - COLDVOX_TEST_TIMEOUT_SHORT_SEC: Override short timeout
pub struct TimeoutConfig {
    pub default: Duration,
    pub extended: Duration,
    pub short: Duration,
}

impl TimeoutConfig {
    pub fn from_env() -> Self {
        let default = std::env::var("COLDVOX_TEST_TIMEOUT_SEC")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .map(Duration::from_secs)
            .unwrap_or(DEFAULT_TEST_TIMEOUT);

        let extended = std::env::var("COLDVOX_TEST_TIMEOUT_EXTENDED_SEC")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .map(Duration::from_secs)
            .unwrap_or(EXTENDED_TEST_TIMEOUT);

        let short = std::env::var("COLDVOX_TEST_TIMEOUT_SHORT_SEC")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .map(Duration::from_secs)
            .unwrap_or(SHORT_TEST_TIMEOUT);

        Self {
            default,
            extended,
            short,
        }
    }
}

/// Macro for convenient timeout wrapping with automatic operation naming
///
/// # Examples
/// ```
/// // Use default timeout
/// let result = timeout_test!(some_async_operation()).await?;
///
/// // Use specific timeout
/// let result = timeout_test!(complex_operation(), EXTENDED_TEST_TIMEOUT).await?;
///
/// // Use custom timeout with name
/// let result = timeout_test!(custom_op(), Duration::from_secs(45), "custom operation").await?;
/// ```
#[macro_export]
macro_rules! timeout_test {
    ($future:expr) => {
        $crate::common::timeout::with_timeout($future, None, stringify!($future))
    };
    ($future:expr, $duration:expr) => {
        $crate::common::timeout::with_timeout($future, Some($duration), stringify!($future))
    };
    ($future:expr, $duration:expr, $name:expr) => {
        $crate::common::timeout::with_timeout($future, Some($duration), $name)
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[tokio::test]
    async fn test_timeout_success() {
        let start = Instant::now();
        let result = with_timeout(
            async { 42 },
            Some(Duration::from_millis(100)),
            "test operation",
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert!(start.elapsed() < Duration::from_millis(50)); // Should complete quickly
    }

    #[tokio::test]
    async fn test_timeout_failure() {
        let start = Instant::now();
        let result = with_timeout(
            async {
                tokio::time::sleep(Duration::from_millis(200)).await;
                42
            },
            Some(Duration::from_millis(50)),
            "slow operation",
        )
        .await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("slow operation"));
        assert!(error.contains("timed out"));

        // Should timeout close to specified duration
        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_millis(45));
        assert!(elapsed <= Duration::from_millis(100));
    }

    #[tokio::test]
    async fn test_stt_timeout_wrapper() {
        let result = with_stt_timeout(async { "transcription result" }, "STT test").await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "transcription result");
    }

    #[tokio::test]
    async fn test_injection_timeout_wrapper() {
        let result = with_injection_timeout(async {}, "injection test").await;

        assert!(result.is_ok());
    }

    #[test]
    fn test_timeout_config_defaults() {
        let config = TimeoutConfig::from_env();

        // Should use defaults if no env vars set
        assert!(config.default >= Duration::from_secs(10)); // Reasonable minimum
        assert!(config.extended > config.default);
        assert!(config.short < config.default);
    }

    #[tokio::test]
    async fn test_timeout_macro() {
        // Test basic macro usage - should compile and work
        let result = timeout_test!(async { "test" }).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test");
    }
}
