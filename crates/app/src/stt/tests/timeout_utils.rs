/// Timeout utilities for STT test execution to prevent hanging in CI/headless environments.
use std::time::Duration;
use tokio::time::timeout;

/// Default timeout for most test operations (30 seconds)
pub const DEFAULT_TEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Wrap an async operation with a timeout and provide clear error messaging.
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
            "Test operation '{}' timed out after {:?}. This may indicate a headless/unresponsive environment or hanging operation.",
            operation_name,
            timeout_duration
        )),
    }
}

/// Timeout wrapper specifically for text injection tests
pub async fn with_injection_timeout<F, T>(future: F, operation_name: &str) -> Result<T, String>
where
    F: std::future::Future<Output = T>,
{
    let result = with_timeout(future, Some(DEFAULT_TEST_TIMEOUT), operation_name).await;

    match result {
        Err(timeout_msg) => Err(format!(
            "{}. Text injection tests require display server and desktop environment.",
            timeout_msg
        )),
        Ok(value) => Ok(value),
    }
}
