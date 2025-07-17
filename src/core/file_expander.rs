//! File expansion logic for semantic analysis features
//!
//! This module handles expanding the file list based on semantic relationships
//! discovered during analysis (imports, type references, function calls).

use crate::cli::Config;
use crate::core::cache::FileCache;
use crate::core::semantic::path_validator::validate_import_path;
use crate::core::walker::FileInfo;
use crate::utils::error::ContextCreatorError;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Expand file list based on semantic relationships
///
/// This function takes the initial set of files and expands it to include
/// files that define types, export functions, or are imported by the initial files.
pub fn expand_file_list(
    mut files_map: HashMap<PathBuf, FileInfo>,
    config: &Config,
    cache: &Arc<FileCache>,
) -> Result<HashMap<PathBuf, FileInfo>, ContextCreatorError> {
    // If no semantic features are enabled, return as-is
    if !config.trace_imports && !config.include_callers && !config.include_types {
        return Ok(files_map);
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
            work_queue.push_back((path.clone(), file_info.clone(), ExpansionReason::Imports, 0));
        }
        if config.include_callers && !file_info.function_calls.is_empty() {
            work_queue.push_back((path.clone(), file_info.clone(), ExpansionReason::Callers, 0));
        }
    }

    // Process work queue
    while let Some((source_path, source_file, reason, depth)) = work_queue.pop_front() {
        // Check if we've exceeded the semantic depth limit
        if depth >= config.semantic_depth {
            continue;
        }

        match reason {
            ExpansionReason::Types => {
                // Process type references
                for type_ref in &source_file.type_references {
                    // Skip external types
                    if type_ref.is_external {
                        continue;
                    }

                    // If we have a definition path, add it
                    if let Some(ref def_path) = type_ref.definition_path {
                        if !visited_paths.contains(def_path) && def_path.exists() {
                            // Validate the path for security
                            // Use the common ancestor as the base directory
                            let base_dir = common_ancestor(&source_path, def_path);
                            match validate_import_path(&base_dir, def_path) {
                                Ok(validated_path) => {
                                    visited_paths.insert(validated_path.clone());

                                    // Create FileInfo for the definition file
                                    let mut file_info =
                                        create_file_info_for_path(&validated_path, &source_path)?;

                                    // Queue for next depth level if it has type references
                                    if depth + 1 < config.semantic_depth
                                        && !file_info.type_references.is_empty()
                                    {
                                        // We'll need to perform semantic analysis on this file first
                                        // For now, mark it for addition
                                        file_info.imported_by.push(source_path.clone());
                                    }

                                    files_to_add.push((validated_path.clone(), file_info));
                                }
                                Err(_) => {
                                    // Path validation failed, skip this file
                                }
                            }
                        }
                    } else {
                        // Try to find the type definition file
                        if let Some(def_path) = find_type_definition_file(
                            &type_ref.name,
                            type_ref.module.as_deref(),
                            &source_path,
                            cache,
                        ) {
                            if !visited_paths.contains(&def_path) {
                                // Validate the path for security
                                // Use the common ancestor as the base directory
                                let base_dir = common_ancestor(&source_path, &def_path);
                                match validate_import_path(&base_dir, &def_path) {
                                    Ok(validated_path) => {
                                        visited_paths.insert(validated_path.clone());

                                        // Create FileInfo for the definition file
                                        let mut file_info = create_file_info_for_path(
                                            &validated_path,
                                            &source_path,
                                        )?;

                                        // Queue for next depth level if it has type references
                                        if depth + 1 < config.semantic_depth
                                            && !file_info.type_references.is_empty()
                                        {
                                            // We'll need to perform semantic analysis on this file first
                                            // For now, mark it for addition
                                            file_info.imported_by.push(source_path.clone());
                                        }

                                        files_to_add.push((validated_path, file_info));
                                    }
                                    Err(_) => {
                                        // Path validation failed, skip this file
                                    }
                                }
                            }
                        }
                    }
                }
            }
            ExpansionReason::Imports => {
                // Process imports - already handled by semantic analysis
                // The imported files should already be in the imported_by relationships
            }
            ExpansionReason::Callers => {
                // Process function calls - this would require finding files that define the called functions
                // For now, this is a placeholder for future enhancement
            }
        }
    }

    // Add new files to the map
    for (path, file_info) in files_to_add {
        files_map.insert(path, file_info);
    }

    // Update imported_by relationships for proper prioritization
    update_import_relationships(&mut files_map);

    Ok(files_map)
}

/// Reason for expanding to include a file
#[derive(Debug, Clone, Copy)]
enum ExpansionReason {
    Types,
    Imports,
    Callers,
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
    })
}

/// Find common ancestor of two paths
fn common_ancestor(path1: &Path, path2: &Path) -> PathBuf {
    let mut ancestors1 = Vec::new();
    let mut current = path1;
    while let Some(parent) = current.parent() {
        ancestors1.push(parent);
        current = parent;
    }

    let mut current = path2;
    while let Some(parent) = current.parent() {
        if ancestors1.contains(&parent) {
            return parent.to_path_buf();
        }
        current = parent;
    }

    // Fallback to root
    PathBuf::from("/")
}

/// Find a type definition file by searching nearby paths
fn find_type_definition_file(
    type_name: &str,
    module_name: Option<&str>,
    source_file: &Path,
    cache: &FileCache,
) -> Option<PathBuf> {
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

    // Search in current directory first
    for pattern in &patterns {
        let candidate = source_dir.join(pattern);
        if candidate.exists() {
            // Read the file to verify it contains the type definition
            if let Ok(content) = cache.get_or_load(&candidate) {
                // Simple heuristic: check if the file contains the type name
                // For a more robust solution, we'd use the language analyzer
                if content.contains(&format!("struct {type_name}"))
                    || content.contains(&format!("class {type_name}"))
                    || content.contains(&format!("type {type_name} "))
                    || content.contains(&format!("interface {type_name}"))
                    || content.contains(&format!("enum {type_name}"))
                    || content.contains(&format!("pub struct {type_name}"))
                    || content.contains(&format!("trait {type_name}"))
                    || content.contains(&format!("pub trait {type_name}"))
                {
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
                    if content.contains(&format!("struct {type_name}"))
                        || content.contains(&format!("class {type_name}"))
                        || content.contains(&format!("type {type_name} "))
                        || content.contains(&format!("interface {type_name}"))
                        || content.contains(&format!("enum {type_name}"))
                        || content.contains(&format!("trait {type_name}"))
                        || content.contains(&format!("pub trait {type_name}"))
                    {
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
                        if content.contains(&format!("struct {type_name}"))
                            || content.contains(&format!("class {type_name}"))
                            || content.contains(&format!("type {type_name} "))
                            || content.contains(&format!("interface {type_name}"))
                            || content.contains(&format!("enum {type_name}"))
                            || content.contains(&format!("pub struct {type_name}"))
                            || content.contains(&format!("export class {type_name}"))
                            || content.contains(&format!("export interface {type_name}"))
                            || content.contains(&format!("trait {type_name}"))
                            || content.contains(&format!("pub trait {type_name}"))
                        {
                            return Some(candidate);
                        }
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
            },
        );

        let config = Config {
            trace_imports: false,
            include_callers: false,
            include_types: false,
            ..Default::default()
        };

        let cache = Arc::new(FileCache::new());
        let result = expand_file_list(files_map.clone(), &config, &cache).unwrap();

        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_common_ancestor() {
        let path1 = PathBuf::from("/home/user/project/src/main.rs");
        let path2 = PathBuf::from("/home/user/project/lib/util.rs");
        let ancestor = common_ancestor(&path1, &path2);
        assert_eq!(ancestor, PathBuf::from("/home/user/project"));

        let path3 = PathBuf::from("/usr/local/bin/tool");
        let path4 = PathBuf::from("/home/user/file");
        let ancestor2 = common_ancestor(&path3, &path4);
        assert_eq!(ancestor2, PathBuf::from("/"));
    }
}
