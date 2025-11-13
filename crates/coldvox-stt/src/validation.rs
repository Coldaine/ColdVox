//! Types and functions for validating file integrity using SHA256 checksums.

use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use coldvox_foundation::error::{ColdVoxError, SttError};

/// Represents a collection of SHA256 checksums for model files.
/// The keys are filenames, and the values are the corresponding checksums.
#[derive(Debug, Deserialize, Clone)]
pub struct Checksums {
    #[serde(flatten)]
    pub files: HashMap<PathBuf, String>,
}

impl Checksums {
    /// Loads checksums from a JSON file.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the checksums file.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `Checksums` struct or an error if the file
    /// cannot be read or parsed.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, ColdVoxError> {
        let content = fs::read_to_string(path.as_ref()).map_err(|err| {
            SttError::ChecksumFailed(format!(
                "Failed to read checksum file at {}: {}",
                path.as_ref().display(),
                err
            ))
        })?;
        serde_json::from_str(&content).map_err(|err| {
            {
                SttError::ChecksumFailed(format!(
                    "Failed to parse checksum file at {}: {}",
                    path.as_ref().display(),
                    err
                ))
            }
            .into()
        })
    }

    /// Verifies the checksum of a file.
    ///
    /// # Arguments
    ///
    /// * `file_path` - The path to the file to verify.
    ///
    /// # Returns
    ///
    /// A `Result` indicating whether the checksum is valid or an error if
    /// the checksum is missing or does not match.
    pub fn verify<P: AsRef<Path>>(&self, file_path: P) -> Result<(), ColdVoxError> {
        let file_path = file_path.as_ref();
        let file_name = file_path.file_name().ok_or_else(|| {
            SttError::ChecksumFailed(format!(
                "Could not get file name from path: {}",
                file_path.display()
            ))
        })?;

        let expected_checksum = self.files.get(Path::new(file_name)).ok_or_else(|| {
            SttError::ChecksumFailed(format!(
                "No checksum found for model: {}",
                file_name.to_string_lossy()
            ))
        })?;

        let actual_checksum = compute_sha256(file_path)?;

        if &actual_checksum == expected_checksum {
            Ok(())
        } else {
            Err(SttError::ChecksumFailed(format!(
                "Checksum mismatch for model: {}\n  Expected: {}\n  Actual:   {}",
                file_name.to_string_lossy(),
                expected_checksum,
                actual_checksum
            ))
            .into())
        }
    }
}

/// Computes the SHA256 checksum of a file.
///
/// # Arguments
///
/// * `path` - The path to the file.
///
/// # Returns
///
/// A `Result` containing the hex-encoded SHA256 checksum or an error if
/// the file cannot be read.
pub fn compute_sha256<P: AsRef<Path>>(path: P) -> Result<String, ColdVoxError> {
    use sha2::{Digest, Sha256};
    let mut file = fs::File::open(path.as_ref()).map_err(|err| {
        SttError::ChecksumFailed(format!(
            "Failed to open file for hashing at {}: {}",
            path.as_ref().display(),
            err
        ))
    })?;
    let mut hasher = Sha256::new();
    io::copy(&mut file, &mut hasher).map_err(|err| {
        SttError::ChecksumFailed(format!(
            "Failed to read file for hashing at {}: {}",
            path.as_ref().display(),
            err
        ))
    })?;
    let hash = hasher.finalize();
    Ok(format!("{:x}", hash))
}
