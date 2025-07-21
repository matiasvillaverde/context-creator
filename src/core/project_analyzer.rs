//! Project-wide analysis cache for semantic features
//!
//! This module provides a single-pass project analysis that can be reused
//! across different semantic features to avoid redundant directory walks.

use crate::cli::Config;
use crate::core::cache::FileCache;
use crate::core::walker::{walk_directory, FileInfo, WalkOptions};
use crate::utils::error::ContextCreatorError;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::info;

/// Cached project analysis results
pub struct ProjectAnalysis {
    /// All files in the project with semantic analysis
    pub all_files: Vec<FileInfo>,
    /// Map from canonical paths to file info for fast lookups
    pub file_map: HashMap<PathBuf, FileInfo>,
    /// Project root directory
    pub project_root: PathBuf,
}

impl ProjectAnalysis {
    /// Perform a single comprehensive analysis of the entire project
    pub fn analyze_project(
        start_path: &Path,
        base_walk_options: &WalkOptions,
        config: &Config,
        cache: &Arc<FileCache>,
    ) -> Result<Self, ContextCreatorError> {
        // Detect project root
        let project_root = if start_path.is_file() {
            super::file_expander::detect_project_root(start_path)
        } else {
            // For directories, detect project root directly
            super::file_expander::detect_project_root(start_path)
        };

        // Create walk options for full project scan (no include patterns)
        let mut project_walk_options = base_walk_options.clone();
        project_walk_options.include_patterns.clear();

        // Single walk of the entire project
        if config.progress && !config.quiet {
            info!("Analyzing project from: {}", project_root.display());
        }

        let mut all_files = walk_directory(&project_root, project_walk_options)
            .map_err(|e| ContextCreatorError::ContextGenerationError(e.to_string()))?;

        // Perform semantic analysis once
        if config.trace_imports || config.include_callers || config.include_types {
            super::walker::perform_semantic_analysis(&mut all_files, config, cache)
                .map_err(|e| ContextCreatorError::ContextGenerationError(e.to_string()))?;

            if config.progress && !config.quiet {
                let import_count: usize = all_files.iter().map(|f| f.imports.len()).sum();
                info!("Found {} import relationships in project", import_count);
            }
        }

        // Build file map for fast lookups
        let mut file_map = HashMap::with_capacity(all_files.len());
        for file in &all_files {
            // Use both original and canonical paths as keys
            file_map.insert(file.path.clone(), file.clone());
            if let Ok(canonical) = file.path.canonicalize() {
                file_map.insert(canonical, file.clone());
            }
        }

        Ok(ProjectAnalysis {
            all_files,
            file_map,
            project_root,
        })
    }

    /// Get a file by path (handles both canonical and non-canonical paths)
    pub fn get_file(&self, path: &Path) -> Option<&FileInfo> {
        // Try direct lookup first
        if let Some(file) = self.file_map.get(path) {
            return Some(file);
        }

        // Try canonical path
        if let Ok(canonical) = path.canonicalize() {
            self.file_map.get(&canonical)
        } else {
            None
        }
    }

    /// Filter files by the original walk options
    pub fn filter_files(&self, walk_options: &WalkOptions) -> Vec<FileInfo> {
        self.all_files
            .iter()
            .filter(|file| {
                // Apply include patterns if any
                if !walk_options.include_patterns.is_empty() {
                    let matches_include = walk_options.include_patterns.iter().any(|pattern| {
                        glob::Pattern::new(pattern)
                            .ok()
                            .map(|p| p.matches_path(&file.relative_path))
                            .unwrap_or(false)
                    });
                    if !matches_include {
                        return false;
                    }
                }

                // File passed all filters
                true
            })
            .cloned()
            .collect()
    }
}
