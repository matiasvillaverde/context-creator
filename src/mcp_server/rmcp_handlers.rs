//! Helper functions for RMCP server implementation

use anyhow::{bail, Result};
use std::path::Path;

/// Validate path for security issues
pub fn validate_path(path: &Path) -> Result<()> {
    // Check for path traversal attempts
    let path_str = path.to_string_lossy();
    if path_str.contains("..") || path_str.contains('~') {
        bail!("Path traversal attempt detected");
    }

    // Check if path exists
    if !path.exists() {
        bail!("Path does not exist: {}", path.display());
    }

    // Check if we have read permissions
    if !path.is_dir() && !path.is_file() {
        bail!("Path is not a file or directory: {}", path.display());
    }

    Ok(())
}

/// Validate URL for security
pub fn validate_url(url: &str) -> Result<()> {
    // Basic URL validation
    if !url.starts_with("https://") && !url.starts_with("http://") {
        bail!("Invalid URL: must start with http:// or https://");
    }

    // GitHub URL validation
    if url.contains("github.com") && !url.contains("/") {
        bail!("Invalid GitHub URL format");
    }

    Ok(())
}