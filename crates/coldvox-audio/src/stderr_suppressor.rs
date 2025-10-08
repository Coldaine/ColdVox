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
        // SAFETY: We need unsafe for low-level file descriptor manipulation via libc:
        // 1. `libc::dup(STDERR_FILENO)` - Duplicates stderr FD so we can restore it later.
        //    This is safe because STDERR_FILENO is always valid (2) in Unix processes.
        // 2. `File::from_raw_fd()` - Takes ownership of the duplicated FD. Safe because
        //    we just created a valid FD via dup, and we ensure it's only used once.
        // 3. `libc::dup2(devnull_fd, STDERR_FILENO)` - Atomically replaces stderr with /dev/null.
        //    Safe because both FDs are valid: devnull_fd from File::open, STDERR_FILENO is 2.
        // The FDs are properly managed: original_stderr owns the dup'd FD and will close it on drop,
        // and the dup2 call ensures the replacement is atomic.
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
            // SAFETY: We need unsafe to restore the original stderr file descriptor:
            // `libc::dup2(original_fd, STDERR_FILENO)` - Atomically replaces current stderr
            // with our saved original. Safe because:
            // 1. original_fd comes from the File we created in new(), which owns a valid FD
            // 2. STDERR_FILENO (2) is always a valid target in Unix processes
            // 3. dup2 is atomic and handles the close of the current stderr internally
            // We intentionally ignore errors here because Drop must not panic, and there's
            // nothing meaningful to do if restoration fails (the program is likely shutting down).
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
