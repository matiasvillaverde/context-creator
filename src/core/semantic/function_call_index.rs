//! Function call index for efficient caller lookup
//!
//! This module provides an index that maps function names to the files that call them,
//! enabling O(1) lookup for finding callers instead of O(n) file scanning.

use crate::core::walker::FileInfo;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Index that maps function names to their callers
#[derive(Debug, Default)]
pub struct FunctionCallIndex {
    /// Maps function name -> set of files that call this function
    function_to_callers: HashMap<String, HashSet<PathBuf>>,
    /// Maps function name -> call sites with module context
    function_to_call_sites: HashMap<String, Vec<FunctionCallSite>>,
    /// Maps file path -> list of functions it exports
    file_to_exports: HashMap<PathBuf, Vec<String>>,
    /// Maps file path -> resolved imports
    file_to_imports: HashMap<PathBuf, Vec<PathBuf>>,
}

#[derive(Debug, Clone)]
struct FunctionCallSite {
    file: PathBuf,
    module: Option<String>,
}

impl FunctionCallIndex {
    /// Create a new empty index
    pub fn new() -> Self {
        Self::default()
    }

    /// Build the index from a collection of analyzed files
    pub fn build(files: &[FileInfo]) -> Self {
        let mut index = Self::new();

        // First pass: collect all exported functions by file
        for file in files {
            let exported_names: Vec<String> = file
                .exported_functions
                .iter()
                .filter(|f| f.is_exported)
                .map(|f| f.name.clone())
                .collect();

            if !exported_names.is_empty() {
                index
                    .file_to_exports
                    .insert(file.path.clone(), exported_names);
            }
        }

        // Second pass: map function calls to callers
        for file in files {
            if !file.imports.is_empty() {
                index
                    .file_to_imports
                    .insert(file.path.clone(), file.imports.clone());
            }

            for func_call in &file.function_calls {
                index
                    .function_to_callers
                    .entry(func_call.name.clone())
                    .or_default()
                    .insert(file.path.clone());
                index
                    .function_to_call_sites
                    .entry(func_call.name.clone())
                    .or_default()
                    .push(FunctionCallSite {
                        file: file.path.clone(),
                        module: func_call.module.clone(),
                    });
            }
        }

        index
    }

    /// Get all files that call the given function
    pub fn get_callers(&self, function_name: &str) -> Option<&HashSet<PathBuf>> {
        self.function_to_callers.get(function_name)
    }

    /// Get all functions exported by the given file
    pub fn get_exports(&self, file_path: &PathBuf) -> Option<&Vec<String>> {
        self.file_to_exports.get(file_path)
    }

    /// Find all files that call any function exported by the given files
    pub fn find_callers_of_files(&self, target_files: &[PathBuf]) -> HashSet<PathBuf> {
        let mut callers = HashSet::new();

        for target_path in target_files {
            let exported_functions = self.exports_for_target(target_path);

            // Find all files that call any of these functions
            for func_name in exported_functions {
                if let Some(call_sites) = self.function_to_call_sites.get(&func_name) {
                    for call_site in call_sites {
                        if !self.call_site_matches_target(call_site, target_path) {
                            continue;
                        }

                        let caller_path = &call_site.file;
                        // Don't include the target files themselves
                        // Check both exact match and filename match
                        let is_target = target_files.iter().any(|target| {
                            caller_path == target
                                || (caller_path.file_name() == target.file_name()
                                    && caller_path.file_name().is_some())
                        });

                        if !is_target {
                            callers.insert(caller_path.clone());
                        }
                    }
                } else if let Some(caller_files) = self.get_callers(&func_name) {
                    // Backward-compatible fallback for indexes built without call sites.
                    for caller_path in caller_files {
                        // Don't include the target files themselves
                        // Check both exact match and filename match
                        let is_target = target_files.iter().any(|target| {
                            caller_path == target
                                || (caller_path.file_name() == target.file_name()
                                    && caller_path.file_name().is_some())
                        });

                        if !is_target {
                            callers.insert(caller_path.clone());
                        }
                    }
                }
            }
        }

        callers
    }

    fn exports_for_target(&self, target_path: &PathBuf) -> Vec<String> {
        // First try exact match.
        if let Some(exports) = self.get_exports(target_path) {
            return exports.clone();
        }

        // If no exact match, look for paths that end with the same filename.
        // This handles cases where paths might be absolute vs relative.
        let Some(target_filename) = target_path.file_name() else {
            return Vec::new();
        };

        self.file_to_exports
            .iter()
            .filter(|(indexed_path, _)| indexed_path.file_name() == Some(target_filename))
            .flat_map(|(_, exports)| exports.clone())
            .collect()
    }

    fn call_site_matches_target(&self, call_site: &FunctionCallSite, target_path: &Path) -> bool {
        match target_path.extension().and_then(|ext| ext.to_str()) {
            Some("go") => {}
            Some("swift") => return self.swift_call_site_matches_target(call_site, target_path),
            _ => return true,
        }

        if same_directory(&call_site.file, target_path) {
            return true;
        }

        if let Some(module) = call_site.module.as_deref() {
            if go_module_matches_target(module, target_path) {
                return true;
            }
        }

        self.file_imports_go_target_package(&call_site.file, target_path)
    }

    fn swift_call_site_matches_target(
        &self,
        call_site: &FunctionCallSite,
        target_path: &Path,
    ) -> bool {
        if same_swift_target(&call_site.file, target_path) {
            return true;
        }

        if let Some(module) = call_site.module.as_deref() {
            if swift_module_matches_target(module, target_path) {
                return true;
            }
        }

        self.file_imports_swift_target_module(&call_site.file, target_path)
    }

    fn file_imports_go_target_package(&self, caller_path: &Path, target_path: &Path) -> bool {
        self.file_to_imports
            .get(caller_path)
            .is_some_and(|imports| {
                imports.iter().any(|import_path| {
                    paths_equivalent(import_path, target_path)
                        || (import_path.extension().and_then(|ext| ext.to_str()) == Some("go")
                            && same_directory(import_path, target_path))
                })
            })
    }

    fn file_imports_swift_target_module(&self, caller_path: &Path, target_path: &Path) -> bool {
        self.file_to_imports
            .get(caller_path)
            .is_some_and(|imports| {
                imports.iter().any(|import_path| {
                    paths_equivalent(import_path, target_path)
                        || (import_path.extension().and_then(|ext| ext.to_str()) == Some("swift")
                            && same_swift_target(import_path, target_path))
                })
            })
    }

    /// Get statistics about the index
    pub fn stats(&self) -> IndexStats {
        IndexStats {
            total_functions: self.function_to_callers.len(),
            total_files_with_exports: self.file_to_exports.len(),
            total_caller_relationships: self
                .function_to_callers
                .values()
                .map(|callers| callers.len())
                .sum(),
        }
    }
}

fn same_directory(left: &Path, right: &Path) -> bool {
    match (left.parent(), right.parent()) {
        (Some(left_parent), Some(right_parent)) => paths_equivalent(left_parent, right_parent),
        _ => false,
    }
}

fn paths_equivalent(left: &std::path::Path, right: &std::path::Path) -> bool {
    if left == right {
        return true;
    }

    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

fn go_module_matches_target(module: &str, target_path: &Path) -> bool {
    let Some(package_name) = target_path
        .parent()
        .and_then(|parent| parent.file_name())
        .and_then(|name| name.to_str())
    else {
        return false;
    };

    module == package_name || module.rsplit('/').next() == Some(package_name)
}

fn same_swift_target(left: &Path, right: &Path) -> bool {
    match (swift_target_dir(left), swift_target_dir(right)) {
        (Some(left_target), Some(right_target)) => paths_equivalent(&left_target, &right_target),
        _ => same_directory(left, right),
    }
}

fn swift_target_dir(path: &Path) -> Option<PathBuf> {
    let components: Vec<_> = path.components().collect();

    for index in 0..components.len().saturating_sub(1) {
        let std::path::Component::Normal(segment) = components[index] else {
            continue;
        };

        let segment = segment.to_string_lossy();
        if segment != "Sources" && segment != "Tests" {
            continue;
        }

        if !matches!(components[index + 1], std::path::Component::Normal(_)) {
            return None;
        }

        let mut target = PathBuf::new();
        for component in &components[..=index + 1] {
            target.push(component.as_os_str());
        }
        return Some(target);
    }

    None
}

fn swift_module_matches_target(module: &str, target_path: &Path) -> bool {
    let Some(target_name) = swift_target_dir(target_path)
        .as_deref()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
        .or_else(|| {
            target_path
                .parent()
                .and_then(Path::file_name)
                .and_then(|name| name.to_str())
                .map(ToOwned::to_owned)
        })
    else {
        return false;
    };

    module == target_name || module.split('.').next() == Some(target_name.as_str())
}

/// Statistics about the function call index
#[derive(Debug)]
pub struct IndexStats {
    pub total_functions: usize,
    pub total_files_with_exports: usize,
    pub total_caller_relationships: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::semantic::analyzer::{FunctionCall, FunctionDefinition};

    fn create_test_file(path: &str, exports: Vec<(&str, bool)>, calls: Vec<&str>) -> FileInfo {
        FileInfo {
            path: PathBuf::from(path),
            relative_path: PathBuf::from(path),
            size: 0,
            file_type: crate::utils::file_ext::FileType::Rust,
            priority: 1.0,
            imports: vec![],
            imported_by: vec![],
            function_calls: calls
                .into_iter()
                .map(|name| FunctionCall {
                    name: name.to_string(),
                    module: None,
                    line: 1,
                })
                .collect(),
            type_references: vec![],
            exported_functions: exports
                .into_iter()
                .map(|(name, is_exported)| FunctionDefinition {
                    name: name.to_string(),
                    is_exported,
                    line: 1,
                })
                .collect(),
        }
    }

    #[test]
    fn test_index_building() {
        let files = vec![
            create_test_file("lib.rs", vec![("foo", true), ("bar", true)], vec![]),
            create_test_file("main.rs", vec![("main", false)], vec!["foo", "println"]),
            create_test_file("test.rs", vec![], vec!["foo", "bar", "assert_eq"]),
        ];

        let index = FunctionCallIndex::build(&files);

        // Check exports
        assert_eq!(
            index.get_exports(&PathBuf::from("lib.rs")).unwrap().len(),
            2
        );
        assert!(index.get_exports(&PathBuf::from("main.rs")).is_none());

        // Check callers
        let foo_callers = index.get_callers("foo").unwrap();
        assert_eq!(foo_callers.len(), 2);
        assert!(foo_callers.contains(&PathBuf::from("main.rs")));
        assert!(foo_callers.contains(&PathBuf::from("test.rs")));

        let bar_callers = index.get_callers("bar").unwrap();
        assert_eq!(bar_callers.len(), 1);
        assert!(bar_callers.contains(&PathBuf::from("test.rs")));
    }

    #[test]
    fn test_find_callers_of_files() {
        let files = vec![
            create_test_file("utils.rs", vec![("helper", true)], vec![]),
            create_test_file("app.rs", vec![], vec!["helper"]),
            create_test_file("tests.rs", vec![], vec!["helper"]),
            create_test_file("other.rs", vec![], vec!["unrelated"]),
        ];

        let index = FunctionCallIndex::build(&files);
        let callers = index.find_callers_of_files(&[PathBuf::from("utils.rs")]);

        assert_eq!(callers.len(), 2);
        assert!(callers.contains(&PathBuf::from("app.rs")));
        assert!(callers.contains(&PathBuf::from("tests.rs")));
        assert!(!callers.contains(&PathBuf::from("other.rs")));
    }
}
