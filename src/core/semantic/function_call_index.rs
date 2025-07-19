//! Function call index for efficient caller lookup
//!
//! This module provides an index that maps function names to the files that call them,
//! enabling O(1) lookup for finding callers instead of O(n) file scanning.

use crate::core::walker::FileInfo;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Index that maps function names to their callers
#[derive(Debug, Default)]
pub struct FunctionCallIndex {
    /// Maps function name -> set of files that call this function
    function_to_callers: HashMap<String, HashSet<PathBuf>>,
    /// Maps file path -> list of functions it exports
    file_to_exports: HashMap<PathBuf, Vec<String>>,
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
            for func_call in &file.function_calls {
                index
                    .function_to_callers
                    .entry(func_call.name.clone())
                    .or_default()
                    .insert(file.path.clone());
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

        // Collect all function names exported by target files
        let mut exported_functions = HashSet::new();

        // Try both the exact path and checking if any indexed path ends with the target
        for target_path in target_files {
            // First try exact match
            if let Some(exports) = self.get_exports(target_path) {
                for func_name in exports {
                    exported_functions.insert(func_name);
                }
            } else {
                // If no exact match, look for paths that end with the same filename
                // This handles cases where paths might be absolute vs relative
                if let Some(target_filename) = target_path.file_name() {
                    for (indexed_path, exports) in &self.file_to_exports {
                        if indexed_path.file_name() == Some(target_filename) {
                            for func_name in exports {
                                exported_functions.insert(func_name);
                            }
                        }
                    }
                }
            }
        }

        // Find all files that call any of these functions
        for func_name in exported_functions {
            if let Some(caller_files) = self.get_callers(func_name) {
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

        callers
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
