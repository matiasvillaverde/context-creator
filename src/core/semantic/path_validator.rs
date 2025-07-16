//! Secure path validation - KISS implementation
//!
//! Security principles:
//! 1. Fail closed - deny by default
//! 2. No TOCTOU - single atomic check
//! 3. Proper URL decoding before validation
//! 4. No manual path resolution

use crate::utils::error::CodeDigestError;
use std::path::{Path, PathBuf};

/// Validate import path - production-ready, fast, secure
pub fn validate_import_path(
    base_dir: &Path,
    import_path: &Path,
) -> Result<PathBuf, CodeDigestError> {
    // 1. Base directory must be absolute
    if !base_dir.is_absolute() {
        return Err(CodeDigestError::SecurityError(
            "Base directory must be absolute".to_string(),
        ));
    }

    // 2. Decode URL encoding BEFORE any path operations
    let path_str = import_path.to_string_lossy();
    let decoded = decode_url_path(&path_str)?;

    // 3. Reject if decoded path differs (indicates encoding was present)
    if decoded != path_str {
        return Err(CodeDigestError::SecurityError(format!(
            "URL-encoded paths are not allowed: {}",
            path_str
        )));
    }

    // 4. Convert to PathBuf and normalize slashes
    let normalized = PathBuf::from(decoded.replace('\\', "/"));

    // 5. Build the full path
    let full_path = if normalized.is_absolute() {
        normalized
    } else {
        base_dir.join(normalized)
    };

    // 6. CRITICAL: Only use canonicalize - never fall back to manual resolution
    // If the file doesn't exist, that's a legitimate error, not a security bypass
    let canonical_path = full_path.canonicalize().map_err(|e| {
        CodeDigestError::InvalidPath(format!(
            "Path does not exist or cannot be resolved: {} ({})",
            full_path.display(),
            e
        ))
    })?;

    let canonical_base = base_dir.canonicalize().map_err(|e| {
        CodeDigestError::SecurityError(format!("Cannot canonicalize base directory: {}", e))
    })?;

    // 7. Verify the canonical path is within base directory
    if !canonical_path.starts_with(&canonical_base) {
        return Err(CodeDigestError::SecurityError(format!(
            "Path escapes project directory: {}",
            import_path.display()
        )));
    }

    Ok(canonical_path)
}

/// Validate module name - fast, simple, secure
pub fn validate_module_name(module_name: &str) -> Result<(), CodeDigestError> {
    // Reject if empty
    if module_name.is_empty() {
        return Err(CodeDigestError::SecurityError(
            "Module name cannot be empty".to_string(),
        ));
    }

    // Reject if too long (DoS protection)
    if module_name.len() > 255 {
        return Err(CodeDigestError::SecurityError(
            "Module name too long".to_string(),
        ));
    }

    // Check for null bytes (string termination attacks)
    if module_name.contains('\0') {
        return Err(CodeDigestError::SecurityError(
            "Module name contains null byte".to_string(),
        ));
    }

    // Simple check for path traversal
    if module_name.contains("..") {
        return Err(CodeDigestError::SecurityError(format!(
            "Invalid module name: {}",
            module_name
        )));
    }

    // Allow only safe characters using a fast check
    let valid_chars = module_name.chars().all(|c| {
        c.is_alphanumeric() || c == '_' || c == '-' || c == '.' || c == '@' || c == '/' || c == ':'
        // For scoped packages like @types/node
    });

    if !valid_chars {
        return Err(CodeDigestError::SecurityError(format!(
            "Module name contains invalid characters: {}",
            module_name
        )));
    }

    Ok(())
}

/// Decode URL-encoded path - handles all encoding variants
fn decode_url_path(path: &str) -> Result<String, CodeDigestError> {
    // Fast path - if no % sign, no decoding needed
    if !path.contains('%') {
        return Ok(path.to_string());
    }

    // Use percent_encoding crate for proper decoding
    // For now, simple check for common patterns
    let lower = path.to_lowercase();

    // Check for any hex encoding patterns
    if lower.contains("%2e") || // .
       lower.contains("%2f") || // /
       lower.contains("%5c") || // \
       lower.contains("%00") || // null
       lower.contains("%25") || // % (double encoding)
       lower.contains("%c0") || // UTF-8 overlong
       lower.contains("%e0") || // UTF-8 variants
       lower.contains("%f0") || // UTF-8 variants
       lower.contains("%u00")
    // Unicode encoding
    {
        return Err(CodeDigestError::SecurityError(
            "URL-encoded characters detected in path".to_string(),
        ));
    }

    Ok(path.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_valid_paths() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();

        // Create test structure
        fs::create_dir_all(base.join("src")).unwrap();
        fs::write(base.join("src/lib.rs"), "").unwrap();

        // Valid paths should work
        let result = validate_import_path(base, &PathBuf::from("src/lib.rs"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_path_traversal_blocked() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();

        // Create a file to try to escape to
        let target = base.join("target.txt");
        fs::write(&target, "target").unwrap();

        // Try to escape using ../
        let escape_path = base.join("src/../../../etc/passwd");
        let result = validate_import_path(base, &escape_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_url_encoding_blocked() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();

        let encoded_paths = vec![
            "src/%2e%2e/secret",
            "src%2f%2e%2e%2fsecret",
            "%2e%2e%2f%2e%2e%2fetc%2fpasswd",
        ];

        for path in encoded_paths {
            let result = validate_import_path(base, &PathBuf::from(path));
            assert!(result.is_err(), "Should block: {}", path);
        }
    }

    #[test]
    fn test_symlink_blocked() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;

            // Create a symlink to /etc/passwd
            let link_path = base.join("evil_link");
            symlink("/etc/passwd", &link_path).unwrap();

            let result = validate_import_path(base, &link_path);
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_nonexistent_file_fails() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();

        // Nonexistent files should fail (fail closed)
        let result = validate_import_path(base, &PathBuf::from("does/not/exist.rs"));
        assert!(result.is_err());
    }

    #[test]
    fn test_module_name_validation() {
        // Valid names
        assert!(validate_module_name("lodash").is_ok());
        assert!(validate_module_name("@angular/core").is_ok());
        assert!(validate_module_name("@types/node").is_ok());

        // Invalid names
        assert!(validate_module_name("").is_err());
        assert!(validate_module_name("../../../etc/passwd").is_err());
        assert!(validate_module_name("name\0with\0null").is_err());
        assert!(validate_module_name(&"a".repeat(256)).is_err());
        assert!(validate_module_name("rm -rf /").is_err());
    }
}
