//! File expansion logic for semantic analysis features
//!
//! This module handles expanding the file list based on semantic relationships
//! discovered during analysis (imports, type references, function calls).

use crate::cli::Config;
use crate::core::cache::FileCache;
use crate::core::semantic::function_call_index::FunctionCallIndex;
use crate::core::semantic::path_validator::validate_import_path;
use crate::core::semantic::type_resolver::{ResolutionLimits, TypeResolver};
use crate::core::walker::{walk_directory, FileInfo};
use crate::utils::error::ContextCreatorError;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Build an efficient gitignore matcher from walk options
fn build_ignore_matcher(
    walk_options: &crate::core::walker::WalkOptions,
    base_path: &Path,
) -> Option<Gitignore> {
    if walk_options.ignore_patterns.is_empty() {
        return None;
    }

    let mut builder = GitignoreBuilder::new(base_path);

    // Add each ignore pattern
    for pattern in &walk_options.ignore_patterns {
        // The ignore crate handles patterns efficiently
        let _ = builder.add_line(None, pattern);
    }

    builder.build().ok()
}

/// Detect the project root directory using git root or fallback methods
pub fn detect_project_root(start_path: &Path) -> PathBuf {
    // If start_path is a file, start from its parent directory
    let start_dir = if start_path.is_file() {
        start_path.parent().unwrap_or(start_path)
    } else {
        start_path
    };

    // First try to find git root
    let mut current = start_dir;
    loop {
        if current.join(".git").exists() {
            return current.to_path_buf();
        }
        if let Some(parent) = current.parent() {
            current = parent;
        } else {
            break;
        }
    }

    // Fallback: Look for common project markers
    current = start_dir;
    loop {
        // Check for Rust project markers
        if current.join("Cargo.toml").exists() {
            return current.to_path_buf();
        }
        // Check for Node.js project markers
        if current.join("package.json").exists() {
            return current.to_path_buf();
        }
        // Check for Python project markers
        if current.join("pyproject.toml").exists() || current.join("setup.py").exists() {
            return current.to_path_buf();
        }
        // Check for generic project markers
        if current.join("README.md").exists() || current.join("readme.md").exists() {
            return current.to_path_buf();
        }

        if let Some(parent) = current.parent() {
            current = parent;
        } else {
            break;
        }
    }

    // Ultimate fallback: use the start directory
    start_dir.to_path_buf()
}

/// Expand file list based on semantic relationships with full project context
///
/// This function takes the initial set of files and expands it to include
/// files that define types, export functions, or are imported by the initial files.
/// It uses the full project context to find dependencies that may be outside the initial scope.
pub fn expand_file_list_with_context(
    files_map: HashMap<PathBuf, FileInfo>,
    config: &Config,
    cache: &Arc<FileCache>,
    walk_options: &crate::core::walker::WalkOptions,
    all_files_context: &HashMap<PathBuf, FileInfo>,
) -> Result<HashMap<PathBuf, FileInfo>, ContextCreatorError> {
    expand_file_list_internal(
        files_map,
        config,
        cache,
        walk_options,
        Some(all_files_context),
    )
}

/// Expand file list based on semantic relationships
///
/// This function takes the initial set of files and expands it to include
/// files that define types, export functions, or are imported by the initial files.
pub fn expand_file_list(
    files_map: HashMap<PathBuf, FileInfo>,
    config: &Config,
    cache: &Arc<FileCache>,
    walk_options: &crate::core::walker::WalkOptions,
) -> Result<HashMap<PathBuf, FileInfo>, ContextCreatorError> {
    expand_file_list_internal(files_map, config, cache, walk_options, None)
}

/// Internal implementation of expand_file_list with optional context
fn expand_file_list_internal(
    files_map: HashMap<PathBuf, FileInfo>,
    config: &Config,
    cache: &Arc<FileCache>,
    walk_options: &crate::core::walker::WalkOptions,
    all_files_context: Option<&HashMap<PathBuf, FileInfo>>,
) -> Result<HashMap<PathBuf, FileInfo>, ContextCreatorError> {
    // If no semantic features are enabled, return as-is
    if !config.trace_imports && !config.include_callers && !config.include_types {
        return Ok(files_map);
    }

    let mut files_map = files_map;

    // Detect the project root for secure path validation
    let project_root = if let Some((first_path, _)) = files_map.iter().next() {
        detect_project_root(first_path)
    } else {
        // If no files, use current directory
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    };

    // First, perform semantic analysis on the initial files if needed
    if config.trace_imports || config.include_types {
        use crate::core::semantic::analyzer::SemanticContext;
        use crate::core::semantic::get_analyzer_for_file;

        for (path, file_info) in files_map.iter_mut() {
            // Skip if already analyzed (has imports or type references)
            if !file_info.imports.is_empty() || !file_info.type_references.is_empty() {
                continue;
            }

            // Read file content and analyze
            if let Ok(content) = cache.get_or_load(path) {
                if let Ok(Some(analyzer)) = get_analyzer_for_file(path) {
                    let context = SemanticContext {
                        current_file: path.clone(),
                        base_dir: project_root.clone(),
                        current_depth: 0,
                        max_depth: config.semantic_depth,
                        visited_files: HashSet::new(),
                    };

                    if let Ok(analysis) = analyzer.analyze_file(path, &content, &context) {
                        eprintln!(
                            "[DEBUG] Analyzed {} - {} imports found",
                            path.display(),
                            analysis.imports.len()
                        );
                        for imp in &analysis.imports {
                            eprintln!("[DEBUG]   Import: {} (items: {:?})", imp.module, imp.items);
                        }
                        // Convert imports to resolved file paths
                        file_info.imports = analysis
                            .imports
                            .iter()
                            .filter_map(|imp| {
                                // Try to resolve import to file path
                                resolve_import_to_path(&imp.module, path, &project_root)
                            })
                            .collect();
                        file_info.function_calls = analysis.function_calls;
                        file_info.type_references = analysis.type_references;

                        tracing::debug!(
                            "Initial file {} analyzed: {} imports, {} types, {} calls",
                            path.display(),
                            file_info.imports.len(),
                            file_info.type_references.len(),
                            file_info.function_calls.len()
                        );
                    }
                }
            }
        }
    }

    // Create work queue and visited set for BFS traversal
    let mut work_queue = VecDeque::new();
    let mut visited_paths = HashSet::new();
    let mut files_to_add = Vec::new();

    // Initialize with files that have semantic relationships
    for (path, file_info) in &files_map {
        visited_paths.insert(path.clone());

        // Queue files based on enabled features (depth 0 for initial files)
        if config.include_types && !file_info.type_references.is_empty() {
            work_queue.push_back((path.clone(), file_info.clone(), ExpansionReason::Types, 0));
        }
        if config.trace_imports && !file_info.imports.is_empty() {
            tracing::debug!(
                "Queuing {} for import expansion (has {} imports)",
                path.display(),
                file_info.imports.len()
            );
            work_queue.push_back((path.clone(), file_info.clone(), ExpansionReason::Imports, 0));
        }
    }

    // Optimized caller expansion using pre-built index (O(n) instead of O(n²))
    if config.include_callers {
        // Create walk options for caller search that searches all files
        // but still respects ignore patterns for security
        let mut caller_walk_options = walk_options.clone();
        caller_walk_options.include_patterns.clear(); // Search all files for callers

        // Only perform project-wide analysis once
        let mut project_files =
            walk_directory(&project_root, caller_walk_options).map_err(|e| {
                ContextCreatorError::ParseError(format!("Failed to walk directory: {e}"))
            })?;

        // If no files found in project, return early
        if project_files.is_empty() {
            return Ok(files_map);
        }

        // Perform semantic analysis to get function calls and exports
        use crate::core::semantic_graph::perform_semantic_analysis_graph;
        perform_semantic_analysis_graph(&mut project_files, config, cache).map_err(|e| {
            ContextCreatorError::ParseError(format!("Failed to analyze files: {e}"))
        })?;

        // Build function call index for O(1) lookups
        let function_call_index = FunctionCallIndex::build(&project_files);

        // Find all callers of functions exported by our initial files
        let initial_files: Vec<PathBuf> = files_map.keys().cloned().collect();
        let caller_paths = function_call_index.find_callers_of_files(&initial_files);

        // Add caller files while respecting security boundaries
        for caller_path in caller_paths {
            if !visited_paths.contains(&caller_path) {
                // For caller expansion, we intentionally expand beyond the original include patterns
                // This is the purpose of the --include-callers feature
                // However, we still respect ignore patterns for security
                let should_ignore = walk_options.ignore_patterns.iter().any(|pattern| {
                    glob::Pattern::new(pattern)
                        .ok()
                        .is_some_and(|p| p.matches_path(&caller_path))
                });

                if !should_ignore {
                    // Find the file info from analyzed project files
                    if let Some(caller_info) = project_files
                        .iter()
                        .find(|f| f.path == caller_path)
                        .cloned()
                    {
                        visited_paths.insert(caller_path.clone());
                        files_to_add.push((caller_path, caller_info));
                    }
                }
            }
        }
    }

    // Create type resolver with circuit breakers
    let resolution_limits = ResolutionLimits {
        max_depth: config.semantic_depth,
        max_visited_types: 1000, // Conservative limit
        max_resolution_time: std::time::Duration::from_secs(30),
    };
    let mut type_resolver = TypeResolver::with_limits(resolution_limits);

    // Process work queue
    // Note: Cycle prevention is handled by visited_paths HashSet which tracks all processed files.
    // This prevents infinite loops in cases like A→B→C→A by not revisiting already processed files.
    while let Some((source_path, source_file, reason, depth)) = work_queue.pop_front() {
        // Check if we've exceeded the semantic depth limit
        if depth > config.semantic_depth {
            tracing::debug!(
                "Skipping {} (depth {} > limit {})",
                source_path.display(),
                depth,
                config.semantic_depth
            );
            continue;
        }

        match reason {
            ExpansionReason::Types => {
                // Process type references
                tracing::debug!(
                    "Processing type references from {} (depth {})",
                    source_path.display(),
                    depth
                );
                for type_ref in &source_file.type_references {
                    // Skip external types
                    if type_ref.is_external {
                        continue;
                    }

                    // Create a local copy to potentially fix the module path
                    let mut type_ref_copy = type_ref.clone();

                    // Fix module path if it contains the type name
                    if let Some(ref module) = type_ref_copy.module {
                        if module.ends_with(&format!("::{}", type_ref_copy.name)) {
                            // Remove the redundant type name from the module path
                            let corrected_module = module
                                .strip_suffix(&format!("::{}", type_ref_copy.name))
                                .unwrap_or(module);
                            type_ref_copy.module = Some(corrected_module.to_string());
                        }
                    }

                    let type_ref = &type_ref_copy;

                    tracing::debug!(
                        "  Type reference: {} (module: {:?}, definition_path: {:?}, is_external: {})",
                        type_ref.name,
                        type_ref.module,
                        type_ref.definition_path,
                        type_ref.is_external
                    );

                    // Use type resolver with circuit breakers
                    match type_resolver.resolve_with_limits(type_ref, depth) {
                        Err(e) => {
                            // Circuit breaker triggered or error - skip this type
                            if config.verbose > 0 {
                                eprintln!("⚠️  Type resolution limited: {e}");
                            }
                            continue;
                        }
                        Ok(_) => {
                            // Resolution succeeded, continue with normal processing
                        }
                    }

                    // If we have a definition path, add it
                    if let Some(ref def_path) = type_ref.definition_path {
                        tracing::debug!("    Type has definition_path: {}", def_path.display());
                        if !visited_paths.contains(def_path) && def_path.exists() {
                            // Validate the path for security using the project root
                            match validate_import_path(&project_root, def_path) {
                                Ok(validated_path) => {
                                    tracing::debug!(
                                        "    Adding type definition file: {}",
                                        validated_path.display()
                                    );
                                    visited_paths.insert(validated_path.clone());

                                    // Create FileInfo for the definition file
                                    let mut file_info =
                                        create_file_info_for_path(&validated_path, &source_path)?;

                                    // Perform semantic analysis on the newly found file to get its type references
                                    if depth + 1 < config.semantic_depth {
                                        if let Ok(content) = cache.get_or_load(&validated_path) {
                                            use crate::core::semantic::analyzer::SemanticContext;
                                            use crate::core::semantic::get_analyzer_for_file;

                                            if let Ok(Some(analyzer)) =
                                                get_analyzer_for_file(&validated_path)
                                            {
                                                let context = SemanticContext::new(
                                                    validated_path.clone(),
                                                    project_root.clone(),
                                                    config.semantic_depth,
                                                );

                                                if let Ok(analysis) = analyzer.analyze_file(
                                                    &validated_path,
                                                    &content,
                                                    &context,
                                                ) {
                                                    // Update file info with semantic data
                                                    file_info.type_references =
                                                        analysis.type_references;

                                                    // Queue for type expansion if it has type references
                                                    if !file_info.type_references.is_empty() {
                                                        work_queue.push_back((
                                                            validated_path.clone(),
                                                            file_info.clone(),
                                                            ExpansionReason::Types,
                                                            depth + 1,
                                                        ));
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    files_to_add.push((validated_path.clone(), file_info));
                                }
                                Err(_) => {
                                    // Path validation failed, skip this file
                                    tracing::debug!(
                                        "    Path validation failed for: {}",
                                        def_path.display()
                                    );
                                }
                            }
                        }
                    } else {
                        tracing::debug!(
                            "    No definition_path, attempting to find type definition file"
                        );
                        // Try to find the type definition file
                        // Use the full module path
                        let module_name = type_ref.module.as_deref();

                        tracing::debug!(
                            "    Looking for type {} with module {:?}",
                            type_ref.name,
                            module_name
                        );
                        if let Some(def_path) = find_type_definition_file(
                            &type_ref.name,
                            module_name,
                            &source_path,
                            cache,
                        ) {
                            tracing::debug!(
                                "    Found type definition file: {}",
                                def_path.display()
                            );
                            if !visited_paths.contains(&def_path) {
                                // Validate the path for security using the project root
                                match validate_import_path(&project_root, &def_path) {
                                    Ok(validated_path) => {
                                        tracing::debug!(
                                            "    Adding found type definition file: {}",
                                            validated_path.display()
                                        );
                                        visited_paths.insert(validated_path.clone());

                                        // Create FileInfo for the definition file
                                        let mut file_info = create_file_info_for_path(
                                            &validated_path,
                                            &source_path,
                                        )?;

                                        // Perform semantic analysis on the newly found file to get its type references
                                        if depth + 1 < config.semantic_depth {
                                            if let Ok(content) = cache.get_or_load(&validated_path)
                                            {
                                                use crate::core::semantic::analyzer::SemanticContext;
                                                use crate::core::semantic::get_analyzer_for_file;

                                                if let Ok(Some(analyzer)) =
                                                    get_analyzer_for_file(&validated_path)
                                                {
                                                    let context = SemanticContext::new(
                                                        validated_path.clone(),
                                                        project_root.clone(),
                                                        config.semantic_depth,
                                                    );

                                                    if let Ok(analysis) = analyzer.analyze_file(
                                                        &validated_path,
                                                        &content,
                                                        &context,
                                                    ) {
                                                        // Update file info with semantic data
                                                        file_info.type_references =
                                                            analysis.type_references;

                                                        // Queue for type expansion if it has type references
                                                        if !file_info.type_references.is_empty() {
                                                            work_queue.push_back((
                                                                validated_path.clone(),
                                                                file_info.clone(),
                                                                ExpansionReason::Types,
                                                                depth + 1,
                                                            ));
                                                        }
                                                    }
                                                }
                                            }
                                        }

                                        files_to_add.push((validated_path, file_info));
                                    }
                                    Err(_) => {
                                        // Path validation failed, skip this file
                                        tracing::debug!(
                                            "    Path validation failed for found file: {}",
                                            def_path.display()
                                        );
                                    }
                                }
                            }
                        } else {
                            tracing::debug!(
                                "    Could not find type definition file for: {}",
                                type_ref.name
                            );
                        }
                    }
                }
            }
            ExpansionReason::Imports => {
                // Process each import in the source file
                if config.verbose > 0 {
                    eprintln!("[DEBUG] Processing imports from {}", source_path.display());
                    eprintln!("[DEBUG]   Import count: {}", source_file.imports.len());
                    for imp in &source_file.imports {
                        eprintln!("[DEBUG]   -> {}", imp.display());
                    }
                }
                for import_path in &source_file.imports {
                    // Skip if doesn't exist
                    if !import_path.exists() {
                        continue;
                    }

                    // Check if already visited (need to check both original and canonical paths)
                    let canonical_import = import_path
                        .canonicalize()
                        .unwrap_or_else(|_| import_path.clone());
                    if visited_paths.contains(import_path)
                        || visited_paths.contains(&canonical_import)
                    {
                        continue;
                    }

                    // Validate the import path for security
                    match validate_import_path(&project_root, import_path) {
                        Ok(validated_path) => {
                            visited_paths.insert(validated_path.clone());

                            // For Rust files, if we're importing a module, also include lib.rs
                            if source_path.extension() == Some(std::ffi::OsStr::new("rs")) {
                                // Check if this is a module file (not main.rs or lib.rs itself)
                                if let Some(parent) = validated_path.parent() {
                                    let lib_rs = parent.join("lib.rs");
                                    if lib_rs.exists()
                                        && lib_rs != validated_path
                                        && !visited_paths.contains(&lib_rs)
                                    {
                                        // Check if lib.rs declares this module
                                        if let Ok(lib_content) = cache.get_or_load(&lib_rs) {
                                            let module_name = validated_path
                                                .file_stem()
                                                .and_then(|s| s.to_str())
                                                .unwrap_or("");
                                            if lib_content.contains(&format!("mod {module_name};"))
                                                || lib_content
                                                    .contains(&format!("pub mod {module_name};"))
                                            {
                                                // Add lib.rs to files to include
                                                visited_paths.insert(lib_rs.clone());
                                                if let Some(context) = all_files_context {
                                                    if let Some(lib_file) = context.get(&lib_rs) {
                                                        files_to_add.push((
                                                            lib_rs.clone(),
                                                            lib_file.clone(),
                                                        ));
                                                    }
                                                } else {
                                                    let lib_info = create_file_info_for_path(
                                                        &lib_rs,
                                                        &source_path,
                                                    )?;
                                                    files_to_add.push((lib_rs, lib_info));
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            // Check if we have this file in the context first
                            let mut file_info = if let Some(context) = all_files_context {
                                // Try to canonicalize for lookup, but fall back to validated_path
                                let lookup_path = validated_path
                                    .canonicalize()
                                    .unwrap_or_else(|_| validated_path.clone());

                                // Also try the non-canonical path
                                let context_file = context
                                    .get(&lookup_path)
                                    .or_else(|| context.get(&validated_path));

                                if let Some(context_file) = context_file {
                                    // Use the pre-analyzed file from context
                                    let mut file = context_file.clone();
                                    // Mark that this file was imported by the source file
                                    file.imported_by.push(source_path.clone());
                                    file
                                } else {
                                    // Create FileInfo for the imported file
                                    let mut file =
                                        create_file_info_for_path(&validated_path, &source_path)?;
                                    file.imported_by.push(source_path.clone());
                                    file
                                }
                            } else {
                                // No context, create from scratch
                                let mut file =
                                    create_file_info_for_path(&validated_path, &source_path)?;
                                file.imported_by.push(source_path.clone());
                                file
                            };

                            // Queue for next depth level if within limits
                            if depth + 1 < config.semantic_depth {
                                // If we already have semantic data from context, use it
                                if !file_info.imports.is_empty()
                                    || !file_info.type_references.is_empty()
                                    || !file_info.function_calls.is_empty()
                                {
                                    // Already analyzed, just queue if needed
                                    if !file_info.imports.is_empty() {
                                        work_queue.push_back((
                                            validated_path.clone(),
                                            file_info.clone(),
                                            ExpansionReason::Imports,
                                            depth + 1,
                                        ));
                                    }
                                    if config.include_types && !file_info.type_references.is_empty()
                                    {
                                        work_queue.push_back((
                                            validated_path.clone(),
                                            file_info.clone(),
                                            ExpansionReason::Types,
                                            depth + 1,
                                        ));
                                    }
                                } else if let Ok(content) = cache.get_or_load(&validated_path) {
                                    // Perform semantic analysis on the imported file
                                    use crate::core::semantic::analyzer::SemanticContext;
                                    use crate::core::semantic::get_analyzer_for_file;

                                    if let Ok(Some(analyzer)) =
                                        get_analyzer_for_file(&validated_path)
                                    {
                                        let context = SemanticContext::new(
                                            validated_path.clone(),
                                            project_root.clone(),
                                            config.semantic_depth,
                                        );

                                        if let Ok(analysis) = analyzer.analyze_file(
                                            &validated_path,
                                            &content,
                                            &context,
                                        ) {
                                            // Update file info with semantic data
                                            file_info.imports = analysis
                                                .imports
                                                .iter()
                                                .filter_map(|imp| {
                                                    // Try to resolve import to file path
                                                    resolve_import_to_path(
                                                        &imp.module,
                                                        &validated_path,
                                                        &project_root,
                                                    )
                                                })
                                                .collect();
                                            file_info.function_calls = analysis.function_calls;
                                            file_info.type_references = analysis.type_references;

                                            // Queue if it has imports
                                            if !file_info.imports.is_empty() {
                                                work_queue.push_back((
                                                    validated_path.clone(),
                                                    file_info.clone(),
                                                    ExpansionReason::Imports,
                                                    depth + 1,
                                                ));
                                            }
                                        }
                                    }
                                }
                            }

                            files_to_add.push((validated_path, file_info));
                        }
                        Err(_) => {
                            // Path validation failed, skip this import
                            if config.verbose > 0 {
                                eprintln!(
                                    "⚠️  Skipping invalid import path: {}",
                                    import_path.display()
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    // Add new files to the map
    for (path, file_info) in files_to_add {
        files_map.insert(path, file_info);
    }

    // Update imported_by relationships for proper prioritization
    update_import_relationships(&mut files_map);

    // Build ignore matcher for efficient filtering
    if let Some(ignore_matcher) = build_ignore_matcher(walk_options, &project_root) {
        // Remove ignored files from the final output
        // This is extremely efficient as the ignore crate uses optimized algorithms
        let ignored_files: Vec<PathBuf> = files_map
            .keys()
            .filter(|path| {
                // The ignore crate's Match type indicates if a path should be ignored
                ignore_matcher.matched(path, path.is_dir()).is_ignore()
            })
            .cloned()
            .collect();

        for ignored_path in ignored_files {
            files_map.remove(&ignored_path);
        }
    }

    Ok(files_map)
}

/// Reason for expanding to include a file
#[derive(Debug, Clone, Copy)]
enum ExpansionReason {
    Types,
    Imports,
}

/// Create a basic FileInfo for a newly discovered file
fn create_file_info_for_path(
    path: &PathBuf,
    source_path: &Path,
) -> Result<FileInfo, ContextCreatorError> {
    use crate::utils::file_ext::FileType;
    use std::fs;

    let metadata = fs::metadata(path)?;
    let file_type = FileType::from_path(path);

    // Calculate relative path from common ancestor
    let relative_path = path
        .strip_prefix(common_ancestor(path, source_path))
        .unwrap_or(path)
        .to_path_buf();

    Ok(FileInfo {
        path: path.clone(),
        relative_path,
        size: metadata.len(),
        file_type,
        priority: 1.0, // Default priority, will be adjusted by prioritizer
        imports: Vec::new(),
        imported_by: vec![source_path.to_path_buf()], // Track who caused this file to be included
        function_calls: Vec::new(),
        type_references: Vec::new(),
        exported_functions: Vec::new(),
    })
}

/// Find the lowest common ancestor (LCA) of two paths using a proper set-based approach
fn common_ancestor(path1: &Path, path2: &Path) -> PathBuf {
    use std::collections::HashSet;

    // Collect all ancestors of path1 into a set for efficient lookup
    let ancestors1: HashSet<&Path> = path1.ancestors().collect();

    // Find the first ancestor of path2 that is also in ancestors1
    // This will be the lowest common ancestor since ancestors() returns in order from leaf to root
    for ancestor in path2.ancestors() {
        if ancestors1.contains(ancestor) {
            return ancestor.to_path_buf();
        }
    }

    // If no common ancestor found (shouldn't happen in normal filesystem),
    // fallback to appropriate root for the platform
    #[cfg(windows)]
    {
        // On Windows, try to get the root from one of the paths
        if let Some(root) = path1.ancestors().last() {
            root.to_path_buf()
        } else {
            PathBuf::from("C:\\")
        }
    }
    #[cfg(not(windows))]
    {
        PathBuf::from("/")
    }
}

/// Check if a file contains a definition for a given type name using AST parsing.
fn file_contains_definition(path: &Path, content: &str, type_name: &str) -> bool {
    // Determine the language from the file extension.
    let language = match path.extension().and_then(|s| s.to_str()) {
        Some("rs") => Some(tree_sitter_rust::language()),
        Some("py") => Some(tree_sitter_python::language()),
        Some("ts") | Some("tsx") => Some(tree_sitter_typescript::language_typescript()),
        Some("js") | Some("jsx") => Some(tree_sitter_javascript::language()),
        _ => None,
    };

    if let Some(language) = language {
        let mut parser = tree_sitter::Parser::new();
        if parser.set_language(language).is_err() {
            return false;
        }

        if let Some(tree) = parser.parse(content, None) {
            // Language-specific queries for type definitions (without predicates)
            let query_text = match path.extension().and_then(|s| s.to_str()) {
                Some("rs") => {
                    r#"
                    [
                      (struct_item name: (type_identifier) @name)
                      (enum_item name: (type_identifier) @name)
                      (trait_item name: (type_identifier) @name)
                      (type_item name: (type_identifier) @name)
                      (union_item name: (type_identifier) @name)
                    ]
                "#
                }
                Some("py") => {
                    r#"
                    [
                      (class_definition name: (identifier) @name)
                      (function_definition name: (identifier) @name)
                    ]
                "#
                }
                Some("ts") | Some("tsx") => {
                    r#"
                    [
                      (interface_declaration name: (type_identifier) @name)
                      (type_alias_declaration name: (type_identifier) @name)
                      (class_declaration name: (type_identifier) @name)
                      (enum_declaration name: (identifier) @name)
                    ]
                "#
                }
                Some("js") | Some("jsx") => {
                    r#"
                    [
                      (class_declaration name: (identifier) @name)
                      (function_declaration name: (identifier) @name)
                    ]
                "#
                }
                _ => return false,
            };

            if let Ok(query) = tree_sitter::Query::new(language, query_text) {
                let mut cursor = tree_sitter::QueryCursor::new();
                let matches = cursor.matches(&query, tree.root_node(), content.as_bytes());

                // Check each match to see if the captured name matches our target type
                for m in matches {
                    for capture in m.captures {
                        if let Ok(captured_text) = capture.node.utf8_text(content.as_bytes()) {
                            if captured_text == type_name {
                                return true;
                            }
                        }
                    }
                }
                return false;
            }
        }
    }
    false
}

/// Find a type definition file by searching nearby paths
fn find_type_definition_file(
    type_name: &str,
    module_name: Option<&str>,
    source_file: &Path,
    cache: &FileCache,
) -> Option<PathBuf> {
    tracing::debug!(
        "find_type_definition_file: type_name={}, module_name={:?}, source_file={}",
        type_name,
        module_name,
        source_file.display()
    );
    // Get the directory of the source file
    let source_dir = source_file.parent()?;

    // Also get the project root (go up to find src directory)
    let mut project_root = source_dir;
    while let Some(parent) = project_root.parent() {
        // If we find a Cargo.toml or src directory, the parent is likely the project root
        if parent.join("Cargo.toml").exists() || parent.join("src").exists() {
            project_root = parent;
            break;
        }
        // If current dir is named "src", its parent is likely the project root
        if project_root.file_name() == Some(std::ffi::OsStr::new("src")) {
            project_root = parent;
            break;
        }
        project_root = parent;
    }

    // Convert type name to lowercase for file matching
    let type_name_lower = type_name.to_lowercase();

    // Common patterns for type definition files
    let mut patterns = vec![
        // Direct file name matches
        format!("{type_name_lower}.rs"),
        format!("{type_name_lower}.py"),
        format!("{type_name_lower}.ts"),
        format!("{type_name_lower}.js"),
        format!("{type_name_lower}.tsx"),
        format!("{type_name_lower}.jsx"),
        // Types files
        "types.rs".to_string(),
        "types.py".to_string(),
        "types.ts".to_string(),
        "types.js".to_string(),
        // Module files
        "mod.rs".to_string(),
        "index.ts".to_string(),
        "index.js".to_string(),
        "__init__.py".to_string(),
        // Common type definition patterns
        format!("{type_name_lower}_types.rs"),
        format!("{type_name_lower}_type.rs"),
        format!("{type_name_lower}s.rs"), // plural form
    ];

    // If we have a module name, add module-based patterns
    if let Some(module) = module_name {
        // Handle Rust module paths like "crate::models"
        if module.starts_with("crate::") {
            let relative_path = module.strip_prefix("crate::").unwrap();
            // Convert module path to file path (e.g., "models" or "domain::types")
            let module_path = relative_path.replace("::", "/");

            // For crate:: paths, we need to look in the src directory
            patterns.insert(0, format!("src/{module_path}.rs"));
            patterns.insert(1, format!("src/{module_path}/mod.rs"));
            patterns.insert(2, format!("{module_path}.rs"));
            patterns.insert(3, format!("{module_path}/mod.rs"));
        } else if module.contains("::") {
            // Handle other module paths with ::
            let module_path = module.replace("::", "/");
            patterns.insert(0, format!("{module_path}.rs"));
            patterns.insert(1, format!("{module_path}/mod.rs"));
        } else {
            // Simple module names
            let module_lower = module.to_lowercase();
            patterns.insert(0, format!("{module_lower}.rs"));
            patterns.insert(1, format!("{module_lower}.py"));
            patterns.insert(2, format!("{module_lower}.ts"));
            patterns.insert(3, format!("{module_lower}.js"));
            patterns.insert(4, format!("{module_lower}.tsx"));
            patterns.insert(5, format!("{module_lower}.jsx"));
            patterns.insert(6, format!("{module}.rs")); // Also try original case
            patterns.insert(7, format!("{module}.py"));
            patterns.insert(8, format!("{module}.ts"));
            patterns.insert(9, format!("{module}.js"));
        }
    }

    // Search in current directory first
    for pattern in &patterns {
        let candidate = source_dir.join(pattern);
        if candidate.exists() {
            // Read the file to verify it contains the type definition
            if let Ok(content) = cache.get_or_load(&candidate) {
                // Use AST-based validation to check for type definitions
                if file_contains_definition(&candidate, &content, type_name) {
                    return Some(candidate);
                }
            }
        }
    }

    // Search in parent directory
    if let Some(parent_dir) = source_dir.parent() {
        for pattern in &patterns {
            let candidate = parent_dir.join(pattern);
            if candidate.exists() {
                if let Ok(content) = cache.get_or_load(&candidate) {
                    if file_contains_definition(&candidate, &content, type_name) {
                        return Some(candidate);
                    }
                }
            }
        }
    }

    // Search in common module directories relative to project root
    let search_dirs = vec![
        project_root.to_path_buf(),
        project_root.join("src"),
        project_root.join("src/models"),
        project_root.join("src/types"),
        project_root.join("shared"),
        project_root.join("shared/types"),
        project_root.join("lib"),
        project_root.join("domain"),
        source_dir.join("models"),
        source_dir.join("types"),
    ];

    for search_dir in search_dirs {
        if search_dir.exists() {
            for pattern in &patterns {
                let candidate = search_dir.join(pattern);
                if candidate.exists() {
                    if let Ok(content) = cache.get_or_load(&candidate) {
                        if file_contains_definition(&candidate, &content, type_name) {
                            return Some(candidate);
                        }
                    }
                }
            }
        }
    }

    None
}

/// Resolve an import module name to a file path
fn resolve_import_to_path(
    module_name: &str,
    importing_file: &Path,
    project_root: &Path,
) -> Option<PathBuf> {
    // Use the semantic module resolver system
    use crate::core::semantic::get_module_resolver_for_file;

    // Get the appropriate resolver for this file type
    let resolver = match get_module_resolver_for_file(importing_file) {
        Ok(Some(r)) => r,
        _ => {
            // No resolver available, fall back to simple resolution
            let source_dir = importing_file.parent()?;

            // Handle relative imports (Python style: ".", "..", "..sibling")
            if module_name.starts_with('.') {
                return resolve_relative_import(module_name, source_dir, project_root);
            }

            // Language-specific resolution based on file extension
            return match importing_file.extension().and_then(|s| s.to_str()) {
                Some("rs") => resolve_rust_import(module_name, source_dir, project_root),
                Some("py") => resolve_python_import(module_name, source_dir, project_root),
                Some("js") | Some("jsx") => {
                    resolve_javascript_import(module_name, source_dir, project_root)
                }
                Some("ts") | Some("tsx") => {
                    resolve_typescript_import(module_name, source_dir, project_root)
                }
                Some("go") => resolve_go_import(module_name, source_dir, project_root),
                _ => None,
            };
        }
    };

    // Resolve the import
    match resolver.resolve_import(module_name, importing_file, project_root) {
        Ok(resolved) => {
            if resolved.is_external {
                // Skip external modules
                None
            } else {
                Some(resolved.path)
            }
        }
        Err(_) => {
            // Fallback to simple resolution for backwards compatibility
            let source_dir = importing_file.parent()?;

            // Handle relative imports (Python style: ".", "..", "..sibling")
            if module_name.starts_with('.') {
                return resolve_relative_import(module_name, source_dir, project_root);
            }

            // Language-specific resolution based on file extension
            match importing_file.extension().and_then(|s| s.to_str()) {
                Some("rs") => resolve_rust_import(module_name, source_dir, project_root),
                Some("py") => resolve_python_import(module_name, source_dir, project_root),
                Some("js") | Some("jsx") => {
                    resolve_javascript_import(module_name, source_dir, project_root)
                }
                Some("ts") | Some("tsx") => {
                    resolve_typescript_import(module_name, source_dir, project_root)
                }
                Some("go") => resolve_go_import(module_name, source_dir, project_root),
                _ => None,
            }
        }
    }
}

/// Resolve relative imports (e.g., "..", ".", "../sibling")
fn resolve_relative_import(
    module_name: &str,
    source_dir: &Path,
    _project_root: &Path,
) -> Option<PathBuf> {
    let mut path = source_dir.to_path_buf();

    // Count leading dots
    let dots: Vec<&str> = module_name.split('/').collect();
    if dots.is_empty() {
        return None;
    }

    // Handle ".." for parent directory
    for part in &dots {
        if *part == ".." {
            path = path.parent()?.to_path_buf();
        } else if *part == "." {
            // Stay in current directory
        } else {
            // This is the actual module name after dots
            path = path.join(part);
            break;
        }
    }

    // Try common file extensions
    for ext in &["py", "js", "ts", "rs"] {
        let file_path = path.with_extension(ext);
        if file_path.exists() {
            return Some(file_path);
        }
    }

    // Try as directory with index/mod/__init__ files
    if path.is_dir() {
        for index_file in &["__init__.py", "index.js", "index.ts", "mod.rs"] {
            let index_path = path.join(index_file);
            if index_path.exists() {
                return Some(index_path);
            }
        }
    }

    None
}

/// Resolve Rust module imports
fn resolve_rust_import(
    module_name: &str,
    source_dir: &Path,
    project_root: &Path,
) -> Option<PathBuf> {
    // Handle crate:: prefix
    let module_path = if module_name.starts_with("crate::") {
        module_name.strip_prefix("crate::").unwrap()
    } else {
        module_name
    };

    // Convert module path to file path (e.g., "foo::bar" -> "foo/bar")
    let parts: Vec<&str> = module_path.split("::").collect();

    // Try in source directory first
    let mut path = source_dir.to_path_buf();
    for part in &parts {
        path = path.join(part);
    }

    // Try as .rs file
    let rs_file = path.with_extension("rs");
    if rs_file.exists() {
        return Some(rs_file);
    }

    // Try as mod.rs in directory
    let mod_file = path.join("mod.rs");
    if mod_file.exists() {
        return Some(mod_file);
    }

    // Try from project root src directory
    let src_path = project_root.join("src");
    if src_path.exists() {
        let mut path = src_path;
        for part in &parts {
            path = path.join(part);
        }

        let rs_file = path.with_extension("rs");
        if rs_file.exists() {
            return Some(rs_file);
        }

        let mod_file = path.join("mod.rs");
        if mod_file.exists() {
            return Some(mod_file);
        }
    }

    None
}

/// Resolve Python module imports
fn resolve_python_import(
    module_name: &str,
    source_dir: &Path,
    project_root: &Path,
) -> Option<PathBuf> {
    // Convert module path to file path (e.g., "foo.bar" -> "foo/bar")
    let parts: Vec<&str> = module_name.split('.').collect();

    // Try from source directory
    let mut path = source_dir.to_path_buf();
    for part in &parts {
        path = path.join(part);
    }

    // Try as .py file
    let py_file = path.with_extension("py");
    if py_file.exists() {
        return Some(py_file);
    }

    // Try as __init__.py in directory
    let init_file = path.join("__init__.py");
    if init_file.exists() {
        return Some(init_file);
    }

    // Try from project root
    let mut path = project_root.to_path_buf();
    for part in &parts {
        path = path.join(part);
    }

    let py_file = path.with_extension("py");
    if py_file.exists() {
        return Some(py_file);
    }

    let init_file = path.join("__init__.py");
    if init_file.exists() {
        return Some(init_file);
    }

    None
}

/// Resolve JavaScript module imports
fn resolve_javascript_import(
    module_name: &str,
    source_dir: &Path,
    _project_root: &Path,
) -> Option<PathBuf> {
    // Handle relative paths
    if module_name.starts_with("./") || module_name.starts_with("../") {
        let path = source_dir.join(module_name);

        // Try exact path first
        if path.exists() {
            return Some(path);
        }

        // Try with .js extension
        let js_file = path.with_extension("js");
        if js_file.exists() {
            return Some(js_file);
        }

        // Try with .jsx extension
        let jsx_file = path.with_extension("jsx");
        if jsx_file.exists() {
            return Some(jsx_file);
        }

        // Try as directory with index.js
        let index_file = path.join("index.js");
        if index_file.exists() {
            return Some(index_file);
        }
    }

    // For non-relative imports, they're likely npm modules - skip
    None
}

/// Resolve TypeScript module imports
fn resolve_typescript_import(
    module_name: &str,
    source_dir: &Path,
    _project_root: &Path,
) -> Option<PathBuf> {
    // Handle relative paths
    if module_name.starts_with("./") || module_name.starts_with("../") {
        let path = source_dir.join(module_name);

        // Try exact path first
        if path.exists() {
            return Some(path);
        }

        // Try with .ts extension
        let ts_file = path.with_extension("ts");
        if ts_file.exists() {
            return Some(ts_file);
        }

        // Try with .tsx extension
        let tsx_file = path.with_extension("tsx");
        if tsx_file.exists() {
            return Some(tsx_file);
        }

        // Try as directory with index.ts
        let index_file = path.join("index.ts");
        if index_file.exists() {
            return Some(index_file);
        }
    }

    // For non-relative imports, they're likely npm modules - skip
    None
}

/// Resolve Go module imports
fn resolve_go_import(
    module_name: &str,
    _source_dir: &Path,
    project_root: &Path,
) -> Option<PathBuf> {
    // Go imports are typically package-based
    // Skip external packages (those with dots in the first part usually)
    if module_name.contains('/') && module_name.split('/').next()?.contains('.') {
        return None; // External package
    }

    // Try to find in project
    let parts: Vec<&str> = module_name.split('/').collect();
    let mut path = project_root.to_path_buf();

    for part in parts {
        path = path.join(part);
    }

    // Go files in a directory form a package
    if path.is_dir() {
        // Return the first .go file in the directory (excluding tests)
        if let Ok(entries) = std::fs::read_dir(&path) {
            for entry in entries.flatten() {
                let file_path = entry.path();
                if file_path.extension() == Some(std::ffi::OsStr::new("go")) {
                    let file_name = file_path.file_name()?.to_string_lossy();
                    if !file_name.ends_with("_test.go") {
                        return Some(file_path);
                    }
                }
            }
        }
    }

    None
}

/// Update import relationships after expansion
fn update_import_relationships(files_map: &mut HashMap<PathBuf, FileInfo>) {
    // Build a map of which files import which
    let mut import_map: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();

    for (path, file_info) in files_map.iter() {
        for import_path in &file_info.imports {
            // Track which files import which
            import_map
                .entry(import_path.clone())
                .or_default()
                .push(path.clone());
        }
    }

    // Update imported_by fields
    for (imported_path, importers) in import_map {
        if let Some(file_info) = files_map.get_mut(&imported_path) {
            file_info.imported_by.extend(importers);
            file_info.imported_by.sort();
            file_info.imported_by.dedup();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::file_ext::FileType;

    #[test]
    fn test_no_expansion_when_disabled() {
        let mut files_map = HashMap::new();
        files_map.insert(
            PathBuf::from("test.rs"),
            FileInfo {
                path: PathBuf::from("test.rs"),
                relative_path: PathBuf::from("test.rs"),
                size: 100,
                file_type: FileType::Rust,
                priority: 1.0,
                imports: Vec::new(),
                imported_by: Vec::new(),
                function_calls: Vec::new(),
                type_references: Vec::new(),
                exported_functions: Vec::new(),
            },
        );

        let config = Config {
            trace_imports: false,
            include_callers: false,
            include_types: false,
            ..Default::default()
        };

        let cache = Arc::new(FileCache::new());
        let walk_options = crate::core::walker::WalkOptions {
            max_file_size: None,
            follow_links: false,
            include_hidden: false,
            parallel: false,
            ignore_file: ".context-creator-ignore".to_string(),
            ignore_patterns: vec![],
            include_patterns: vec![],
            custom_priorities: vec![],
            filter_binary_files: false,
        };
        let result = expand_file_list(files_map.clone(), &config, &cache, &walk_options).unwrap();

        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_common_ancestor() {
        #[cfg(windows)]
        {
            let path1 = PathBuf::from("C:\\Users\\user\\project\\src\\main.rs");
            let path2 = PathBuf::from("C:\\Users\\user\\project\\lib\\util.rs");
            let ancestor = common_ancestor(&path1, &path2);
            assert_eq!(ancestor, PathBuf::from("C:\\Users\\user\\project"));

            // Test with same directory
            let path3 = PathBuf::from("C:\\Users\\user\\project\\main.rs");
            let path4 = PathBuf::from("C:\\Users\\user\\project\\util.rs");
            let ancestor2 = common_ancestor(&path3, &path4);
            assert_eq!(ancestor2, PathBuf::from("C:\\Users\\user\\project"));

            // Test with nested paths
            let path5 = PathBuf::from("C:\\Users\\user\\project\\src\\deep\\nested\\file.rs");
            let path6 = PathBuf::from("C:\\Users\\user\\project\\src\\main.rs");
            let ancestor3 = common_ancestor(&path5, &path6);
            assert_eq!(ancestor3, PathBuf::from("C:\\Users\\user\\project\\src"));

            // Test with completely different drives
            let path7 = PathBuf::from("C:\\Program Files\\tool");
            let path8 = PathBuf::from("D:\\Users\\user\\file");
            let ancestor4 = common_ancestor(&path7, &path8);
            // Should fall back to C:\ (root from first path)
            assert_eq!(ancestor4, PathBuf::from("C:\\"));
        }

        #[cfg(not(windows))]
        {
            let path1 = PathBuf::from("/home/user/project/src/main.rs");
            let path2 = PathBuf::from("/home/user/project/lib/util.rs");
            let ancestor = common_ancestor(&path1, &path2);
            assert_eq!(ancestor, PathBuf::from("/home/user/project"));

            // Test with same directory
            let path3 = PathBuf::from("/home/user/project/main.rs");
            let path4 = PathBuf::from("/home/user/project/util.rs");
            let ancestor2 = common_ancestor(&path3, &path4);
            assert_eq!(ancestor2, PathBuf::from("/home/user/project"));

            // Test with nested paths
            let path5 = PathBuf::from("/home/user/project/src/deep/nested/file.rs");
            let path6 = PathBuf::from("/home/user/project/src/main.rs");
            let ancestor3 = common_ancestor(&path5, &path6);
            assert_eq!(ancestor3, PathBuf::from("/home/user/project/src"));

            // Test with completely different top-level directories
            let path7 = PathBuf::from("/usr/local/bin/tool");
            let path8 = PathBuf::from("/home/user/file");
            let ancestor4 = common_ancestor(&path7, &path8);
            assert_eq!(ancestor4, PathBuf::from("/"));

            // Test with one path being an ancestor of the other
            let path9 = PathBuf::from("/home/user/project");
            let path10 = PathBuf::from("/home/user/project/src/main.rs");
            let ancestor5 = common_ancestor(&path9, &path10);
            assert_eq!(ancestor5, PathBuf::from("/home/user/project"));
        }
    }

    #[test]
    fn test_file_contains_definition() {
        // Debug: Test simple tree-sitter parsing first
        let rust_content = r#"
            pub struct MyStruct {
                field1: String,
                field2: i32,
            }
            
            pub enum MyEnum {
                Variant1,
                Variant2(String),
            }
            
            pub trait MyTrait {
                fn method(&self);
            }
        "#;

        let mut parser = tree_sitter::Parser::new();
        parser.set_language(tree_sitter_rust::language()).unwrap();
        let tree = parser.parse(rust_content, None).unwrap();

        // Test simple query without predicate first
        let simple_query = r#"
            [
              (struct_item name: (type_identifier) @name)
              (enum_item name: (type_identifier) @name)
              (trait_item name: (type_identifier) @name)
            ]
        "#;

        let query = tree_sitter::Query::new(tree_sitter_rust::language(), simple_query).unwrap();
        let mut cursor = tree_sitter::QueryCursor::new();
        let matches: Vec<_> = cursor
            .matches(&query, tree.root_node(), rust_content.as_bytes())
            .collect();

        // Should find 3 definitions: MyStruct, MyEnum, MyTrait
        assert_eq!(matches.len(), 3, "Should find exactly 3 type definitions");

        // Test the actual function with a simpler approach
        let rust_path = PathBuf::from("test.rs");
        assert!(file_contains_definition(
            &rust_path,
            rust_content,
            "MyStruct"
        ));
        assert!(file_contains_definition(&rust_path, rust_content, "MyEnum"));
        assert!(file_contains_definition(
            &rust_path,
            rust_content,
            "MyTrait"
        ));
        assert!(!file_contains_definition(
            &rust_path,
            rust_content,
            "NonExistent"
        ));

        // Test Python class definition
        let python_content = r#"
            class MyClass:
                def __init__(self):
                    pass
                    
            def my_function():
                pass
        "#;

        let python_path = PathBuf::from("test.py");
        assert!(file_contains_definition(
            &python_path,
            python_content,
            "MyClass"
        ));
        assert!(file_contains_definition(
            &python_path,
            python_content,
            "my_function"
        ));
        assert!(!file_contains_definition(
            &python_path,
            python_content,
            "NonExistent"
        ));

        // Test TypeScript interface definition
        let typescript_content = r#"
            export interface MyInterface {
                prop1: string;
                prop2: number;
            }
            
            export type MyType = string | number;
            
            export class MyClass {
                constructor() {}
            }
        "#;

        let typescript_path = PathBuf::from("test.ts");
        assert!(file_contains_definition(
            &typescript_path,
            typescript_content,
            "MyInterface"
        ));
        assert!(file_contains_definition(
            &typescript_path,
            typescript_content,
            "MyType"
        ));
        assert!(file_contains_definition(
            &typescript_path,
            typescript_content,
            "MyClass"
        ));
        assert!(!file_contains_definition(
            &typescript_path,
            typescript_content,
            "NonExistent"
        ));
    }

    #[test]
    fn test_query_engine_rust() {
        use crate::core::semantic::query_engine::QueryEngine;
        use tree_sitter::Parser;

        let rust_content = r#"
            use model::{Account, DatabaseFactory, Rule, RuleLevel, RuleName};
            
            pub fn create(
                database: &mut dyn DatabaseFactory,
                account: &Account,
                rule_name: &RuleName,
            ) -> Result<Rule, Box<dyn std::error::Error>> {
                Ok(Rule::new())
            }
        "#;

        let language = tree_sitter_rust::language();
        let query_engine = QueryEngine::new(language, "rust").unwrap();

        let mut parser = Parser::new();
        parser.set_language(tree_sitter_rust::language()).unwrap();

        let result = query_engine
            .analyze_with_parser(&mut parser, rust_content)
            .unwrap();

        println!("Imports found: {:?}", result.imports);
        println!("Type references found: {:?}", result.type_references);

        // Should find imports
        assert!(!result.imports.is_empty(), "Should find imports");

        // Should find type references from the imports
        assert!(
            !result.type_references.is_empty(),
            "Should find type references"
        );

        // Check specific types
        let type_names: Vec<&str> = result
            .type_references
            .iter()
            .map(|t| t.name.as_str())
            .collect();
        assert!(
            type_names.contains(&"DatabaseFactory"),
            "Should find DatabaseFactory type"
        );
        assert!(type_names.contains(&"Account"), "Should find Account type");
        assert!(
            type_names.contains(&"RuleName"),
            "Should find RuleName type"
        );
    }
}
