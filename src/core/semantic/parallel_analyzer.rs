//! Parallel file analysis module for semantic analysis
//!
//! This module is responsible for managing parallel processing of file analysis.
//! It follows the Single Responsibility Principle by focusing solely on parallelization.

use crate::core::cache::FileCache;
use crate::core::semantic::analyzer::SemanticContext;
use crate::core::semantic::dependency_types::{DependencyEdgeType, FileAnalysisResult};
use crate::core::semantic::{get_analyzer_for_file, get_resolver_for_file};
use anyhow::Result;
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Options for file analysis
#[derive(Debug, Clone)]
pub struct AnalysisOptions {
    /// Maximum depth for semantic analysis
    pub semantic_depth: usize,
    /// Whether to trace imports
    pub trace_imports: bool,
    /// Whether to include type references
    pub include_types: bool,
    /// Whether to include function calls
    pub include_functions: bool,
}

impl Default for AnalysisOptions {
    fn default() -> Self {
        Self {
            semantic_depth: 3,
            trace_imports: true,
            include_types: true,
            include_functions: true,
        }
    }
}

/// Parallel analyzer for file processing
pub struct ParallelAnalyzer<'a> {
    cache: &'a FileCache,
    thread_count: Option<usize>,
}

impl<'a> ParallelAnalyzer<'a> {
    /// Create a new ParallelAnalyzer
    pub fn new(cache: &'a FileCache) -> Self {
        Self {
            cache,
            thread_count: None,
        }
    }

    /// Create a new ParallelAnalyzer with a specific thread count
    pub fn with_thread_count(cache: &'a FileCache, thread_count: usize) -> Self {
        Self {
            cache,
            thread_count: Some(thread_count),
        }
    }

    /// Analyze multiple files in parallel
    pub fn analyze_files(
        &self,
        files: &[PathBuf],
        project_root: &Path,
        options: &AnalysisOptions,
        valid_files: &std::collections::HashSet<PathBuf>,
    ) -> Result<Vec<FileAnalysisResult>> {
        // Configure thread pool if specified
        if let Some(count) = self.thread_count {
            rayon::ThreadPoolBuilder::new()
                .num_threads(count)
                .build_global()
                .ok(); // Ignore error if already initialized
        }

        // Create error collector
        let errors = Arc::new(Mutex::new(Vec::new()));
        let errors_ref = &errors;

        // Analyze files in parallel
        let results: Vec<FileAnalysisResult> = files
            .par_iter()
            .enumerate()
            .map(|(index, file_path)| {
                match self.analyze_single_file(index, file_path, project_root, options, valid_files)
                {
                    Ok(result) => result,
                    Err(e) => {
                        let error_msg = format!("Failed to analyze {}: {}", file_path.display(), e);
                        errors_ref.lock().unwrap().push(error_msg.clone());

                        // Return a minimal result with error
                        FileAnalysisResult {
                            file_index: index,
                            imports: Vec::new(),
                            function_calls: Vec::new(),
                            type_references: Vec::new(),
                            content_hash: None,
                            error: Some(error_msg),
                        }
                    }
                }
            })
            .collect();

        // Print collected errors
        let error_list = errors.lock().unwrap();
        for error in error_list.iter() {
            eprintln!("Warning: {error}");
        }

        Ok(results)
    }

    /// Analyze a single file
    #[allow(clippy::too_many_arguments)]
    fn analyze_single_file(
        &self,
        file_index: usize,
        file_path: &Path,
        project_root: &Path,
        options: &AnalysisOptions,
        valid_files: &std::collections::HashSet<PathBuf>,
    ) -> Result<FileAnalysisResult> {
        // Get analyzer for the file type
        let analyzer = match get_analyzer_for_file(file_path)? {
            Some(analyzer) => analyzer,
            None => {
                // No analyzer for this file type - return empty result
                return Ok(FileAnalysisResult {
                    file_index,
                    imports: Vec::new(),
                    function_calls: Vec::new(),
                    type_references: Vec::new(),
                    content_hash: Some(self.compute_content_hash(file_path)?),
                    error: None,
                });
            }
        };

        // Read file content
        let content = self.cache.get_or_load(file_path)?;

        // Compute content hash
        let content_hash = {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            content.hash(&mut hasher);
            hasher.finish()
        };

        // Create semantic context
        let context = SemanticContext::new(
            file_path.to_path_buf(),
            project_root.to_path_buf(),
            options.semantic_depth,
        );

        // Perform analysis
        let analysis_result = analyzer.analyze_file(file_path, &content, &context)?;

        // Process imports if enabled
        let imports = if options.trace_imports {
            self.process_imports(
                file_path,
                project_root,
                &analysis_result.imports,
                valid_files,
            )?
        } else {
            Vec::new()
        };

        // Filter results based on options
        let function_calls = if options.include_functions {
            analysis_result.function_calls
        } else {
            Vec::new()
        };

        let type_references = if options.include_types {
            analysis_result.type_references
        } else {
            Vec::new()
        };

        Ok(FileAnalysisResult {
            file_index,
            imports,
            function_calls,
            type_references,
            content_hash: Some(content_hash),
            error: None,
        })
    }

    /// Process imports to create typed edges
    fn process_imports(
        &self,
        file_path: &Path,
        project_root: &Path,
        imports: &[crate::core::semantic::analyzer::Import],
        valid_files: &std::collections::HashSet<PathBuf>,
    ) -> Result<Vec<(PathBuf, DependencyEdgeType)>> {
        let mut typed_imports = Vec::new();

        // Get resolver for the file type
        if let Some(resolver) = get_resolver_for_file(file_path)? {
            for import in imports {
                // Try to resolve the import
                match resolver.resolve_import(&import.module, file_path, project_root) {
                    Ok(resolved) => {
                        if !resolved.is_external && valid_files.contains(&resolved.path) {
                            let edge_type = DependencyEdgeType::Import {
                                symbols: import.items.clone(),
                            };
                            typed_imports.push((resolved.path, edge_type));
                        }
                    }
                    Err(_) => {
                        // For relative imports, try to resolve manually
                        if import.module.starts_with(".") {
                            if let Some(parent) = file_path.parent() {
                                let module_base = import.module.trim_start_matches("./");

                                // Try common extensions
                                for ext in &["js", "jsx", "ts", "tsx"] {
                                    let potential_path =
                                        parent.join(format!("{module_base}.{ext}"));

                                    if valid_files.contains(&potential_path) {
                                        let edge_type = DependencyEdgeType::Import {
                                            symbols: import.items.clone(),
                                        };
                                        typed_imports.push((potential_path, edge_type));
                                        break;
                                    }
                                }
                            }
                        } else {
                            // Fallback: use module path as-is only if it's in valid files
                            let fallback_path = PathBuf::from(&import.module);
                            if valid_files.contains(&fallback_path) {
                                let edge_type = DependencyEdgeType::Import {
                                    symbols: import.items.clone(),
                                };
                                typed_imports.push((fallback_path, edge_type));
                            }
                        }
                    }
                }
            }
        } else {
            // No resolver available - use basic import edges only if in valid files
            for import in imports {
                let import_path = PathBuf::from(&import.module);
                if valid_files.contains(&import_path) {
                    let edge_type = DependencyEdgeType::Import {
                        symbols: import.items.clone(),
                    };
                    typed_imports.push((import_path, edge_type));
                }
            }
        }

        Ok(typed_imports)
    }

    /// Compute content hash for a file
    fn compute_content_hash(&self, file_path: &Path) -> Result<u64> {
        let content = self.cache.get_or_load(file_path)?;

        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        Ok(hasher.finish())
    }
}

#[cfg(test)]
#[path = "parallel_analyzer_tests.rs"]
mod tests;
