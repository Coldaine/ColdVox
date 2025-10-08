/// Utility to temporarily suppress stderr output from ALSA/CPAL library calls
/// that produce noisy warnings about missing PCM plugins (pulse, jack, oss, etc.)
use std::fs::File;
use std::io;
use std::os::unix::io::{AsRawFd, FromRawFd};

/// RAII guard that redirects stderr to /dev/null and restores it on drop
pub struct StderrSuppressor {
    original_stderr: Option<File>,
}

impl StderrSuppressor {
    /// Suppress stderr by redirecting to /dev/null
    pub fn new() -> io::Result<Self> {
        unsafe {
            // Save original stderr
            let original_stderr_fd = libc::dup(libc::STDERR_FILENO);
            if original_stderr_fd < 0 {
                return Err(io::Error::last_os_error());
            }
            let original_stderr = File::from_raw_fd(original_stderr_fd);

            // Redirect stderr to /dev/null
            let devnull = File::open("/dev/null")?;
            let devnull_fd = devnull.as_raw_fd();
            if libc::dup2(devnull_fd, libc::STDERR_FILENO) < 0 {
                return Err(io::Error::last_os_error());
            }

            Ok(Self {
                original_stderr: Some(original_stderr),
            })
        }
    }

    /// Execute a closure with stderr suppressed
    pub fn with_suppressed<F, R>(f: F) -> R
    where
        F: FnOnce() -> R,
    {
        match Self::new() {
            Ok(guard) => {
                let result = f();
                drop(guard);
                result
            }
            Err(_) => {
                // If we can't suppress, just run anyway
                f()
            }
        }
    }
}

impl Drop for StderrSuppressor {
    fn drop(&mut self) {
        if let Some(original) = self.original_stderr.take() {
            unsafe {
                // Restore original stderr
                let original_fd = original.as_raw_fd();
                libc::dup2(original_fd, libc::STDERR_FILENO);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suppressor_basic() {
        // Should not panic
        let _guard = StderrSuppressor::new().unwrap();
    }

    #[test]
    fn test_with_suppressed() {
        let result = StderrSuppressor::with_suppressed(|| {
            eprintln!("This should be suppressed");
            42
        });
        assert_eq!(result, 42);
    }
}
