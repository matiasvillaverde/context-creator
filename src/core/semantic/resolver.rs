//! Module resolution for converting import strings to file paths

use crate::utils::error::ContextCreatorError;
use std::path::{Path, PathBuf};

/// A resolved module path
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedPath {
    /// The resolved file path
    pub path: PathBuf,
    /// Whether this is a third-party module (not in project)
    pub is_external: bool,
    /// Confidence in the resolution (0.0 to 1.0)
    pub confidence: f32,
}

/// Trait for language-specific module resolution
pub trait ModuleResolver: Send + Sync {
    /// Resolve a module import to a file path
    fn resolve_import(
        &self,
        module_path: &str,
        from_file: &Path,
        base_dir: &Path,
    ) -> Result<ResolvedPath, ContextCreatorError>;

    /// Get common file extensions for this language
    fn get_file_extensions(&self) -> Vec<&'static str>;

    /// Check if a module is likely external/third-party
    fn is_external_module(&self, module_path: &str) -> bool {
        // Default heuristics - languages can override
        module_path.starts_with('@') || // npm scoped packages
        !module_path.starts_with('.') || // relative imports usually start with .
        module_path.contains("node_modules") ||
        module_path.contains("site-packages") ||
        module_path.contains("vendor")
    }
}

/// Common module resolution utilities
pub struct ResolverUtils;

impl ResolverUtils {
    /// Try to find a file with different extensions
    pub fn find_with_extensions(base_path: &Path, extensions: &[&str]) -> Option<PathBuf> {
        // Try exact path first
        if base_path.exists() && base_path.is_file() {
            return Some(base_path.to_path_buf());
        }

        // Try with each extension
        for ext in extensions {
            let with_ext = base_path.with_extension(ext);
            if with_ext.exists() && with_ext.is_file() {
                return Some(with_ext);
            }
        }

        // Try as directory with index file
        if base_path.exists() && base_path.is_dir() {
            for index_name in &["index", "mod", "__init__"] {
                for ext in extensions {
                    let index_path = base_path.join(format!("{index_name}.{ext}"));
                    if index_path.exists() && index_path.is_file() {
                        return Some(index_path);
                    }
                }
            }
        }

        None
    }

    /// Convert module path separators to file path separators
    pub fn module_to_path(module_path: &str) -> PathBuf {
        PathBuf::from(module_path.replace('.', "/").replace("::", "/"))
    }

    /// Resolve a relative import path
    pub fn resolve_relative(
        import_path: &str,
        from_file: &Path,
        extensions: &[&str],
    ) -> Option<PathBuf> {
        let from_dir = from_file.parent()?;

        // Handle different relative import styles
        let clean_path = import_path
            .trim_start_matches("./")
            .trim_start_matches("../");

        let mut current_dir = from_dir.to_path_buf();

        // Count leading ../ to go up directories
        let up_count = import_path.matches("../").count();
        for _ in 0..up_count {
            current_dir = current_dir.parent()?.to_path_buf();
        }

        let target = current_dir.join(clean_path);
        Self::find_with_extensions(&target, extensions)
    }

    /// Check if a path is within the project directory
    pub fn is_within_project(path: &Path, base_dir: &Path) -> bool {
        path.canonicalize()
            .ok()
            .and_then(|p| base_dir.canonicalize().ok().map(|b| p.starts_with(b)))
            .unwrap_or(false)
    }
}
