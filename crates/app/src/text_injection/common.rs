use anyhow::Result;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum InjectionError {
    #[error("method not available: {0}")]
    MethodNotAvailable(String),
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    #[error("injection failed: {0}")]
    InjectionFailed(String),
}

pub trait TextInjector: Send + Sync {
    fn name(&self) -> &'static str;
    fn is_available(&self) -> bool;
    fn inject(&self, text: &str) -> Result<()>;
}
