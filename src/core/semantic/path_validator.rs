//! Path validation for import resolution to prevent security vulnerabilities
//! Ensures all resolved paths stay within the project directory

use crate::utils::error::CodeDigestError;
use std::path::{Path, PathBuf};

/// Security error type for path validation failures
#[derive(Debug, thiserror::Error)]
pub enum PathSecurityError {
    #[error("Path traversal attempt detected: {path}")]
    PathTraversal { path: String },

    #[error("Absolute path outside project: {path}")]
    AbsolutePathOutsideProject { path: String },

    #[error("Invalid path: {reason}")]
    InvalidPath { reason: String },

    #[error("Symlink points outside project: {path} -> {target}")]
    SymlinkEscape { path: String, target: String },
}

/// Validate that an import path is safe and within the project directory
///
/// # Security
///
/// This function prevents:
/// - Path traversal attacks (../, ..\)
/// - Absolute paths outside the project
/// - Symlinks that escape the project directory
/// - URL-encoded path traversal attempts
///
/// # Arguments
///
/// * `base_dir` - The project root directory (must be absolute)
/// * `import_path` - The path to validate (can be relative or absolute)
///
/// # Returns
///
/// The canonicalized, validated path if safe, or an error if the path is unsafe
pub fn validate_import_path(
    base_dir: &Path,
    import_path: &Path,
) -> Result<PathBuf, CodeDigestError> {
    // Ensure base_dir is absolute
    let base_dir = if base_dir.is_absolute() {
        base_dir
    } else {
        return Err(CodeDigestError::SecurityError(
            PathSecurityError::InvalidPath {
                reason: "Base directory must be absolute".to_string(),
            }
            .to_string(),
        ));
    };

    // Decode any URL-encoded sequences first
    let import_str = import_path.to_string_lossy();
    if import_str.contains("%2e") || import_str.contains("%2E") {
        return Err(CodeDigestError::SecurityError(
            PathSecurityError::PathTraversal {
                path: import_str.to_string(),
            }
            .to_string(),
        ));
    }

    // Convert Windows-style path separators to Unix style for consistent checking
    let normalized_path = if cfg!(windows) {
        PathBuf::from(import_str.replace('\\', "/"))
    } else {
        import_path.to_path_buf()
    };

    // Check for obvious path traversal patterns
    let path_str = normalized_path.to_string_lossy();
    if path_str.contains("../") || path_str.contains("..\\") {
        // Allow it only if the final resolved path is still within base_dir
        // This is checked below with canonicalize
    }

    // Resolve the path relative to base_dir if it's relative
    let resolved_path = if normalized_path.is_absolute() {
        normalized_path
    } else {
        base_dir.join(&normalized_path)
    };

    // Canonicalize to resolve symlinks and remove ../ components
    // Note: This will fail if the file doesn't exist, which is fine for security
    let canonical_path = match resolved_path.canonicalize() {
        Ok(path) => path,
        Err(_) => {
            // If canonicalize fails (file doesn't exist), manually resolve ../ components
            resolve_path_components(&resolved_path)?
        }
    };

    // Canonicalize base_dir for comparison
    let canonical_base = match base_dir.canonicalize() {
        Ok(path) => path,
        Err(e) => {
            return Err(CodeDigestError::SecurityError(format!(
                "Failed to canonicalize base directory: {}",
                e
            )));
        }
    };

    // Check if the resolved path is within the base directory
    if !canonical_path.starts_with(&canonical_base) {
        return Err(CodeDigestError::SecurityError(
            PathSecurityError::PathTraversal {
                path: canonical_path.display().to_string(),
            }
            .to_string(),
        ));
    }

    // Additional check: if the original path was absolute, ensure it was within base_dir
    if import_path.is_absolute() && !import_path.starts_with(base_dir) {
        return Err(CodeDigestError::SecurityError(
            PathSecurityError::AbsolutePathOutsideProject {
                path: import_path.display().to_string(),
            }
            .to_string(),
        ));
    }

    Ok(canonical_path)
}

/// Manually resolve path components when canonicalize fails
/// This is a security-critical function that must handle ../ safely
fn resolve_path_components(path: &Path) -> Result<PathBuf, CodeDigestError> {
    let mut components = Vec::new();

    for component in path.components() {
        use std::path::Component;

        match component {
            Component::ParentDir => {
                // Only pop if we have components and the last one isn't also ParentDir
                if !components.is_empty() {
                    components.pop();
                }
            }
            Component::Normal(name) => {
                components.push(name);
            }
            Component::RootDir => {
                components.clear();
                components.push(std::ffi::OsStr::new("/"));
            }
            Component::CurDir => {
                // Skip . components
            }
            Component::Prefix(_) => {
                // Windows prefix (C:, etc.)
                return Err(CodeDigestError::SecurityError(
                    "Windows prefix paths not supported in path resolution".to_string(),
                ));
            }
        }
    }

    // Reconstruct the path
    let mut result = PathBuf::new();
    for component in components {
        result.push(component);
    }

    Ok(result)
}

/// Validate a module name for safety
/// Prevents injection attacks through module names
pub fn validate_module_name(module_name: &str) -> Result<(), CodeDigestError> {
    // Check for path traversal in module name
    if module_name.contains("..") {
        return Err(CodeDigestError::SecurityError(format!(
            "Invalid module name: {}",
            module_name
        )));
    }

    // Allow forward slashes for scoped packages like @angular/core
    // but not backslashes which could be Windows path separators
    if module_name.contains("\\") {
        return Err(CodeDigestError::SecurityError(format!(
            "Invalid module name: {}",
            module_name
        )));
    }

    // Check for special characters that could be used for injection
    // Allow alphanumeric, underscore, hyphen, dot, at sign, and forward slash
    if module_name.contains(|c: char| {
        !c.is_alphanumeric() && c != '_' && c != '-' && c != '.' && c != '@' && c != '/'
    }) {
        return Err(CodeDigestError::SecurityError(format!(
            "Module name contains invalid characters: {}",
            module_name
        )));
    }

    Ok(())
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

        // Create some test files
        fs::create_dir_all(base.join("src")).unwrap();
        fs::write(base.join("src/lib.rs"), "").unwrap();

        // Test valid relative path
        let result = validate_import_path(base, &PathBuf::from("src/lib.rs"));
        assert!(result.is_ok());

        // Test valid absolute path within project
        let result = validate_import_path(base, &base.join("src/lib.rs"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_path_traversal_blocked() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();

        // Test various traversal attempts
        assert!(validate_import_path(base, &PathBuf::from("../../../etc/passwd")).is_err());
        assert!(validate_import_path(base, &PathBuf::from("src/../../etc/passwd")).is_err());

        // Test absolute path outside project
        assert!(validate_import_path(base, &PathBuf::from("/etc/passwd")).is_err());
    }

    #[test]
    fn test_url_encoded_traversal() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();

        // Test URL-encoded traversal attempts
        assert!(validate_import_path(base, &PathBuf::from("%2e%2e/etc/passwd")).is_err());
        assert!(validate_import_path(base, &PathBuf::from("..%2f..%2fetc")).is_err());
    }

    #[test]
    fn test_module_name_validation() {
        // Valid module names
        assert!(validate_module_name("lodash").is_ok());
        assert!(validate_module_name("@angular/core").is_ok());
        assert!(validate_module_name("react-dom").is_ok());
        assert!(validate_module_name("vue_3").is_ok());

        // Invalid module names
        assert!(validate_module_name("../../../etc/passwd").is_err());
        assert!(validate_module_name("../../secret").is_err());
        assert!(validate_module_name("rm -rf /").is_err());
        assert!(validate_module_name("module; cat /etc/passwd").is_err());
    }

    #[test]
    fn test_resolve_path_components() {
        // Test manual path resolution
        let path = PathBuf::from("/home/user/project/src/../lib/test.rs");
        let resolved = resolve_path_components(&path).unwrap();
        assert_eq!(resolved, PathBuf::from("/home/user/project/lib/test.rs"));

        // Test with multiple parent dirs
        let path = PathBuf::from("src/../../lib/test.rs");
        let resolved = resolve_path_components(&path).unwrap();
        assert_eq!(resolved, PathBuf::from("lib/test.rs"));
    }
}
